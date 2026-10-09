import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  NOTIFICATION_OBSERVED_EVENT,
  parseNativeNotificationObservation,
  subscribeNativeNotificationObservations,
  type NativeNotificationObservation,
} from '../nativeNotificationObservation';

import {
  buildNativeObservedNotificationPresentation,
  createObservedBrowserNotificationRegistry,
  isNativeNotificationActionForSession,
  deliverNativeObservedNotificationCandidate,
} from '../nativeNotificationPresentation';
import type { NativeNotificationCandidate } from '../nativeNotificationDecision';

const OBSERVATION: NativeNotificationObservation = {
  sessionGeneration: 7,
  roomId: '!room:example.org',
  eventId: '$event:example.org',
  sender: '@bob:example.org',
  eventType: 'm.room.message',
  originServerTs: 1_700_000_000_000,
  agentApproval: {
    expiresAt: 1_700_000_300_000,
    expired: false,
    summary: { reason: 'find -delete', command: 'find /tmp/x -delete' },
  },
};

const APPROVAL = OBSERVATION.agentApproval!;

test('observation parser accepts identity plus bounded facts and rejects verdicts', () => {
  assert.deepEqual(parseNativeNotificationObservation(OBSERVATION), OBSERVATION);
  // Approval classification is optional (ordinary/undecryptable events).
  const bare: NativeNotificationObservation = { ...OBSERVATION };
  delete bare.agentApproval;
  assert.deepEqual(parseNativeNotificationObservation({ ...bare, agentApproval: null }), bare);
  assert.deepEqual(parseNativeNotificationObservation(bare), bare);
  // Core sends absent summary fields as null.
  assert.deepEqual(
    parseNativeNotificationObservation({
      ...OBSERVATION,
      agentApproval: { ...APPROVAL, summary: { reason: null, command: 'ls' } },
    }),
    { ...OBSERVATION, agentApproval: { ...APPROVAL, summary: { command: 'ls' } } }
  );

  // A policy verdict or any unknown key fails closed so the wire cannot grow
  // one silently; identity must be well-formed; the type vocabulary is closed.
  for (const bad of [
    { ...OBSERVATION, highlight: true },
    { ...OBSERVATION, sound: true },
    { ...OBSERVATION, notify: true },
    { ...OBSERVATION, mode: 'all_messages' },
    { ...OBSERVATION, roomId: 'room' },
    { ...OBSERVATION, eventId: 'event' },
    { ...OBSERVATION, sender: 'bob' },
    { ...OBSERVATION, eventType: 'm.reaction' },
    { ...OBSERVATION, sessionGeneration: -1 },
    { ...OBSERVATION, originServerTs: 'now' },
    { ...OBSERVATION, agentApproval: 42 },
    { ...OBSERVATION, body: 'retired plaintext parser input' },
    { ...OBSERVATION, agentApproval: { ...APPROVAL, body: 'raw prompt' } },
    { ...OBSERVATION, agentApproval: { ...APPROVAL, expiresAt: -1 } },
    { ...OBSERVATION, agentApproval: { expiresAt: 123, expired: false } },
    { ...OBSERVATION, agentApproval: { ...APPROVAL, summary: { reason: 'r', body: 'raw' } } },
    { ...OBSERVATION, agentApproval: { ...APPROVAL, summary: { command: 42 } } },
    { ...OBSERVATION, agentApproval: { ...APPROVAL, summary: { command: 'x'.repeat(513) } } },
    null,
    [],
    'observation',
  ]) {
    assert.equal(parseNativeNotificationObservation(bad), undefined);
  }
});

test('subscription delivers only well-formed observations for the current generation', async () => {
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  let unlistened = 0;
  const session: { generation: number | undefined } = { generation: undefined };
  const received: NativeNotificationObservation[] = [];

  const dispose = subscribeNativeNotificationObservations(
    () => session.generation,
    (observation) => received.push(observation),
    {
      desktopAvailable: true,
      listen: async (event, handler) => {
        handlers.set(event, handler as (event: { payload: unknown }) => void);
        return () => {
          unlistened += 1;
        };
      },
    }
  );
  await Promise.resolve();
  const emit = handlers.get(NOTIFICATION_OBSERVED_EVENT);
  assert.ok(emit, 'subscribes to the observation event');

  // Before the renderer knows its generation nothing is decided.
  emit({ payload: OBSERVATION });
  assert.deepEqual(received, []);

  session.generation = 7;
  emit({ payload: OBSERVATION });
  emit({ payload: { ...OBSERVATION, sessionGeneration: 6 } });
  emit({ payload: { ...OBSERVATION, highlight: true } });
  assert.deepEqual(received, [OBSERVATION]);

  dispose();
  emit({ payload: OBSERVATION });
  assert.deepEqual(received, [OBSERVATION]);
  assert.equal(unlistened, 1);
});

