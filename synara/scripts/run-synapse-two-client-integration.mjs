import { randomBytes } from 'node:crypto';
import { pathToFileURL } from 'node:url';

const DEFAULT_HOMESERVER = 'http://127.0.0.1:8008';
const EVENT_COUNT = 64;
const INITIAL_SYNC_LIMIT = 8;
const PAGE_SIZE = 20;
const POLL_TIMEOUT_MS = 20_000;
const REQUEST_TIMEOUT_MS = 15_000;

export class SafeIntegrationError extends Error {}

export function validateLocalHomeserverUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new SafeIntegrationError('SYNARA_SYNAPSE_URL must be a valid URL.');
  }

  const loopbackHosts = new Set(['127.0.0.1', 'localhost', '[::1]']);
  if (
    url.protocol !== 'http:' ||
    !loopbackHosts.has(url.hostname) ||
    url.username ||
    url.password ||
    (url.pathname !== '/' && url.pathname !== '') ||
    url.search ||
    url.hash
  ) {
    throw new SafeIntegrationError(
      'SYNARA_SYNAPSE_URL must be an HTTP loopback origin with no credentials, path, query, or fragment.'
    );
  }

  return url.origin;
}

export function parseReceiptModes(value = 'both') {
  if (value === 'public') return ['public'];
  if (value === 'private') return ['private'];
  if (value === 'both') return ['public', 'private'];
  throw new SafeIntegrationError('SYNARA_RECEIPT_MODE must be public, private, or both.');
}

export async function pollUntil(
  description,
  predicate,
  { timeoutMs = POLL_TIMEOUT_MS, intervalMs = 100 } = {}
) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const value = await predicate();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  throw new SafeIntegrationError(`Timed out waiting for ${description}.`);
}

function requireValue(value, message) {
  if (!value) throw new SafeIntegrationError(message);
  return value;
}

function assertCondition(condition, message) {
  if (!condition) throw new SafeIntegrationError(message);
}

function randomLocalpart(prefix) {
  return `${prefix}_${randomBytes(9).toString('hex')}`;
}

function randomPassword() {
  return `S-${randomBytes(32).toString('base64url')}`;
}

function silentLogger() {
  const logger = {
    trace() {},
    debug() {},
    info() {},
    warn() {},
    error() {},
    getChild() {
      return logger;
    },
  };
  return logger;
}

function clientOptions(baseUrl, credentials = {}) {
  return {
    baseUrl,
    localTimeoutMs: REQUEST_TIMEOUT_MS,
    logger: silentLogger(),
    disableVoip: true,
    ...credentials,
  };
}

function safeFailureDetail(error) {
  const errorCode =
    typeof error?.errcode === 'string' && /^[A-Z0-9_.-]+$/.test(error.errcode)
      ? error.errcode
      : undefined;
  const status = Number.isInteger(error?.httpStatus) ? error.httpStatus : undefined;
  if (!errorCode && !status) return '';
  return ` (${[errorCode, status && `HTTP ${status}`].filter(Boolean).join(', ')})`;
}

async function phase(name, operation) {
  try {
    return await operation();
  } catch (error) {
    if (error instanceof SafeIntegrationError) throw error;
    throw new SafeIntegrationError(`${name} failed${safeFailureDetail(error)}.`);
  }
}

export function selectLocalRegistrationAuth(errorData) {
  const session = typeof errorData?.session === 'string' ? errorData.session : undefined;
  const flows = Array.isArray(errorData?.flows) ? errorData.flows : [];
  const stages = flows
    .map((flow) => (Array.isArray(flow?.stages) ? flow.stages : undefined))
    .filter(Boolean);

  if (stages.some((flowStages) => flowStages.length === 0)) {
    return session ? { session } : {};
  }
  if (stages.some((flowStages) => flowStages.length === 1 && flowStages[0] === 'm.login.dummy')) {
    return { type: 'm.login.dummy', ...(session ? { session } : {}) };
  }
  throw new SafeIntegrationError('Registration requires an unsupported local UIAA flow.');
}

async function registerAccount(sdk, baseUrl, localpart, password, deviceName) {
  const registrationClient = sdk.createClient(clientOptions(baseUrl));
  const request = {
    username: localpart,
    password,
    initial_device_display_name: deviceName,
  };

  let auth;
  try {
    return await registrationClient.registerRequest(request);
  } catch (error) {
    if (error?.errcode !== 'M_UNAUTHORIZED') throw error;
    auth = selectLocalRegistrationAuth(error?.data);
  }

  return registrationClient.registerRequest({
    ...request,
    auth,
  });
}

function authenticatedClient(sdk, baseUrl, response) {
  return sdk.createClient(
    clientOptions(baseUrl, {
      userId: requireValue(response.user_id, 'Authentication omitted user_id.'),
      deviceId: requireValue(response.device_id, 'Authentication omitted device_id.'),
      accessToken: requireValue(response.access_token, 'Authentication omitted access_token.'),
    })
  );
}

