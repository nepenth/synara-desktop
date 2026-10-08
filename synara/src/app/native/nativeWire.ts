import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqBoolean,
  reqNumber,
  reqString,
} from '../features/matrix-dto/parseUtil';
import type {
  CommandGate,
  SyncReadiness,
  SyncReadinessSnapshot,
} from '../features/matrix-dto/generated';
import type { EventId, RoomId, UserId } from '../features/matrix-dto/ids';
import { parseRoomSummary, type RoomSummary } from '../features/matrix-dto/room';
import type { DesktopInvokeResult } from '../utils/desktop';

/**
 * Wire shapes and fail-closed parsers for the native session, sync and command
 * results the renderer reads. Every parser rejects payloads carrying
 * forbidden wire fields (tokens, raw SDK text) and returns `null`.
 */

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeReadiness = SyncReadiness;
export type NativeCommandGate = CommandGate;
export type NativeSyncStatus = SyncReadinessSnapshot;

const READINESSES = [
  'unconfigured',
  'idle',
  'running',
  'offline',
  'failed',
  'terminated',
] as const satisfies readonly NativeReadiness[];
type MissingReadiness = Exclude<NativeReadiness, (typeof READINESSES)[number]>;
const readinessListIsExhaustive: MissingReadiness extends never ? true : never = true;
void readinessListIsExhaustive;

/** Connection states the banner and lifecycle hooks read. */
export type NativeSyncState =
  'PREPARED' | 'SYNCING' | 'RECONNECTING' | 'ERROR' | 'STOPPED' | 'CATCHUP';

export type NativeSyncStateData = {
  readiness: NativeReadiness;
  sessionGeneration: number;
  failureDiagnosticId?: string | null;
  slidingSyncCapable?: boolean | null;
};

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
  displayName?: string;
  avatarUrl?: string;
};

export type NativeInvokeResult = {
  status: 'ok';
};

export const UNAVAILABLE_MESSAGE = 'Native Matrix client is unavailable.';

const parseCommandGate = (value: unknown): NativeCommandGate => {
  if (value === undefined || value === null || value === 'open') return 'open';
  return 'closed';
};

/** Map Rust sync readiness onto the connection states the UI reads. */
export const readinessToSyncState = (
  readiness: NativeReadiness,
  commandGate: NativeCommandGate = 'open'
): NativeSyncState => {
  // Mirrors iOS `ConnectionStatusCopy.fromReadiness`; the shared case table
  // lives in syncStatusCopy.test.ts and ConnectionStatusCopyTests.swift.
  if (commandGate !== 'open') return 'ERROR';
  switch (readiness) {
    case 'running':
      return 'PREPARED';
    case 'offline':
      return 'RECONNECTING';
    case 'failed':
    case 'terminated':
      // A terminated SyncService is not syncing; it restarts only through
      // recovery. Show the loss rather than a blank banner.
      return 'ERROR';
    case 'unconfigured':
    case 'idle':
      // Lost only once this mount had connected (see isSignedInSessionForBanner).
      return 'STOPPED';
    default:
      return 'STOPPED';
  }
};

export const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

export const parseSyncStatus = (value: unknown): NativeSyncStatus | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const readiness = optString(value, 'readiness');
  const sessionGeneration = reqNumber(value, 'sessionGeneration');
  const offlineModeEnabled = optBoolean(value, 'offlineModeEnabled');
  if (
    typeof readiness !== 'string' ||
    !(READINESSES as readonly string[]).includes(readiness) ||
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
    commandGate: parseCommandGate(
      Object.prototype.hasOwnProperty.call(value, 'commandGate') ? value.commandGate : undefined
    ),
  };
};

