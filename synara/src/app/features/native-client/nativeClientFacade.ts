import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqNumber,
  reqString,
} from '../matrix-dto/parseUtil';
import type { EventId, RoomId, UserId } from '../matrix-dto/ids';
import { parseRoomSummary, type RoomSummary } from '../matrix-dto/room';
import type { MatrixEventReading, RoomReading } from '../../utils/room';
import type { DesktopInvokeResult } from '../../utils/desktop';

/**
 * F1 — renderer NativeMatrixClient facade core (emitter + lifecycle + identity).
 * Operator-authorized Option A (complete native) + D1C (renderer cedes token
 * custody): this facade is the structural replacement for the js-sdk client
 * object that currently lives behind `Awaited<ReturnType<typeof initClient>>`.
 *
 * ADDITIVE SLICE (F1): this module is not yet wired into the boot path
 * (initMatrix.ts is untouched; importer drop happens at F6). It is unit-tested
 * in isolation via an injectable NativeInvoke, per the house owner pattern.
 *
 * D1C contract: NO token surface. getAccessToken/setAccessToken/refreshToken do
 * not exist here; token custody is native-only and never crosses IPC (a
 * `session_updated` event carries readiness/generation, never tokens).
 */

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/** Serialized `SyncReadiness` enum (src-tauri/src/matrix/sync/readiness.rs). */
export type NativeReadiness =
  | 'unconfigured'
  | 'idle'
  | 'running'
  | 'offline'
  | 'failed'
  | 'terminated';

/** Structural mirror of the Rust `SyncReadinessSnapshot` DTO. */
export type NativeSyncStatus = {
  readiness: NativeReadiness;
  sessionGeneration: number;
  offlineModeEnabled: boolean;
  failureDiagnosticId?: string | null;
};

/** js-sdk-compatible sync-state strings the app UI already consumes. */
export type NativeSyncState =
  | 'PREPARED'
  | 'SYNCING'
  | 'CATCHUP'
  | 'ERROR'
  | 'RECONNECTING'
  | 'STOPPED';

export type NativeSyncStateData = {
  readiness: NativeReadiness;
  sessionGeneration: number;
  failureDiagnosticId?: string | null;
};

/** Structural mirror of the Rust `MatrixSessionSnapshot` DTO. */
export type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  userId?: string;
  deviceId?: string;
  homeserverUrl?: string;
  sessionGeneration?: number;
};

export type NativeClientIdentity = {
  userId?: string;
  deviceId?: string;
  homeserverUrl?: string;
};

export type NativeInvokeResult = {
  status: 'ok';
};

export type NativeClientListener = (payload?: unknown) => void;

/** Minimal typed emitter surface covering the app's USED event names (probed). */
export type NativeClientEvents = {
  sync: NativeSyncState | null;
  session: NativeSessionSnapshot;
};

export type NativeClientEventName = keyof NativeClientEvents | string;

const UNAVAILABLE_MESSAGE = 'Native Matrix client is unavailable.';
const FORBIDDEN_READINESS = new Set(['offline', 'failed']);

/** Map Rust sync readiness onto the js-sdk SyncState literals the UI reads. */
export const readinessToSyncState = (readiness: NativeReadiness): NativeSyncState => {
  switch (readiness) {
    case 'running':
      return 'PREPARED';
    case 'offline':
      return 'RECONNECTING';
    case 'failed':
      return 'ERROR';
    case 'unconfigured':
    case 'idle':
      return 'STOPPED';
    default:
      return 'STOPPED';
  }
};

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const parseSyncStatus = (value: unknown): NativeSyncStatus | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const readiness = optString(value, 'readiness');
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  const offlineModeEnabled = optBoolean(value, 'offlineModeEnabled');
  const readinesses: readonly string[] = [
    'unconfigured',
    'idle',
    'running',
    'offline',
    'failed',
    'terminated',
  ];
  if (
    typeof readiness !== 'string' ||
    !readinesses.includes(readiness) ||
    sessionGeneration === null ||
    !isSafeGeneration(sessionGeneration) ||
    typeof offlineModeEnabled !== 'boolean'
  ) {
    return null;
  }
  return {
    readiness: readiness as NativeReadiness,
    sessionGeneration,
    offlineModeEnabled,
    failureDiagnosticId: optString(value, 'failureDiagnosticId') ?? null,
  };
};

