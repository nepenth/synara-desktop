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
import type { MatrixClientReading, MatrixEventReading, RoomReading } from '../../utils/room';
import type { DesktopInvokeResult } from '../../utils/desktop';
import { RoomType } from '../../../types/matrix/room';

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

/** Structural match to `EventedRoomReading` (utils/roomEvents) so the facade's
 * synchronous room cache satisfies consumer casts without per-file edits. */
export type FacadeEventedRoomReading = RoomReading & {
  client: MatrixClientReading;
  on(event: string, listener: (...args: unknown[]) => void): void;
  removeListener(event: string, listener: (...args: unknown[]) => void): void;
  getUsersReadUpTo(event: MatrixEventReading): string[];
  /** Real js-sdk room method; native has no equivalent — fail-closed stub. */
  findEventById(eventId: string): MatrixEventReading | undefined;
  /** js-sdk encryption-state read; native event-room projection lacks it — fail-closed false. */
  hasEncryptionStateEvent(): boolean;
};

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
  /** Tri-state server capability probe: true=support, false=absent, null=unprobed. */
  slidingSyncCapable?: boolean | null;
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
  slidingSyncCapable?: boolean | null;
};

/** Structural mirror of the Rust `MatrixSessionSnapshot` DTO. */
export type NativeSessionSnapshot =
  | { status: 'logged_out' }
  | {
      status: 'logged_in';
      userId: string;
      deviceId: string;
      homeserverUrl: string;
      sessionGeneration: number;
    };

export type NativeClientIdentity = {
  userId?: string;
  deviceId?: string;
  homeserverUrl?: string;
  avatarUrl?: string;
  displayName?: string;
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
    slidingSyncCapable: optBoolean(value, 'slidingSyncCapable') ?? null,
  };
};

const parseSessionSnapshot = (value: unknown): NativeSessionSnapshot | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  if (value.status === 'logged_out') return { status: 'logged_out' };
  if (value.status !== 'logged_in') return null;
  const userId = reqString(value, 'user_id') ?? reqString(value, 'userId');
  const deviceId = reqString(value, 'device_id') ?? reqString(value, 'deviceId');
  const homeserverUrl = reqString(value, 'homeserver_url') ?? reqString(value, 'homeserverUrl');
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  if (
    userId === null ||
    deviceId === null ||
    homeserverUrl === null ||
    sessionGeneration === null ||
    !isSafeGeneration(sessionGeneration)
  ) {
    return null;
  }
  return { status: 'logged_in', userId, deviceId, homeserverUrl, sessionGeneration };
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
  lastMessagePreview?: string;
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

const TIMELINE_MEDIA_HANDLE_PREFIX = 'timeline-media-';

/** Prefer an opaque timeline handle over leftover `mxc://` or protocol URLs. */
export const timelineMediaHandleFromUri = (contentUri: string): string | null => {
  const trimmed = contentUri.trim();
  if (trimmed.startsWith(TIMELINE_MEDIA_HANDLE_PREFIX)) {
    return trimmed;
  }
  const match = /^synara-media:\/\/[^/]*\/(.+)$/i.exec(trimmed);
  if (!match?.[1]) {
    return null;
  }
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return match[1];
  }
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

type FacadeRoomSummaryRef = { current: RoomSummary };

/**
 * F6a — full structural RoomReading projection (fail-closed deep surface).
 *
 * The native room-list poll replaces a summary frequently. Keep a stable room
 * wrapper and read through this ref so consumers holding a room object do not
 * keep a stale name, avatar, membership, or unread count.
 */