export const parseSessionSnapshot = (value: unknown): NativeSessionSnapshot | null => {
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

export const parseRoomListSnapshot = (value: unknown): NativeRoomListSnapshot | null => {
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

/** One event read back through `matrix_timeline_event_readback`. */
export type NativeTimelineEventReading = {
  eventId: EventId;
  sender: UserId;
  type: string;
  body: string;
  originServerTs: number;
};

export const parseTimelineEventReadback = (value: unknown): NativeTimelineEventReading | null => {
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

export type NativeSendTextResult = {
  roomId: RoomId;
  eventId: EventId;
  /** Send-result alias (`event_id`) kept for existing consumers. */
  event_id?: string;
  localTxnId: string;
  status: 'sent';
};

export const parseSendTextResult = (value: unknown): NativeSendTextResult | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const eventId = reqString(value, 'eventId');
  const localTxnId = reqString(value, 'localTxnId');
  if (roomId === null || eventId === null || localTxnId === null) return null;
  return { roomId, eventId, localTxnId, status: 'sent' };
};

export type NativeSendStateEventResult = {
  status: string;
  roomId?: RoomId;
  event_id?: string;
};

export type NativeUploadMediaResult = {
  mxc: string;
  /** Upload response alias (`content_uri` = mxc) for media consumers. */
  content_uri: string;
};

export const parseUploadMediaResult = (value: unknown): NativeUploadMediaResult | null => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const mxc = reqString(value, 'mxc');
  return mxc === null ? null : { mxc, content_uri: mxc };
};

export type NativeMediaConfig = {
  /** Wire key is `m.upload.size` (Rust MatrixCallMediaConfigResult). */
  maxUploadSizeBytes?: number;
};

export const parseMediaConfig = (value: unknown): NativeMediaConfig => {
  if (!isObject(value) || hasForbiddenWireFields(value)) return {};
  const maybeSize = (value as Record<string, unknown>)['m.upload.size'];
  return typeof maybeSize === 'number' && Number.isSafeInteger(maybeSize) && maybeSize > 0
    ? { maxUploadSizeBytes: maybeSize }
    : {};
};

export type NativeCrossSigningState = 'Unavailable' | 'NotSetUp' | 'Partial' | 'Ready';

/** Structural mirror of the Rust MatrixCryptoStatus DTO. */
export type NativeCryptoStatus = {
  sessionGeneration: number;
  encryptionEnabled: boolean;
  crossSigningState: NativeCrossSigningState;
};

export const parseCryptoStatus = (value: unknown): NativeCryptoStatus | null => {
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
    crossSigningState: crossSigningStateRaw as NativeCrossSigningState,
  };
};

export type NativeUserDirectoryResult = {
  limited: boolean;
  results: Array<{ user_id: string; display_name?: string; avatar_url?: string }>;
};

const isMxcAvatar = (value: string): boolean =>
  value.startsWith('mxc://') && value.split('/').length >= 4;

export const parseUserDirectorySearch = (value: unknown): NativeUserDirectoryResult => {
  if (!isObject(value) || hasForbiddenWireFields(value)) {
    return { limited: false, results: [] };
  }
  const limited = reqBoolean(value, 'limited') === true;
  const rawResults = value.results;
  if (!Array.isArray(rawResults)) {
    return { limited, results: [] };
  }
  const results: NativeUserDirectoryResult['results'] = [];
  for (const raw of rawResults) {
    if (!isObject(raw) || hasForbiddenWireFields(raw)) continue;
    const userId = reqString(raw, 'userId');
    if (!userId || !userId.startsWith('@') || !userId.includes(':')) continue;
    const displayName = optString(raw, 'displayName') ?? undefined;
    const avatarUrl = optString(raw, 'avatarUrl') ?? undefined;
    results.push({
      user_id: userId,
      display_name: displayName || undefined,
      avatar_url: avatarUrl && isMxcAvatar(avatarUrl) ? avatarUrl : undefined,
    });
  }
  return { limited, results };
};

export const parseOwnProfile = (
  value: unknown
): { displayName?: string; avatarUrl?: string } | null => {
  if (!isObject(value)) return null;
  const displayName = optString(value, 'displayName') ?? undefined;
  const avatarUrl = optString(value, 'avatarUrl') ?? undefined;
  return {
    displayName,
    avatarUrl:
      avatarUrl && avatarUrl.startsWith('mxc://') && avatarUrl.split('/').length >= 4
        ? avatarUrl
        : undefined,
  };
};