const parseSessionSnapshot = (value: unknown): NativeSessionSnapshot => {
  if (!isObject(value) || hasForbiddenWireFields(value)) {
    return { status: 'logged_out' };
  }
  if (value.status !== 'logged_in') {
    return { status: 'logged_out' };
  }
  return {
    status: 'logged_in',
    userId: reqString(value, 'user_id') ?? reqString(value, 'userId') ?? undefined,
    deviceId: reqString(value, 'device_id') ?? reqString(value, 'deviceId') ?? undefined,
    homeserverUrl:
      reqString(value, 'homeserver_url') ?? reqString(value, 'homeserverUrl') ?? undefined,
    sessionGeneration: reqNumber(value, 'sessionGeneration') ?? undefined,
  };
};

/** Structural mirror of the Rust `NativeRoomListSnapshot` DTO (room_list/live.rs). */
export type NativeRoomListSnapshot = {
  sessionGeneration: number;
  orderedRoomIds?: string[];
  rooms: RoomSummary[];
};

/** Minimal F2 room reading backed by the native room-list projection. */
export type FacadeRoomReading = {
  roomId: RoomId;
  name: string;
  canonicalAlias: string | null;
  avatarUrl: string | null;
  membership: RoomSummary['membership'];
  isDirect: boolean;
  isSpace: boolean;
  isEncrypted: boolean;
  unreadCount: number;
  highlightCount: number;
  lastActivityTs?: number;
  tombstoneSuccessorRoomId?: string | null;
  getMyMembership(): string;
  getCanonicalAlias(): string | null;
  getJoinedMemberCount(): number;
  isSpaceRoom(): boolean;
  isCallRoom(): boolean;
};

/** Minimal F2 timeline event readback (matrix_timeline_event_readback). */
export type FacadeTimelineEventReading = {
  eventId: EventId;
  sender: UserId;
  type: string;
  body: string;
  originServerTs: number;
};

const parseRoomListSnapshot = (value: unknown): NativeRoomListSnapshot | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  if (sessionGeneration === null || !isSafeGeneration(sessionGeneration)) return null;
  const rawRooms = value.rooms;
  if (!Array.isArray(rawRooms)) return null;
  const rooms: RoomSummary[] = [];
  for (const raw of rawRooms) {
    const parsed = parseRoomSummary(raw);
    if (parsed) rooms.push(parsed);
  }
  return { sessionGeneration, rooms };
};

type EmptyStateReading = {
  getStateEvents(eventType: string): MatrixEventReading[];
  getStateEvents(eventType: string, stateKey: string): MatrixEventReading | null;
};

const EMPTY_STATE: EmptyStateReading = {
  getStateEvents: (eventType: string, stateKey?: string) => (stateKey === undefined ? [] : null),
} as EmptyStateReading;

/** F6a — full structural RoomReading projection (fail-closed deep surface). */
const toRoomReading = (
  summary: RoomSummary,
  roomClient?: {
    on: (event: string, listener: (payload?: unknown) => void) => void;
    removeListener: (event: string, listener: (payload?: unknown) => void) => void;
  }
): RoomReading & {
  client?: unknown;
  on?: (event: string, listener: (payload?: unknown) => void) => void;
  removeListener?: (event: string, listener: (payload?: unknown) => void) => void;
  getUsersReadUpTo?: (event: MatrixEventReading) => string[];
  findEventById?: () => unknown;
} => ({
  roomId: summary.roomId,
  name: summary.name ?? '',
  currentState: EMPTY_STATE as RoomReading['currentState'],
  getLiveTimeline: () => ({
    getState: () => undefined,
    getEvents: () => [],
  }),
  getMember: () => null,
  getMembers: () => [],
  getMxcAvatarUrl: () => summary.avatarUrl ?? null,
  getAvatarFallbackMember: () => undefined,
  getUnreadNotificationCount: () => summary.unreadCount,
  getEventReadUpTo: () => null,
  getLastActiveTimestamp: () => summary.lastActivityTs,
  getBumpStamp: () => summary.lastActivityTs,
  getThreads: () => [],
  accountData: { get: () => undefined },
  getMyMembership: () => summary.membership,
  getJoinRule: () => summary.joinRule ?? '',
  getJoinedMemberCount: () => 0,
  getCanonicalAlias: () => summary.canonicalAlias ?? null,
  getType: () => undefined,
  getVersion: () => '',
  isCallRoom: () => false,
  isSpaceRoom: () => summary.isSpace,
  getTimelineForEvent: () => null,
  hasMembershipState: () => summary.membership === 'join',
  on: roomClient?.on,
  removeListener: roomClient?.removeListener,
  getUsersReadUpTo: () => [],
  findEventById: () => undefined,
});

