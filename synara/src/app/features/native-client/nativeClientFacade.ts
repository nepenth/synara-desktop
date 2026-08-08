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

const toFacadeRoomReading = (summary: RoomSummary): FacadeRoomReading => ({
  roomId: summary.roomId,
  name: summary.name ?? '',
  canonicalAlias: summary.canonicalAlias ?? null,
  avatarUrl: summary.avatarUrl ?? null,
  membership: summary.membership,
  isDirect: summary.isDirect,
  isSpace: summary.isSpace,
  isEncrypted: summary.isEncrypted,
  unreadCount: summary.unreadCount,
  highlightCount: summary.highlightCount,
  lastActivityTs: summary.lastActivityTs,
  tombstoneSuccessorRoomId: summary.tombstoneSuccessorRoomId ?? null,
  getMyMembership: () => summary.membership,
  getCanonicalAlias: () => summary.canonicalAlias ?? null,
  getJoinedMemberCount: () => summary.heroes?.length ?? 0,
  isSpaceRoom: () => summary.isSpace,
  isCallRoom: () => false,
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
  let cachedSyncState: NativeSyncState | null = null;
  let cachedSyncData: NativeSyncStateData | null = null;

  const readSyncStatus = async (): Promise<NativeSyncStatus | null> => {
    const result = await invoke('matrix_sync_status');
    if (!result.available) return null;
    return parseSyncStatus(result.value);
  };

  const emitSyncState = (state: NativeSyncState, data: NativeSyncStateData | null): void => {
    cachedSyncState = state;
    cachedSyncData = data;
    emitter.emit('sync', state);
  };

  return Object.freeze({
    emitter,
    on: emitter.on.bind(emitter),
    once: emitter.once.bind(emitter),
    off: emitter.off.bind(emitter),
    removeListener: emitter.removeListener.bind(emitter),
    emit: emitter.emit.bind(emitter),
    setMaxListeners: emitter.setMaxListeners.bind(emitter),

    /** js-sdk-compatible lifecycle. readState=false: cached value only. */
    async getSyncState(readState?: boolean): Promise<NativeSyncState | null> {
      if (readState !== false || cachedSyncState === null) {
        const status = await readSyncStatus();
        if (status) {
          emitSyncState(readinessToSyncState(status.readiness), {
            readiness: status.readiness,
            sessionGeneration: status.sessionGeneration,
            failureDiagnosticId: status.failureDiagnosticId,
          });
        }
      }
      return cachedSyncState;
    },

    async getSyncStateData(): Promise<NativeSyncStateData | null> {
      if (cachedSyncData === null) {
        const status = await readSyncStatus();
        if (status) {
          emitSyncState(readinessToSyncState(status.readiness), {
            readiness: status.readiness,
            sessionGeneration: status.sessionGeneration,
            failureDiagnosticId: status.failureDiagnosticId,
          });
        }
      }
      return cachedSyncData;
    },

    async clientRunning(): Promise<boolean> {
      const status = await readSyncStatus();
      return status?.readiness === 'running';
    },

    async retryImmediately(): Promise<void> {
      const status = await readSyncStatus();
      if (!status) throw new Error(UNAVAILABLE_MESSAGE);
      emitSyncState(readinessToSyncState(status.readiness), {
        readiness: status.readiness,
        sessionGeneration: status.sessionGeneration,
        failureDiagnosticId: status.failureDiagnosticId,
      });
    },

    /** Native sync runs in Rust; startClient is a readiness confirmation. */
    async startClient(): Promise<void> {
      await this.getSyncState();
    },

    /** Session teardown path: retain the facade, drop readiness. */
    async stopClient(): Promise<void> {
      cachedSyncState = null;
      cachedSyncData = null;
      emitter.emit('sync', 'STOPPED');
    },

    async logout(): Promise<void> {
      const result = await invoke('matrix_logout');
      if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
      cachedSyncState = null;
      cachedSyncData = null;
      emitter.emit('sync', 'STOPPED');
    },

    /** D1C identity: user/device/homeserver from the native session snapshot. */
    async getIdentity(): Promise<NativeClientIdentity> {
      const result = await invoke('matrix_session_snapshot');
      if (!result.available) return {};
      return parseSessionSnapshot(result.value) as NativeClientIdentity;
    },

    async getUserId(): Promise<string | undefined> {
      return (await this.getIdentity()).userId;
    },

    async getSafeUserId(): Promise<string> {
      return (await this.getIdentity()).userId ?? '';
    },

    async getDeviceId(): Promise<string | undefined> {
      return (await this.getIdentity()).deviceId;
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
            emitSyncState(next, {
              readiness: status.readiness,
              sessionGeneration: status.sessionGeneration,
              failureDiagnosticId: status.failureDiagnosticId,
            });
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

    /** F2 — all joined rooms from matrix_room_list_snapshot (fail-closed []). */
    async getRooms(): Promise<FacadeRoomReading[]> {
      const result = await invoke('matrix_room_list_snapshot');
      if (!result.available) return [];
      const snapshot = parseRoomListSnapshot(result.value);
      return snapshot ? snapshot.rooms.map(toFacadeRoomReading) : [];
    },

    /** F2 — single room by id from the native room-list projection. */
    async getRoom(roomId: RoomId): Promise<FacadeRoomReading | null> {
      const rooms = await this.getRooms();
      const found = rooms.find((r) => r.roomId === roomId);
      return found ?? null;
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
      content: FacadeSendStateEventContent
    ): Promise<FacadeSendStateEventResult | null> {
      const command =
        type === 'm.room.name'
          ? 'matrix_set_room_name'
          : type === 'm.room.topic'
          ? 'matrix_set_room_topic'
          : type === 'm.room.avatar'
          ? 'matrix_set_room_avatar'
          : null;
      if (!command) return null; // GAP: no generic state-event command
      const result = await invoke(command, { roomId, name: content.name, avatarUrl: content.url });
      if (!result.available) return null;
      return isObject(result.value) ? (result.value as FacadeSendStateEventResult) : null;
    },

    /** F3 — account-data is a documented GAP (no native command yet); fail-closed null. */
    async getAccountData(type: string): Promise<unknown> {
      const noNativeAccountDataCommand = type.length > 0; // eslint-disable-line no-unused-vars
      return Promise.resolve(noNativeAccountDataCommand ? null : null);
    },
    async setAccountData(type: string, content: Record<string, unknown>): Promise<unknown> {
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

    /** F4 — profile/identity from session snapshot + session-level hints. */
    async getProfileInfo(): Promise<FacadeProfileInfo> {
      const identity = await this.getIdentity();
      const sessionId = await this.getDeviceId();
      return { userId: identity.userId, deviceId: sessionId };
    },

    // D1C guard: fail-closed readiness helper for callers that must not run
    // while the native session is offline/failed.
    isReady(): boolean {
      return !FORBIDDEN_READINESS.has(cachedSyncData?.readiness ?? 'unconfigured');
    },
  });
};

export type NativeMatrixClient = ReturnType<typeof createNativeMatrixClient>;