const toRoomReading = (
  summaryRef: FacadeRoomSummaryRef,
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
} => {
  const summary = (): RoomSummary => summaryRef.current;
  return {
    get roomId() {
      return summary().roomId;
    },
    get name() {
      return summary().name ?? '';
    },
    currentState: EMPTY_STATE as RoomReading['currentState'],
    getLiveTimeline: () => ({
      getState: () => undefined,
      getEvents: () => [],
    }),
    getMember: () => null,
    getMembers: () => [],
    getMxcAvatarUrl: () => summary().avatarUrl ?? null,
    getAvatarFallbackMember: () => undefined,
    getUnreadNotificationCount: () => summary().unreadCount,
    getEventReadUpTo: () => null,
    getLastActiveTimestamp: () => summary().lastActivityTs,
    getBumpStamp: () => summary().lastActivityTs,
    get lastMessagePreview() {
      return summary().lastMessagePreview;
    },
    getThreads: () => [],
    accountData: { get: () => undefined },
    getMyMembership: () => summary().membership,
    getJoinRule: () => summary().joinRule ?? '',
    getJoinedMemberCount: () => 0,
    getCanonicalAlias: () => summary().canonicalAlias ?? null,
    getType: () => (summary().isCall ? RoomType.Call : undefined),
    getVersion: () => '',
    isCallRoom: () => summary().isCall,
    isSpaceRoom: () => summary().isSpace,
    getTimelineForEvent: () => null,
    hasMembershipState: () => summary().membership === 'join',
    on: roomClient?.on,
    removeListener: roomClient?.removeListener,
    getUsersReadUpTo: () => [],
    findEventById: () => undefined,
  };
};

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
  /** native send-result alias (event_id) kept for existing consumers. */
  event_id?: string;
  localTxnId: string;
  status: 'sent';
};

export type FacadeSendEventContent = Record<string, unknown>;

export type FacadeSendStateEventContent = Record<string, unknown>;

export type FacadeSendStateEventResult = {
  status: string;
  roomId?: RoomId;
  event_id?: string;
};

/** F4 — media types. */
export type FacadeMediaUploadInput = {
  mimeType: string;
  bytes: number[];
};