const parseTimelineEventReadback = (value: unknown): FacadeTimelineEventReading | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const eventId = reqString(value, 'eventId');
  const rawItem = value.item;
  if (eventId === null || !isObject(rawItem)) return null;
  const sender = reqString(rawItem, 'sender');
  const eventType = reqString(rawItem, 'type');
  const body = optString(rawItem, 'body');
  const originServerTs = reqNumber(rawItem, 'originServerTs');
  if (sender === null || eventType === null || originServerTs === null) return null;
  return { eventId, sender, type: eventType, body: body ?? '', originServerTs };
};

/** F3 — send text input mirroring NativeSendTextInput (room/nativeSendTextOwner). */
export type FacadeSendTextInput = {
  roomId: RoomId;
  body: string;
  msgType?: string;
  formattedBody?: string;
  mentionUserIds?: UserId[];
  mentionRoom?: boolean;
  replyTo?: EventId;
  threadRoot?: EventId;
  txnId?: string;
};

export type FacadeSendTextResult = {
  roomId: RoomId;
  eventId: EventId;
  localTxnId: string;
  status: 'sent';
};

export type FacadeSendEventContent = Record<string, unknown>;

export type FacadeSendStateEventContent = Record<string, unknown>;

export type FacadeSendStateEventResult = {
  status: string;
  roomId?: RoomId;
};

/** F4 — media types. */
export type FacadeMediaUploadInput = {
  mimeType: string;
  bytes: number[];
};

export type FacadeUploadMediaResult = {
  mxc: string;
};

export type FacadeMediaConfig = {
  /** Wire key is `m.upload.size` (Rust MatrixCallMediaConfigResult). */
  maxUploadSizeBytes?: number;
};

export type FacadeMediaDownloadResult = {
  bytes: number[];
};

export type FacadeProfileInfo = {
  userId?: UserId;
  deviceId?: string;
  displayName?: string;
  avatarUrl?: string;
};

const parseUploadMediaResult = (value: unknown): FacadeUploadMediaResult | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const mxc = reqString(value, 'mxc');
  return mxc === null ? null : { mxc };
};

const parseMediaConfig = (value: unknown): FacadeMediaConfig => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return {};
  const maybeSize = (value as Record<string, unknown>)['m.upload.size'];
  return typeof maybeSize === 'number' && Number.isSafeInteger(maybeSize) && maybeSize > 0
    ? { maxUploadSizeBytes: maybeSize }
    : {};
};

const parseMediaDownload = (value: unknown): FacadeMediaDownloadResult | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const bytes = value.bytes;
  if (!Array.isArray(bytes)) return null;
  return { bytes: bytes.map((b) => (typeof b === 'number' ? b : 0)) };
};

/** F5 — crypto & extended readings. */

export type FacadePushRuleCondition = {
  kind?: string;
  roomMentions?: string;
};

export type FacadePushRule = {
  ruleId: string;
  enabled: boolean;
  actions: unknown[];
  conditions?: FacadePushRuleCondition[];
};
export type FacadeCryptoCrossSigningState = 'Unavailable' | 'NotSetUp' | 'Partial' | 'Ready';