test('subscription is inert off desktop', async () => {
  let listened = 0;
  const dispose = subscribeNativeNotificationObservations(
    () => 7,
    () => assert.fail('no observation without a native shell'),
    {
      desktopAvailable: false,
      listen: async () => {
        listened += 1;
        return () => undefined;
      },
    }
  );
  await Promise.resolve();
  dispose();
  assert.equal(listened, 0);
});

test('the renderer observes through the Core stream and no longer scans timelines', () => {
  const root = process.cwd();
  const source = readFileSync(`${root}/src/app/pages/client/ClientNonUIFeatures.tsx`, 'utf8');

  // Both the message and the agent-approval pumps subscribe to the stream.
  const subscriptions = source.match(/subscribeNativeNotificationObservations\(/g) ?? [];
  assert.ok(subscriptions.length >= 2, 'messages and approvals subscribe');
  assert.doesNotMatch(source, /encryptedFallbackRef/);
  assert.match(source, /kind: 'agent_approval'/);

  // The dead observation pumps are gone: no sync-state gate the native facade
  // never satisfies, no `Room.timeline` listener the facade never emits, no
  // periodic scan of live timelines the facade stubs to `[]`.
  assert.doesNotMatch(source, /getSyncState\(\) !== 'SYNCING'/);
  assert.doesNotMatch(source, /'Room\.timeline'/);
  assert.doesNotMatch(source, /getLoadedLiveTimelineEvents/);
  assert.doesNotMatch(source, /setInterval\(scanRecent/);
  assert.doesNotMatch(source, /RECENT_MESSAGE_NOTIFICATION_MS/);
});

test('one bounded ciphertext observation reaches the subscription consumer without a handshake', async () => {
  let handler: ((event: { payload: unknown }) => void) | undefined;
  const received: NativeNotificationObservation[] = [];
  const dispose = subscribeNativeNotificationObservations(
    () => 7,
    (observation) => received.push(observation),
    {
      desktopAvailable: true,
      listen: async (_event, callback) => {
        handler = callback as (event: { payload: unknown }) => void;
        return () => undefined;
      },
    }
  );
  await Promise.resolve();
  const ciphertext: NativeNotificationObservation = {
    ...OBSERVATION,
    eventType: 'm.room.encrypted',
  };
  delete ciphertext.agentApproval;
  handler?.({ payload: ciphertext });
  assert.deepEqual(received, [ciphertext]);
  dispose();

  // The mounted message route submits that single observation to Core. It
  // cannot reinstate the retired two-emission handshake or ciphertext skip.
  const source = readFileSync(
    `${process.cwd()}/src/app/pages/client/ClientNonUIFeatures.tsx`,
    'utf8'
  );
  const start = source.indexOf('const decideAndNotify = useCallback(');
  assert.ok(start >= 0);
  const end = source.indexOf('\n  useEffect(', start);
  assert.ok(end > start);
  const route = source.slice(start, end);
  assert.match(route, /decideNotificationWithNativeOwner\(/);
  assert.doesNotMatch(route, /encryptedFallbackRef|eventType\s*===\s*'m\.room\.encrypted'/);
});

test('late approval promotion uses the shared actionable presentation and exact source IDs', () => {
  const candidate: NativeNotificationCandidate = {
    candidateId: 'candidate',
    roomId: OBSERVATION.roomId,
    eventId: OBSERVATION.eventId,
    kind: 'agent_approval',
    title: 'Approval Required: Dangerous Command',
    body: 'Review a request in Synara.',
    suppressIfFocusedRoom: false,
    isEncrypted: true,
  };
  const source = { roomId: OBSERVATION.roomId, eventId: OBSERVATION.eventId, sessionGeneration: 7 };
  const presentation = buildNativeObservedNotificationPresentation(candidate, source);
  assert.equal(presentation.title, candidate.title);
  assert.equal(presentation.body, candidate.body);
  assert.deepEqual(presentation.dismissKeys, [
    `room:${source.roomId}`,
    `event:${source.eventId}`,
    `candidate:${candidate.candidateId}`,
  ]);
  assert.equal(presentation.sessionGeneration, 7);
  assert.equal(presentation.sound, 'default');
  assert.deepEqual(presentation.actionContext, {
    kind: 'agent-approval',
    roomId: source.roomId,
    eventId: source.eventId,
  });
  assert.deepEqual(
    presentation.actions?.map((action) => action.id),
    ['agent-approval.approve-once', 'agent-approval.deny', 'agent-approval.review']
  );
  assert.ok(presentation.route?.includes(encodeURIComponent(source.eventId)));
  for (const mismatched of [
    { ...candidate, roomId: '!other:example.org' },
    { ...candidate, eventId: '$other' },
    { ...candidate, eventId: undefined },
    { ...candidate, kind: 'later_reminder' as const },
  ])
    assert.throws(() => buildNativeObservedNotificationPresentation(mismatched, source));

  const ordinary = buildNativeObservedNotificationPresentation(
    { ...candidate, kind: 'message' },
    source
  );
  assert.equal(ordinary.actions, undefined);
  assert.equal(ordinary.actionContext, undefined);
  assert.deepEqual(ordinary.dismissKeys, presentation.dismissKeys);
  assert.equal(ordinary.sound, 'silent');

  // Both mounted delivery paths use this tested adapter. A late promotion
  // must also get approval sound ownership instead of generic push tweaks.
  const mounted = readFileSync(
    `${process.cwd()}/src/app/pages/client/ClientNonUIFeatures.tsx`,
    'utf8'
  );
  assert.equal(
    (mounted.match(/buildNativeObservedNotificationPresentation\(candidate,/g) ?? []).length,
    2
  );
  assert.match(mounted, /candidate\.kind === 'agent_approval'/);
  assert.doesNotMatch(mounted, /approvalEventId/);
});

test('shown candidate delivery always releases pending ownership after generation or delivery faults', async () => {
  const candidate: NativeNotificationCandidate = {
    candidateId: 'candidate-7',
    roomId: OBSERVATION.roomId,
    eventId: OBSERVATION.eventId,
    kind: 'agent_approval',
    title: 'Approval Required: Dangerous Command',
    body: 'Review a request in Synara.',
    suppressIfFocusedRoom: false,
    isEncrypted: true,
  };
  for (const generation of [undefined, 6, 8]) {
    const pending = new Set([candidate.candidateId]);
    let deliveries = 0;
    await deliverNativeObservedNotificationCandidate({
      candidate,
      observedGeneration: 7,
      currentGeneration: () => generation,
      deliver: async () => {
        deliveries += 1;
        return 'delivered';
      },
      acknowledge: async (id, outcome) => {
        assert.equal(outcome, undefined);
        pending.delete(id);
      },
    });
    assert.equal(deliveries, 0, 'no stale or unknown account reaches platform delivery');
    assert.equal(pending.size, 0, 'stale readback still releases its pending candidate');
  }
  let receipt: string | undefined;
  await deliverNativeObservedNotificationCandidate({
    candidate,
    observedGeneration: 7,
    currentGeneration: () => 7,
    deliver: async () => {
      throw new Error('platform delivery failed');
    },
    acknowledge: async (_id, outcome) => {
      receipt = outcome;
    },
  });
  assert.equal(receipt, 'failed');
  let deliveries = 0;
  await deliverNativeObservedNotificationCandidate({
    candidate,
    observedGeneration: 7,
    currentGeneration: () => 7,
    deliver: async () => {
      deliveries += 1;
      return 'delivered';
    },
    acknowledge: async () => {
      throw new Error('owner already retired');
    },
  });
  assert.equal(deliveries, 1, 'acknowledgement failure never retries platform delivery');
});

test('late authoritative approval reaches delivery during inbox triage with no renderer room metadata', async () => {
  const source = { roomId: OBSERVATION.roomId, eventId: OBSERVATION.eventId, sessionGeneration: 7 };
  const candidate: NativeNotificationCandidate = {
    candidateId: 'approval',
    ...source,
    kind: 'agent_approval',
    title: 'Approval Required: Dangerous Command',
    body: 'Review a request in Synara.',
    suppressIfFocusedRoom: false,
    isEncrypted: true,
  };
  for (const kind of ['message', 'agent_approval'] as const) {
    const posted: unknown[] = [];
    let acknowledgements = 0;
    await deliverNativeObservedNotificationCandidate({
      candidate: { ...candidate, kind },
      observedGeneration: 7,
      currentGeneration: () => 7,
      // This is the mounted value during inbox triage or missing room metadata.
      presentOrdinaryMessages: false,
      deliver: async (accepted) => {
        posted.push(buildNativeObservedNotificationPresentation(accepted, source));
        return 'delivered';
      },
      acknowledge: async (id) => {
        assert.equal(id, candidate.candidateId);
        acknowledgements += 1;
      },
    });
    assert.equal(posted.length, kind === 'agent_approval' ? 1 : 0);
    assert.equal(
      acknowledgements,
      1,
      'both local suppress and delivered outcomes release ownership'
    );
  }
  const mounted = readFileSync(
    `${process.cwd()}/src/app/pages/client/ClientNonUIFeatures.tsx`,
    'utf8'
  );
  const start = mounted.indexOf('function MessageNotifications()');
  const end = mounted.indexOf('function LaterReminderNotifications()');
  const routes = mounted.slice(start, end);
  assert.doesNotMatch(routes, /if \(!room \|\| room\.isSpaceRoom\(\)\) return/);
  assert.doesNotMatch(routes, /if \(notificationSelected\) return/);
  assert.equal(
    (routes.match(/await deliverNativeObservedNotificationCandidate\(/g) ?? []).length,
    2
  );
});

test('generation replacement during asynchronous OS acceptance cancels only the old candidate before committing sound or cache', async () => {
  const registry = createObservedBrowserNotificationRegistry();
  const closed: string[] = [];
  registry.track('notif-7-1', { close: () => closed.push('old') });
  registry.track('notif-8-1', { close: () => closed.push('successor') });
  let generation: number | undefined = 7;
  let accepted!: () => void;
  const acceptance = new Promise<void>((resolve) => {
    accepted = resolve;
  });
  let posting!: () => void;
  const posted = new Promise<void>((resolve) => {
    posting = resolve;
  });
  const committed: unknown[] = [];
  const acknowledgements: string[] = [];
  const cancelled: string[] = [];
  const candidate: NativeNotificationCandidate = {
    candidateId: 'notif-7-1',
    roomId: OBSERVATION.roomId,
    eventId: OBSERVATION.eventId,
    kind: 'message',
    title: 'Message',
    body: 'New message',
    suppressIfFocusedRoom: true,
    isEncrypted: false,
  };
  const delivery = deliverNativeObservedNotificationCandidate({
    candidate,
    observedGeneration: 7,
    currentGeneration: () => generation,
    deliver: async () => {
      posting();
      await acceptance;
      return 'delivered';
    },
    commit: (outcome) => {
      committed.push(outcome);
    },
    cancelDelivery: async () => {
      registry.cancel(candidate.candidateId);
      cancelled.push(`candidate:${candidate.candidateId}`);
    },
    acknowledge: async (id) => {
      acknowledgements.push(id);
    },
  });
  await posted;
  generation = 8;
  accepted();
  await delivery;
  assert.deepEqual(committed, []);
  assert.deepEqual(cancelled, ['candidate:notif-7-1']);
  assert.deepEqual(closed, ['old']);
  assert.deepEqual(acknowledgements, ['notif-7-1']);
  registry.clear();
  assert.deepEqual(closed, ['old', 'successor']);
});

test('retained OS actions reject missing and successor generations without rebinding', () => {
  for (const [original, current] of [
    [undefined, 8],
    [7, undefined],
    [7, 8],
    [NaN, 7],
    [1.5, 1.5],
  ]) {
    assert.equal(isNativeNotificationActionForSession(original, current), false);
  }
  assert.equal(isNativeNotificationActionForSession(7, 7), true);
});