export type FacadeUploadMediaResult = {
  mxc: string;
  /** native upload response alias (content_uri = mxc) for media consumers. */
  content_uri: string;
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
  return mxc === null ? null : { mxc, content_uri: mxc };
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
  /** D1C: to-device encryption is native-owned; GAP-safe no-op stub (batch shape). */
  encryptToDeviceMessages(
    _eventType: string,
    _recipients: Array<{ userId: string; deviceId: string }>,
    _content: unknown
  ): Promise<Array<{ userId: string; deviceId: string; payload: unknown }>>;
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
  let cachedRooms: FacadeEventedRoomReading[] = [];
  let cachedRoomEntries = new Map<
    string,
    { summaryRef: FacadeRoomSummaryRef; room: FacadeEventedRoomReading }
  >();
  let cachedSessionGeneration: number | undefined;

  // F6b: the facade object fills this holder after construction so rooms can
  // reference it as their `client` (EventedRoomReading contract).
  let facadeClient: unknown = null;

  const clearCachedRooms = (): void => {
    cachedRooms = [];
    cachedRoomEntries = new Map();
  };

  const readSyncStatus = (): Promise<NativeSyncStatus | null> =>
    invoke('matrix_sync_status').then((result) =>
      result.available ? parseSyncStatus(result.value) : null
    );

  const readSession = async (): Promise<NativeSessionSnapshot | null> => {
    const result = await invoke('matrix_session_snapshot');
    return result.available ? parseSessionSnapshot(result.value) : null;
  };

  const readRooms = async (): Promise<NativeRoomListSnapshot | null> => {
    const result = await invoke('matrix_room_list_snapshot');
    return result.available ? parseRoomListSnapshot(result.value) : null;
  };

  const createEventedRoom = (summaryRef: FacadeRoomSummaryRef): FacadeEventedRoomReading => {
    const room = toRoomReading(summaryRef, {
      on: emitter.on.bind(emitter),
      removeListener: emitter.removeListener.bind(emitter),
    }) as unknown as FacadeEventedRoomReading;
    room.client = facadeClient as MatrixClientReading;
    room.on = (event, listener) => {
      emitter.on(event, listener as (...args: unknown[]) => void);
    };
    room.removeListener = (event, listener) => {
      emitter.removeListener(event, listener as (...args: unknown[]) => void);
    };
    room.getUsersReadUpTo = () => [];
    room.hasEncryptionStateEvent = () => false;
    room.findEventById = () => undefined;
    return room;
  };

  /**
   * Apply one already-validated native room-list projection. This is called by
   * both facade refreshes and the atom owner, so `getRoom()` observes the same
   * snapshot that drives sidebar ordering. Existing room wrapper identities
   * remain stable while their summary reads become current.
   */
  const applyRoomListSnapshot = (snapshot: NativeRoomListSnapshot): void => {
    if (
      cachedSessionGeneration !== undefined &&
      snapshot.sessionGeneration !== cachedSessionGeneration
    ) {
      return;
    }
    const nextEntries = new Map<
      string,
      { summaryRef: FacadeRoomSummaryRef; room: FacadeEventedRoomReading }
    >();
    const nextRooms: FacadeEventedRoomReading[] = [];
    for (const summary of snapshot.rooms) {
      let entry = cachedRoomEntries.get(summary.roomId);
      if (entry) {
        entry.summaryRef.current = summary;
      } else {
        const summaryRef = { current: summary };
        entry = { summaryRef, room: createEventedRoom(summaryRef) };
      }
      nextEntries.set(summary.roomId, entry);
      nextRooms.push(entry.room);
    }
    cachedRoomEntries = nextEntries;
    cachedRooms = nextRooms;
  };

  const applySyncStatus = (status: NativeSyncStatus): void => {
    cachedSyncState = readinessToSyncState(status.readiness);
    cachedSyncData = {
      readiness: status.readiness,
      sessionGeneration: status.sessionGeneration,
      failureDiagnosticId: status.failureDiagnosticId,
      slidingSyncCapable: status.slidingSyncCapable,
    };
  };

  const emitSyncState = (): void => {
    if (cachedSyncState !== null) emitter.emit('sync', cachedSyncState);
  };

  const clearSession = ({
    clearIdentity = false,
    notifyLoggedOut = false,
  }: { clearIdentity?: boolean; notifyLoggedOut?: boolean } = {}): void => {
    const hadIdentity = Boolean(cachedIdentity.userId || cachedIdentity.deviceId);
    const hadSessionState = hadIdentity || cachedSyncState !== null || cachedRooms.length > 0;
    if (clearIdentity) cachedIdentity = {};
    cachedSyncState = null;
    cachedSyncData = null;
    cachedSessionGeneration = undefined;
    clearCachedRooms();
    if (hadSessionState) emitter.emit('sync', 'STOPPED');
    if (notifyLoggedOut && hadIdentity) {
      emitter.emit('session', { status: 'logged_out' } satisfies NativeSessionSnapshot);
      // ClientRoot retains the js-sdk-compatible session notification name.
      emitter.emit('Session.logged_out');
    }
  };

  /**
   * Apply a definitive native session snapshot before its room-list projection.
   * A new generation/user/device must never reuse the previous session's room
   * wrappers, and stale projections are rejected by applyRoomListSnapshot.
   */
  const applyNativeSessionSnapshot = (session: NativeSessionSnapshot): boolean => {
    if (session.status === 'logged_out') {
      clearSession({ clearIdentity: true, notifyLoggedOut: true });
      return false;
    }

    const sessionChanged =
      (cachedSessionGeneration !== undefined &&
        session.sessionGeneration !== undefined &&
        cachedSessionGeneration !== session.sessionGeneration) ||
      (cachedIdentity.userId !== undefined &&
        session.userId !== undefined &&
        cachedIdentity.userId !== session.userId) ||
      (cachedIdentity.deviceId !== undefined &&
        session.deviceId !== undefined &&
        cachedIdentity.deviceId !== session.deviceId);
    if (sessionChanged) {
      const hadSyncState = cachedSyncState !== null;
      clearCachedRooms();
      cachedSyncState = null;
      cachedSyncData = null;
      if (hadSyncState) emitter.emit('sync', 'STOPPED');
      // A replacement is not a logout: callers can rebuild user-scoped state
      // without issuing matrix_logout against the newly active native session.
      emitter.emit('session', session);
    }

    cachedIdentity = {
      userId: session.userId,
      deviceId: session.deviceId,
      homeserverUrl: session.homeserverUrl,
    };
    cachedSessionGeneration = session.sessionGeneration;
    return true;
  };

  /**
   * F6a — hydrate the whole read cache from native commands. Call this after
   * construct (or on demand) before relying on synchronous reads. Fail-closed:
   * unavailable commands preserve their corresponding last-known cache slot.
   */
  const refresh = async (): Promise<void> => {
    const [status, session, rooms] = await Promise.all([
      readSyncStatus(),
      readSession(),
      readRooms(),
    ]);

    if (session && !applyNativeSessionSnapshot(session)) return;

    const syncStateBeforeStatus = cachedSyncState;
    if (status) applySyncStatus(status);
    if (rooms) applyRoomListSnapshot(rooms);
    if (cachedSyncState !== syncStateBeforeStatus) emitSyncState();
  };

  const clientObj = {
    emitter,
    on: emitter.on.bind(emitter),
    once: emitter.once.bind(emitter),
    off: emitter.off.bind(emitter),
    removeListener: emitter.removeListener.bind(emitter),
    emit: emitter.emit.bind(emitter),
    setMaxListeners: emitter.setMaxListeners.bind(emitter),
    refresh,
    /** Native room-list atom owner supplies its validated, live snapshot here. */
    applyRoomListSnapshot,
    /** Native room-list owner applies its session snapshot before room IDs. */
    applyNativeSessionSnapshot,

    /** F6a — SYNCHRONOUS identity reads (js-sdk object model). */
    getIdentity(): NativeClientIdentity {
      return cachedIdentity;
    },
    getBaseUrl(): string | null {
      return cachedIdentity.homeserverUrl ?? null;
    },
    get baseUrl(): string | null {
      return cachedIdentity.homeserverUrl ?? null;
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
      // Keep identity until the caller has finished user-keyed local cleanup.
      clearSession();
    },
    async logout(): Promise<void> {
      const result = await invoke('matrix_logout');
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      clearSession({ clearIdentity: true });
    },

    /** F6a — SYNCHRONOUS room reads from the cache (evented projection). */
    getRooms(): FacadeEventedRoomReading[] {
      return cachedRooms;
    },
    getRoom(roomId: string | null | undefined): FacadeEventedRoomReading | null {
      if (typeof roomId !== 'string') return null;
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

    /** Poll native sync readiness and emit only transitions; returns unsubscribe. */
    watchSync(pollMs = 1500): () => void {
      let stopped = false;
      let inFlight = false;
      let last: NativeSyncState | null = cachedSyncState;
      const tick = async (): Promise<void> => {
        if (stopped || inFlight) return;
        inFlight = true;
        try {
          const status = await readSyncStatus();
          if (stopped || !status) return;
          const next = readinessToSyncState(status.readiness);
          if (next === last && cachedSyncState === next) return;
          last = next;
          applySyncStatus(status);
          emitSyncState();
        } catch {
          // A transient IPC failure must not strand or stop the lifecycle poll.
        } finally {
          inFlight = false;
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

    /** F3 — send a plain message via matrix_send_text (js-sdk arity; fail-closed null). */
    async sendMessage(
      roomId: RoomId,
      content: Record<string, unknown>
    ): Promise<FacadeSendTextResult | null> {
      const body = typeof content.body === 'string' ? content.body : '';
      const result = await invoke('matrix_send_text', {
        roomId,
        body,
        msgType: typeof content.msgtype === 'string' ? content.msgtype : undefined,
        formattedBody:
          typeof content.formatted_body === 'string' ? content.formatted_body : undefined,
        txnId: typeof content.txnid === 'string' ? content.txnid : undefined,
        ...(typeof content['m.mentions'] === 'object' ? { mentions: content['m.mentions'] } : {}),
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
        return this.sendMessage(roomId, content);
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
    setAccountData(type: string, content: unknown): Promise<unknown> {
      const hasContent = isObject(content) && Object.keys(content).length > 0;
      if (type.length === 0 || !hasContent) return Promise.resolve(null);
      return Promise.resolve(null); // GAP: no native account-data command
    },
    async setRoomAccountData(roomId: RoomId, type: string, content: unknown): Promise<unknown> {
      const hasContent = isObject(content) && Object.keys(content).length > 0;
      if (roomId.length === 0 || type.length === 0 || !hasContent) {
        return Promise.resolve(null);
      }
      return Promise.resolve(null); // GAP: no native account-data command
    },

    /** F6c — read-marker engine surface (ReceiptClientReading); native owns receipts. */
    async setRoomReadMarkers(
      roomId: string,
      fullyReadEventId: string,
      publicReceipt?: unknown,
      privateReceipt?: unknown
    ): Promise<unknown> {
      const x =
        roomId.length +
          (fullyReadEventId?.length ?? 0) +
          (publicReceipt === null ? 0 : 0) +
          (privateReceipt === null ? 0 : 0) >=
        0
          ? ''
          : '';
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      void x;
      return Promise.resolve(null);
    },
    async sendReadReceipt(event: unknown, receiptType?: string): Promise<unknown> {
      const unused = (event === null ? 1 : 0) + (receiptType?.length ?? 0) >= 0; // eslint-disable-line
      return Promise.resolve(null);
    },
    async getLatestTimeline(
      timelineSet: unknown
    ): Promise<{ getEvents(): MatrixEventReading[] } | null | undefined> {
      const unused = timelineSet === null ? 1 : 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },

    /** F4 — upload content bytes via matrix_upload_media (fail-closed null). */
    async uploadContent(input: unknown): Promise<FacadeUploadMediaResult> {
      let mimeType = '';
      let bytes: number[] = [];
      if (isObject(input)) {
        const rec = input as Record<string, unknown>;
        mimeType = typeof rec.mimeType === 'string' ? rec.mimeType : '';
        const rawBytes = rec.bytes;
        bytes = Array.isArray(rawBytes)
          ? rawBytes.filter((b): b is number => typeof b === 'number')
          : [];
      }
      const result = await invoke('matrix_upload_media', { mimeType, bytes });
      if (!result.available) {
        throw new Error('Native media upload is unavailable.');
      }
      const parsed = parseUploadMediaResult(result.value);
      if (!parsed) {
        throw new Error('Native media upload returned an invalid result.');
      }
      return parsed;
    },

    /** F4 — media upload size limit via matrix_media_config. */
    async getMediaConfig(): Promise<FacadeMediaConfig> {
      const result = await invoke('matrix_media_config');
      if (!result.available) return {};
      return parseMediaConfig(result.value);
    },

    /** F4 / P4-S36 — download via handle or leftover mxc. Handles stay opaque. */
    async downloadMedia(contentUri: string): Promise<FacadeMediaDownloadResult | null> {
      const result = await invoke('matrix_media_download', {
        contentUri: timelineMediaHandleFromUri(contentUri) ?? contentUri,
      });
      if (!result.available) return null;
      return parseMediaDownload(result.value);
    },

    /** F4 — profile/identity from the sync cache (no key material, D1C). */
    getProfileInfo(userId?: string): Promise<{ avatar_url?: string; displayname?: string }> {
      const x = userId?.length ?? 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve({
        avatar_url: cachedIdentity.avatarUrl,
        displayname: cachedIdentity.displayName,
      });
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
        encryptToDeviceMessages: async () => [],
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
    /** F6c — mutual-rooms GAP: native has no shared-rooms command; fail-closed []. */
    async _unstable_getSharedRooms(userId: string): Promise<string[]> {
      const x = userId.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve([]);
    },

    /** F6c — user/directory/pusher/imap GAP stubs (no native commands). */
    getUser(userId: string): { avatarUrl?: string; displayName?: string } | null {
      const x = userId.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return null;
    },
    async getThreePids(): Promise<{
      threepids: Array<{ medium: string; address: string }>;
    } | null> {
      return Promise.resolve(null);
    },
    async getPushers(): Promise<{ pushers: Array<{ app_id: string; pushkey: string }> } | null> {
      return Promise.resolve(null);
    },
    async setPusher(pusher: unknown): Promise<unknown> {
      const unused = pusher === null; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },
    async getLocalAliases(roomId: string): Promise<{ aliases: string[] }> {
      const x = roomId.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve({ aliases: [] });
    },
    async createAlias(alias: string, roomId: string): Promise<unknown> {
      const unused = alias.length + roomId.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },
    async deleteAlias(alias: string): Promise<unknown> {
      const unused = alias.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },
    async cancelUpload(_token: unknown): Promise<unknown> {
      const noNativeCancel = _token !== null; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(noNativeCancel ? null : null);
    },
    async _requestDeviceVerification(userId: string): Promise<unknown> {
      const unused = userId.length >= 0; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },
    async searchUserDirectory(opts: { term: string; limit?: number }): Promise<{
      limited: boolean;
      results: Array<{
        user_id: string;
        display_name?: string;
        avatar_url?: string;
      }>;
    }> {
      const x = opts.term.length + (opts.limit ?? 0); // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve({ limited: false, results: [] });
    },
    async searchUserDirectoryFn(term: string): Promise<unknown> {
      const x = term.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },

    /** F5 — GAP stubs required by the type anchor; no native surface. */
    async getCapabilities(): Promise<FacadeCapabilitiesIntersection> {
      return Promise.resolve({});
    },
    async getOpenIdToken(): Promise<{
      access_token: string;
      expires_in?: number;
      matrix_server_name?: string;
      token_type?: string;
    } | null> {
      return Promise.resolve(null);
    },
    async search(opts: { body?: unknown; next_batch?: string }): Promise<{
      search_categories: {
        room_events?: {
          next_batch?: string;
          highlights?: string[];
          results?: Array<{ rank: number; result: unknown; context: { [key: string]: unknown } }>;
        };
      };
      [key: string]: unknown;
    } | null> {
      const argLength =
        (typeof opts.body === 'object' && opts.body !== null ? 1 : 0) +
        (opts.next_batch?.length ?? 0);
      void argLength;
      return Promise.resolve(null);
    },
    /** F6a — aliases + ignored users + auth metadata + relations (GAP-safe). */
    async getRoomIdForAlias(alias: string): Promise<{ room_id?: string } | null> {
      const x = alias.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(null);
    },
    async setIgnoredUsers(userIds: string[]): Promise<void> {
      const ignore = userIds.length >= 0; // eslint-disable-line
      return Promise.resolve(ignore ? undefined : undefined);
    },
    async getAuthMetadata(): Promise<
      | {
          issuer?: string;
          account_management_uri?: string;
          homeserver_url?: string;
        }
      | undefined
    > {
      return Promise.resolve(undefined);
    },
    async relations(
      roomId: string,
      eventId: string,
      relationType?: string | null,
      eventType?: string | null,
      filter?: { from?: string; to?: string; limit?: number; dir?: string }
    ): Promise<{
      events: Array<{
        getContent<T = Record<string, unknown>>(): T;
        getSender(): string | null;
        getTs(): number;
        getEffectiveEvent?(): unknown;
      }>;
      nextBatch?: unknown;
      prevBatch?: unknown;
    }> {
      const argLength =
        roomId.length +
        eventId.length +
        (relationType?.length ?? 0) +
        (eventType?.length ?? 0) +
        (filter ? 1 : 0);
      void argLength;
      return Promise.resolve({ events: [] });
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

    /** F6c — device to-device queue (D1C: native crypto owns); fail-closed no-op. */
    async queueToDevice(batch: unknown[] | { eventType: string; batch: unknown[] }): Promise<void> {
      const x = Array.isArray(batch) ? batch.length : batch.batch.length + batch.eventType.length; // eslint-disable-line @typescript-eslint/no-unused-vars
      return Promise.resolve(undefined);
    },

    /** F6c — delayed-event APIs are not natively surfaced; GAP-safe stubs. */
    async _unstable_sendDelayedEvent(
      roomId: string,
      opts: unknown,
      txnId: string | null,
      eventType: string,
      content: Record<string, unknown>
    ): Promise<{ delay_id?: string } | null> {
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const x =
        roomId.length +
        (opts === null ? 1 : 0) +
        (txnId?.length ?? 0) +
        eventType.length +
        Object.keys(content).length;
      return Promise.resolve(null);
    },
    async _unstable_sendDelayedStateEvent(
      roomId: string,
      opts: unknown,
      eventType: string,
      content: Record<string, unknown>,
      stateKey?: string
    ): Promise<{ delay_id?: string } | null> {
      const argLength =
        roomId.length +
        (opts === null ? 1 : 0) +
        eventType.length +
        Object.keys(content).length +
        (stateKey?.length ?? 0);
      void argLength;
      return Promise.resolve(null);
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

    get http(): {
      authedRequest<T = unknown>(
        _method: string,
        _path: string,
        _queryParams?: Record<string, unknown>
      ): Promise<T>;
    } {
      // D1C/native: the renderer has no js-sdk HTTP transport; legacy authed
      // request paths (unrouted web inbox) resolve to an empty typed result.
      return {
        authedRequest: (async <T>() => {
          const empty: T = {} as T;
          return empty;
        }) as never,
      };
    },
    get store(): { accountData: Map<string, unknown> } {
      // D1C/native: the renderer has no js-sdk IndexedDB store; expose an empty
      // account-data map so dev-tools compile and render an empty list.
      return { accountData: new Map() };
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