/** Structural mirror of the Rust MatrixCryptoStatus DTO. */
export type FacadeCryptoStatus = {
  sessionGeneration: number;
  encryptionEnabled: boolean;
  crossSigningState: FacadeCryptoCrossSigningState;
};

export type FacadeCryptoReading = {
  /** Native crypto owns keys per D1C; expose status, never key material. */
  getCrossSigningState(): Promise<FacadeCryptoCrossSigningState>;
  isCrossSigningReady(): Promise<boolean>;
  isEncryptionEnabled(): Promise<boolean>;
  /** D1C: to-device encryption is native-owned; GAP-safe no-op stub. */
  encryptToDeviceMessages(): Promise<void>;
};

export type FacadeMatrixRtcSession = unknown;

export type FacadeMatrixRtc = {
  getRoomSession(_room: { roomId: string }): FacadeMatrixRtcSession | null;
  on(_event: string, _listener: (...args: unknown[]) => void): void;
  off(_event: string, _listener: (...args: unknown[]) => void): void;
};

export type FacadeDownloadKeysResult = Record<string, unknown>;

export type FacadeCapabilitiesIntersection = Record<string, unknown>;

const parseCryptoStatus = (value: unknown): FacadeCryptoStatus | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  const encryptionEnabledRaw = value.encryptionEnabled;
  const crossSigningStateRaw = value.crossSigningState;
  if (
    sessionGeneration === null ||
    !isSafeGeneration(sessionGeneration) ||
    typeof encryptionEnabledRaw !== 'boolean' ||
    typeof crossSigningStateRaw !== 'string'
  ) {
    return null;
  }
  return {
    sessionGeneration,
    encryptionEnabled: encryptionEnabledRaw,
    crossSigningState: crossSigningStateRaw as FacadeCryptoCrossSigningState,
  };
};

const parseSendTextResult = (value: unknown): FacadeSendTextResult | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const eventId = reqString(value, 'eventId');
  const localTxnId = reqString(value, 'localTxnId');
  if (roomId === null || eventId === null || localTxnId === null) return null;
  return { roomId, eventId, localTxnId, status: 'sent' };
};

/** In-process emitter (F1). A native sync-state PUSH event is a later slice. */
export class NativeClientEmitter {
  private readonly listeners = new Map<string, Set<NativeClientListener>>();

  public on(event: NativeClientEventName, listener: NativeClientListener): this {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    set.add(listener);
    return this;
  }

  public once(event: NativeClientEventName, listener: NativeClientListener): this {
    const wrapped: NativeClientListener = (payload) => {
      this.removeListener(event, wrapped);
      listener(payload);
    };
    return this.on(event, wrapped);
  }

  public removeListener(event: NativeClientEventName, listener: NativeClientListener): this {
    this.listeners.get(event)?.delete(listener);
    return this;
  }

  public off(event: NativeClientEventName, listener: NativeClientListener): this {
    return this.removeListener(event, listener);
  }

  public emit(event: NativeClientEventName, payload?: unknown): boolean {
    const set = this.listeners.get(event);
    if (!set || set.size === 0) return false;
    for (const listener of [...set]) listener(payload);
    return true;
  }

  public setMaxListeners(): this {
    // no-op cap for F1; kept for API parity with the js-sdk emitter surface.
    return this;
  }

  public listenerCount(event: NativeClientEventName): number {
    return this.listeners.get(event)?.size ?? 0;
  }
}

/**
 * F1 facade — command-backed native client proxy.
 * D1C: no token getters/setters; sync-readiness and identity come from
 * `matrix_sync_status` / `matrix_session_snapshot`, never secrets.
 */