async function startAndWait(client, syncLimit, sdk) {
  await client.startClient({
    initialSyncLimit: syncLimit,
    pollTimeout: 1_000,
    disablePresence: true,
    lazyLoadMembers: true,
  });
  await pollUntil('an initial client sync', () => {
    const state = client.getSyncState();
    if (state === sdk.SyncState.Error) {
      throw new SafeIntegrationError('Client entered ERROR during initial sync.');
    }
    return state === sdk.SyncState.Prepared || state === sdk.SyncState.Syncing;
  });
}

function messageEvents(timeline, sdk) {
  return timeline.getEvents().filter((event) => event.getType() === sdk.EventType.RoomMessage);
}

function assertSentOrder(events, sentIndex, description) {
  const indexes = events
    .map((event) => sentIndex.get(event.getId()))
    .filter((index) => index !== undefined);
  assertCondition(indexes.length > 0, `${description} contained no sent events.`);
  for (let index = 1; index < indexes.length; index += 1) {
    assertCondition(
      indexes[index] > indexes[index - 1],
      `${description} did not preserve event order.`
    );
  }
}

function receiptWasObserved(event, eventId, receiptType, userId) {
  return Boolean(event.getContent()?.[eventId]?.[receiptType]?.[userId]);
}

async function runReceiptScenario(sdk, baseUrl, receiptMode) {
  const clients = [];
  try {
    console.log(`[synapse-integration] ${receiptMode}: creating local sessions.`);
    const userLocalpart = randomLocalpart('synara_reader');
    const userPassword = randomPassword();
    const senderLocalpart = randomLocalpart('synara_sender');
    const senderPassword = randomPassword();

    const deviceARegistration = await phase('Reader registration', () =>
      registerAccount(sdk, baseUrl, userLocalpart, userPassword, 'Synara integration device A')
    );
    const loginClient = sdk.createClient(clientOptions(baseUrl));
    const deviceBLogin = await phase('Second-device login', () =>
      loginClient.loginRequest({
        type: 'm.login.password',
        identifier: { type: 'm.id.user', user: userLocalpart },
        password: userPassword,
        initial_device_display_name: 'Synara integration device B',
      })
    );
    const senderRegistration = await phase('Sender registration', () =>
      registerAccount(sdk, baseUrl, senderLocalpart, senderPassword, 'Synara integration sender')
    );

    const deviceA = authenticatedClient(sdk, baseUrl, deviceARegistration);
    const deviceB = authenticatedClient(sdk, baseUrl, deviceBLogin);
    const sender = authenticatedClient(sdk, baseUrl, senderRegistration);
    clients.push(deviceA, deviceB, sender);

    await phase('Device A initial sync', () => startAndWait(deviceA, EVENT_COUNT + 16, sdk));

    console.log(`[synapse-integration] ${receiptMode}: creating timeline fixture.`);
    const roomId = await phase('Room creation', async () => {
      const result = await sender.createRoom({
        visibility: sdk.Visibility.Private,
        preset: sdk.Preset.PrivateChat,
        invite: [deviceARegistration.user_id],
      });
      return requireValue(result.room_id, 'Room creation omitted room_id.');
    });
    await phase('Reader room join', () => deviceA.joinRoom(roomId));
    await pollUntil('device A joined-room state', () => {
      const room = deviceA.getRoom(roomId);
      return room?.getMyMembership() === 'join' ? room : undefined;
    });

    const sentIds = [];
    for (let index = 0; index < EVENT_COUNT; index += 1) {
      const response = await phase(`Ordered event ${index + 1}`, () =>
        sender.sendEvent(roomId, sdk.EventType.RoomMessage, {
          msgtype: sdk.MsgType.Text,
          body: `synara-integration-${receiptMode}-${index}`,
        })
      );
      sentIds.push(requireValue(response.event_id, 'A sent event omitted event_id.'));
    }
    const sentIndex = new Map(sentIds.map((eventId, index) => [eventId, index]));
    const lastEventId = sentIds.at(-1);
    const markerEventId = sentIds[Math.floor(EVENT_COUNT / 2)];

    await pollUntil('device A receiving the marker event', () =>
      deviceA.getRoom(roomId)?.findEventById(markerEventId)
    );

    console.log(
      `[synapse-integration] ${receiptMode}: validating limited sync, context, and pagination.`
    );
    await phase('Device B initial sync', () => startAndWait(deviceB, INITIAL_SYNC_LIMIT, sdk));
    const deviceBRoom = await pollUntil('device B joined-room state', () => {
      const room = deviceB.getRoom(roomId);
      return room?.getMyMembership() === 'join' ? room : undefined;
    });

    const initialMessages = messageEvents(deviceBRoom.getLiveTimeline(), sdk);
    assertSentOrder(initialMessages, sentIndex, 'Limited initial timeline');
    assertCondition(
      initialMessages.at(-1)?.getId() === lastEventId,
      'Limited initial timeline did not end at the actual last event.'
    );
    assertCondition(
      !initialMessages.some((event) => event.getId() === sentIds[0]),
      'Initial sync was not limited, so pagination coverage is invalid.'
    );

    const timelineSet = deviceBRoom.getUnfilteredTimelineSet();
    const contextTimeline = await phase('Context timeline load', () =>
      deviceB.getEventTimeline(timelineSet, markerEventId)
    );
    requireValue(contextTimeline, 'Context endpoint did not return a timeline.');
    assertCondition(
      contextTimeline.getEvents().some((event) => event.getId() === markerEventId),
      'Context timeline omitted the requested event.'
    );
    const beforePagination = messageEvents(contextTimeline, sdk);
    assertSentOrder(beforePagination, sentIndex, 'Context timeline');
    const earliestBefore = sentIndex.get(beforePagination[0]?.getId());
    assertCondition(
      Number.isInteger(earliestBefore) && earliestBefore > 0,
      'Context timeline cannot exercise backwards pagination.'
    );

    await phase('Backwards timeline pagination', () =>
      deviceB.paginateEventTimeline(contextTimeline, {
        backwards: true,
        limit: PAGE_SIZE,
      })
    );
    const afterPagination = messageEvents(contextTimeline, sdk);
    assertSentOrder(afterPagination, sentIndex, 'Paginated context timeline');
    const earliestAfter = sentIndex.get(afterPagination[0]?.getId());
    assertCondition(
      afterPagination.length > beforePagination.length &&
        Number.isInteger(earliestAfter) &&
        earliestAfter < earliestBefore,
      'Backwards pagination did not add an earlier page.'
    );

    const latestTimeline = await phase('Latest timeline load', () =>
      deviceB.getLatestTimeline(timelineSet)
    );
    requireValue(latestTimeline, 'Latest endpoint did not return a timeline.');
    const latestMessages = messageEvents(latestTimeline, sdk);
    assertSentOrder(latestMessages, sentIndex, 'Latest SDK timeline');
    assertCondition(
      latestMessages.at(-1)?.getId() === lastEventId,
      'Latest SDK timeline did not end at the actual last event.'
    );

    console.log(`[synapse-integration] ${receiptMode}: validating cross-device read state.`);
    const receiptType =
      receiptMode === 'private' ? sdk.ReceiptType.ReadPrivate : sdk.ReceiptType.Read;
    let observedReceipt = false;
    const receiptListener = (event, room) => {
      if (room.roomId !== roomId) return;
      if (receiptWasObserved(event, markerEventId, receiptType, deviceARegistration.user_id)) {
        observedReceipt = true;
      }
    };
    deviceB.on(sdk.RoomEvent.Receipt, receiptListener);

    const markerEvent = requireValue(
      deviceA.getRoom(roomId)?.findEventById(markerEventId),
      'Device A lost the read-marker event.'
    );
    await phase('Read-marker update', () =>
      deviceA.setRoomReadMarkers(
        roomId,
        markerEventId,
        receiptMode === 'public' ? markerEvent : undefined,
        receiptMode === 'private' ? markerEvent : undefined
      )
    );

    await pollUntil('device B read-state convergence', () => {
      const fullyRead = deviceB
        .getRoom(roomId)
        ?.getAccountData(sdk.EventType.FullyRead)
        ?.getContent()?.event_id;
      return fullyRead === markerEventId && observedReceipt;
    });
    deviceB.off(sdk.RoomEvent.Receipt, receiptListener);

    console.log(`[synapse-integration] ${receiptMode}: passed ${EVENT_COUNT} ordered events.`);
  } finally {
    for (const client of clients) client.stopClient();
  }
}

export async function runIntegration({
  homeserver = process.env.SYNARA_SYNAPSE_URL ?? DEFAULT_HOMESERVER,
  receiptMode = process.env.SYNARA_RECEIPT_MODE ?? 'both',
} = {}) {
  const baseUrl = validateLocalHomeserverUrl(homeserver);
  const receiptModes = parseReceiptModes(receiptMode);
  const sdk = await phase('Matrix SDK load', () => import('matrix-js-sdk'));

  for (const mode of receiptModes) {
    await runReceiptScenario(sdk, baseUrl, mode);
  }
  console.log('[synapse-integration] all two-client scenarios passed.');
}

async function main() {
  try {
    await runIntegration();
  } catch (error) {
    const message =
      error instanceof SafeIntegrationError
        ? error.message
        : 'Integration runner failed without a safe diagnostic.';
    console.error(`[synapse-integration] ${message}`);
    process.exitCode = 1;
  }
}

if (
  process.env.SYNARA_RUN_SYNAPSE_INTEGRATION === '1' ||
  (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href)
) {
  await main();
}