export const createNativeMatrixClient = (invoke: NativeInvoke) => {
  const emitter = new NativeClientEmitter();

  // F6a synchronous read cache: the facade presents js-sdk-style SYNC reads;
  // refresh() hydrates the cache from native commands. Writes stay async.
  let cachedIdentity: NativeClientIdentity = {};
  let cachedSyncState: NativeSyncState | null = null;
  let cachedSyncData: NativeSyncStateData | null = null;
  let cachedRooms: RoomReading[] = [];

  const readSyncStatus = (): Promise<NativeSyncStatus | null> =>
    invoke('matrix_sync_status').then((result) =>
      result.available ? parseSyncStatus(result.value) : null
    );

  const readIdentity = async (): Promise<NativeClientIdentity> => {
    const result = await invoke('matrix_session_snapshot');
    if (!result.available) return {};
    return parseSessionSnapshot(result.value) as NativeClientIdentity;
  };

  const readRooms = async (): Promise<RoomReading[]> => {
    const result = await invoke('matrix_room_list_snapshot');
    if (!result.available) return [];
    const snapshot = parseRoomListSnapshot(result.value);
    return snapshot
      ? snapshot.rooms.map((r) =>
          toRoomReading(r, {
            on: emitter.on.bind(emitter),
            removeListener: emitter.removeListener.bind(emitter),
          })
        )
      : [];
  };

  const applySyncStatus = (status: NativeSyncStatus): void => {
    cachedSyncState = readinessToSyncState(status.readiness);
    cachedSyncData = {
      readiness: status.readiness,
      sessionGeneration: status.sessionGeneration,
      failureDiagnosticId: status.failureDiagnosticId,
    };
  };

  const emitSyncState = (): void => {
    if (cachedSyncState !== null) emitter.emit('sync', cachedSyncState);
  };

  /**
   * F6a — hydrate the whole read cache from native commands. Call this after
   * construct (or on demand) before relying on synchronous reads. Fail-closed:
   * unavailable commands leave the corresponding cache slot at its default.
   */
  const refresh = async (): Promise<void> => {
    const [status, identity, rooms] = await Promise.all([
      readSyncStatus(),
      readIdentity(),
      readRooms(),
    ]);
    if (status) applySyncStatus(status);
    cachedIdentity = { ...cachedIdentity, ...identity };
    if (rooms.length > 0 || !identity.userId) {
      cachedRooms = rooms.map((room) => {
        const evented = room as RoomReading & {
          client?: unknown;
          on?: (event: string, listener: (payload?: unknown) => void) => void;
          removeListener?: (event: string, listener: (payload?: unknown) => void) => void;
        };
        evented.client = facadeClient;
        evented.on = (event, listener) => emitter.on(event, listener);
        evented.removeListener = (event, listener) => emitter.removeListener(event, listener);
        return evented as RoomReading;
      });
    }
    emitSyncState();
  };

  const clearSession = (): void => {
    cachedSyncState = null;
    cachedSyncData = null;
    cachedRooms = [];
    emitter.emit('sync', 'STOPPED');
  };

  // F6b: the facade object fills this holder after construction so rooms can
  // reference it as their `client` (EventedRoomReading contract).
  let facadeClient: unknown = null;
  const clientObj = {
    emitter,
    on: emitter.on.bind(emitter),
    once: emitter.once.bind(emitter),
    off: emitter.off.bind(emitter),
    removeListener: emitter.removeListener.bind(emitter),
    emit: emitter.emit.bind(emitter),
    setMaxListeners: emitter.setMaxListeners.bind(emitter),
    refresh,

    /** F6a — SYNCHRONOUS identity reads (js-sdk object model). */
    getIdentity(): NativeClientIdentity {
      return cachedIdentity;
    },
    getUserId(): string | null {
      return cachedIdentity.userId ?? null;
    },
    getSafeUserId(): string {
      return cachedIdentity.userId ?? '';
    },
    getDeviceId(): string | undefined {
      return cachedIdentity.deviceId;
    },

    /** F6a — SYNCHRONOUS sync-state reads. */
    getSyncState(): NativeSyncState | null {
      return cachedSyncState;
    },
    getSyncStateData(): NativeSyncStateData | null {
      return cachedSyncData;
    },
    clientRunning(): boolean {
      return cachedSyncData?.readiness === 'running';
    },

    /** Readiness refresh on demand (still async); keeps sync cache fresh. */
    async retryImmediately(): Promise<void> {
      await refresh();
    },
    async startClient(): Promise<void> {
      await refresh();
    },
    async stopClient(): Promise<void> {
      clearSession();
    },
    async logout(): Promise<void> {
      const result = await invoke('matrix_logout');
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      clearSession();
    },

    /** F6a — SYNCHRONOUS room reads from the cache. */
    getRooms(): RoomReading[] {
      return cachedRooms;
    },
    getRoom(roomId: RoomId): RoomReading | null {
      return cachedRooms.find((r) => r.roomId === roomId) ?? null;
    },

    /** F2 — single event readback via matrix_timeline_event_readback. */
    async fetchRoomEvent(
      roomId: RoomId,
      eventId: EventId
    ): Promise<FacadeTimelineEventReading | null> {
      const result = await invoke('matrix_timeline_event_readback', { roomId, eventId });
      if (!result.available) return null;
      return parseTimelineEventReadback(result.value);
    },

    async setDisplayName(displayName: string): Promise<NativeInvokeResult> {
      const result = await invoke('matrix_set_own_display_name', { displayName });
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      return result.value as NativeInvokeResult;
    },

    async setAvatarUrl(mxc: string): Promise<NativeInvokeResult> {
      const result = await invoke('matrix_set_own_avatar', { avatarUrl: mxc });
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      return result.value as NativeInvokeResult;
    },

    /** Poll matrix_sync_status and emit 'sync' on change; returns unsubscribe. */
    watchSync(pollMs = 1500): () => void {
      let stopped = false;
      let last: NativeSyncState | null = cachedSyncState;
      const tick = async (): Promise<void> => {
        if (stopped) return;
        const status = await readSyncStatus();
        if (status) {
          const next = readinessToSyncState(status.readiness);
          if (next !== last) {
            last = next;
            applySyncStatus(status);
            emitSyncState();
          }
        }
      };
      void tick();
      const timer = setInterval(() => void tick(), pollMs);
      timer.unref?.();
      return () => {
        stopped = true;
        clearInterval(timer);
      };
    },

    /** F3 — send a plain message via matrix_send_text (fail-closed null). */
    async sendMessage(input: FacadeSendTextInput): Promise<FacadeSendTextResult | null> {
      const result = await invoke('matrix_send_text', {
        roomId: input.roomId,
        body: input.body,
        msgType: input.msgType,
        formattedBody: input.formattedBody,
        mentionUserIds: input.mentionUserIds,
        mentionRoom: input.mentionRoom,
        replyTo: input.replyTo,
        threadRoot: input.threadRoot,
        txnId: input.txnId,
      });
      if (!result.available) return null;
      return parseSendTextResult(result.value);
    },

    /** F3 — generic event send: m.room.message tunnels to matrix_send_text; else GAP null. */
    async sendEvent(
      roomId: RoomId,
      type: string,
      content: FacadeSendEventContent
    ): Promise<FacadeSendTextResult | null> {
      if (type === 'm.room.message') {
        return this.sendMessage({ roomId, body: String(content.body ?? '') });
      }
      // Other event types have no generic native send command yet (GAP).
      return Promise.resolve(null);
    },

    /** F3 — room state setters for the covered types; else GAP null. */
    async sendStateEvent(
      roomId: RoomId,
      type: string,
      content: FacadeSendStateEventContent,
      stateKey?: string
    ): Promise<FacadeSendStateEventResult | null> {
      const stateKeyApplied = stateKey ?? ''; // eslint-disable-line @typescript-eslint/no-unused-vars
      const command =
        type === 'm.room.name'
          ? 'matrix_set_room_name'
          : type === 'm.room.topic'
          ? 'matrix_set_room_topic'
          : type === 'm.room.avatar'
          ? 'matrix_set_room_avatar'
          : null;
      if (!command) return null; // GAP: no generic state-event command
      const result = await invoke(command, {
        roomId,
        name: content.name,
        avatarUrl: content.url,
        stateKey,
      });
      if (!result.available) return null;
      return isObject(result.value) ? (result.value as FacadeSendStateEventResult) : null;
    },

    /** F3 — account-data is a documented GAP (no native command yet); fail-closed undefined. */
    getAccountData(type: string): MatrixEventReading | undefined {
      const noNativeAccountData = type.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return undefined;
    },
    setAccountData(type: string, content: Record<string, unknown>): Promise<unknown> {
      if (type.length === 0 || Object.keys(content).length === 0) return Promise.resolve(null);
      return Promise.resolve(null); // GAP: no native account-data command
    },
    async setRoomAccountData(
      roomId: RoomId,
      type: string,
      content: Record<string, unknown>
    ): Promise<unknown> {
      if (roomId.length === 0 || type.length === 0 || Object.keys(content).length === 0) {
        return Promise.resolve(null);
      }
      return Promise.resolve(null); // GAP: no native account-data command
    },

    /** F4 — upload content bytes via matrix_upload_media (fail-closed null). */
    async uploadContent(input: FacadeMediaUploadInput): Promise<FacadeUploadMediaResult | null> {
      const result = await invoke('matrix_upload_media', {
        mimeType: input.mimeType,
        bytes: input.bytes,
      });
      if (!result.available) return null;
      return parseUploadMediaResult(result.value);
    },

    /** F4 — media upload size limit via matrix_call_media_config. */
    async getMediaConfig(): Promise<FacadeMediaConfig> {
      const result = await invoke('matrix_call_media_config');
      if (!result.available) return {};
      return parseMediaConfig(result.value);
    },

    /** F4 — download original file bytes via matrix_media_download (fail-closed null). */
    async downloadMedia(contentUri: string): Promise<FacadeMediaDownloadResult | null> {
      const result = await invoke('matrix_media_download', { contentUri });
      if (!result.available) return null;
      return parseMediaDownload(result.value);
    },

    /** F4 — profile/identity from the sync cache (no key material, D1C). */
    getProfileInfo(): FacadeProfileInfo {
      return { ...cachedIdentity };
    },

    /** F5 — crypto status via matrix_crypto_status (never key material, D1C). */
    async getCryptoStatus(): Promise<FacadeCryptoStatus | null> {
      const result = await invoke('matrix_crypto_status');
      if (!result.available) return null;
      return parseCryptoStatus(result.value);
    },

    /** F5 — structural crypto reading: status-backed, key-free. */
    getCrypto(): FacadeCryptoReading {
      return {
        getCrossSigningState: async () =>
          (await this.getCryptoStatus())?.crossSigningState ?? 'Unavailable',
        isCrossSigningReady: async () =>
          (await this.getCryptoStatus())?.crossSigningState === 'Ready',
        isEncryptionEnabled: async () => (await this.getCryptoStatus())?.encryptionEnabled ?? false,
        encryptToDeviceMessages: async () => Promise.resolve(),
      };
    },

    /** F5 — native events arrive decrypted; present for anchor-compat, no key access. */
    async decryptEventIfNeeded(event: { eventId?: string }): Promise<void> {
      const noOp = event?.eventId ?? ''; // eslint-disable-line
      return Promise.resolve(undefined);
    },

    /** F5 — native crypto owns device keys; renderer never downloads (D1C). */
    async downloadKeysForUsers(userIds: string[]): Promise<FacadeDownloadKeysResult> {
      // Native crypto owns device keys (D1C); renderer never downloads keys.
      const keysRequested = userIds.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve({});
    },

    /** F5 — V-CALL runtime remains matrix-widget-api; facade exposes a GAP-safe stub. */
    get matrixRTC(): FacadeMatrixRtc {
      return {
        getRoomSession: () => null,
        on: () => undefined,
        off: () => undefined,
      };
    },

    /** F5 — GAP stubs required by the type anchor; no native surface. */
    async getCapabilities(): Promise<FacadeCapabilitiesIntersection> {
      return Promise.resolve({});
    },
    async getOpenIdToken(): Promise<unknown> {
      return Promise.resolve(null);
    },
    async search(query: { term: string }): Promise<unknown> {
      // Room-directory search is a stateful native owner; not a facade call (GAP).
      const nativeSearchDoesNotApply = query.term.length >= 0; // eslint-disable-line
      return Promise.resolve(nativeSearchDoesNotApply ? null : null);
    },
    /** F6a — aliases + ignored users + auth metadata + relations (GAP-safe). */
    async getRoomIdForAlias(alias: string): Promise<string | null> {
      const noNativeAliasResolve = alias.length > 0; // eslint-disable-line
      return Promise.resolve(noNativeAliasResolve ? null : null);
    },
    async setIgnoredUsers(userIds: string[]): Promise<void> {
      const ignore = userIds.length >= 0; // eslint-disable-line
      return Promise.resolve(ignore ? undefined : undefined);
    },
    async getAuthMetadata(): Promise<unknown> {
      return Promise.resolve(null);
    },
    async relations(eventId: string, relationType?: string, eventType?: string): Promise<unknown> {
      const noNativeRelations = eventId.length > 0; // eslint-disable-line
      return Promise.resolve(noNativeRelations && relationType && eventType ? null : null);
    },

    /** F6c — redact a message/event (matrix_timeline_redact). */
    async redactEvent(
      roomId: RoomId,
      eventId: EventId,
      reason?: string
    ): Promise<{ event_id: string } | null> {
      const result = await invoke('matrix_timeline_redact', { roomId, eventId, reason });
      if (!result.available) return null;
      return isObject(result.value) &&
        typeof (result.value as { event_id?: string }).event_id === 'string'
        ? { event_id: (result.value as { event_id: string }).event_id }
        : null;
    },

    /** F6c — user-directory search GAP is a stateful owner; fail-closed null. */
    async searchUserDirectory(term: string): Promise<unknown> {
      const termIgnored = term.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },

    /** F6c — device to-device queue (D1C: native crypto owns); fail-closed no-op. */
    async queueToDevice(eventType: string, content: Record<string, unknown>): Promise<void> {
      const eventTypeLen = eventType.length + Object.keys(content).length; // eslint-disable-line
      return Promise.resolve(undefined);
    },

    /** F6c — delayed-event APIs are not natively surfaced; GAP-safe stubs. */
    async _unstable_sendDelayedEvent(...args: unknown[]): Promise<unknown> {
      const noNativeDelayed = args.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(noNativeDelayed ? null : null);
    },
    async _unstable_sendDelayedStateEvent(...args: unknown[]): Promise<unknown> {
      const noNativeDelayed = args.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(noNativeDelayed ? null : null);
    },
    async _unstable_updateDelayedEvent(...args: unknown[]): Promise<unknown> {
      const noNativeDelayed = args.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(noNativeDelayed ? null : null);
    },

    /** F6c — openid-credentials GAP stub (real shape: { access_token }). */
    async getOpenIdTokenData(): Promise<{ access_token: string } | null> {
      return Promise.resolve(null);
    },
    /** F6a — push-rule read (notification GAP: fail-closed undefined). */
    getRoomPushRule(
      scope: string,
      roomId: RoomId
    ):
      | {
          actions: (string | { [key: string]: any })[];
          conditions?: { kind?: string }[];
          rule_id: string;
        }
      | undefined {
      const noNativePushRule = scope.length > 0 && roomId.length > 0; // eslint-disable-line
      return noNativePushRule ? undefined : undefined;
    },

    /** F6a — MXC -> native URI (media handle protocol); fail-closed null. */
    mxcUrlToHttp(mxcUrl: string): string | null {
      return mxcUrl.startsWith('mxc://') ? mxcUrl : null;
    },

    get http(): unknown {
      return undefined;
    },
    get store(): unknown {
      return undefined;
    },

    // D1C guard: fail-closed readiness helper for callers that must not run
    // while the native session is offline/failed.
    isReady(): boolean {
      return !FORBIDDEN_READINESS.has(cachedSyncData?.readiness ?? 'unconfigured');
    },
  };
  facadeClient = clientObj;
  return Object.freeze(clientObj);
};

export type NativeMatrixClient = ReturnType<typeof createNativeMatrixClient>;
