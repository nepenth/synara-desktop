import type { DesktopInvokeResult } from '../../../utils/desktop';
import { hasForbiddenWireFields, isObject } from '../../matrix-dto/parseUtil';

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
  sessionGeneration?: number;
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

export type NativeRoomRetentionSnapshot = {
  status: 'ok';
  roomId: string;
  sessionGeneration: number;
  advertised: boolean;
  maxLifetimeMs?: number;
  minLifetimeMs?: number;
  summary: string;
  distinction: string;
  mediaCacheSummary: string;
};

const unavailableMessage = 'Native Matrix room retention is unavailable.';
const roomIdPattern = /^![^:\s]+:[^\s]+$/;

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 512 && roomIdPattern.test(value);

const isSafeCount = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const hasExactKeys = (
  value: Record<string, unknown>,
  keys: readonly string[],
  requiredKeys: readonly string[] = keys
): boolean => {
  const allowed = new Set(keys);
  return (
    Object.keys(value).every((key) => allowed.has(key)) && requiredKeys.every((key) => key in value)
  );
};

const invokeSafely = async (
  command: string,
  args: Record<string, unknown> | undefined,
  invoke: NativeInvoke
): Promise<DesktopInvokeResult<unknown>> => {
  try {
    return await invoke(command, args);
  } catch {
    throw new Error(unavailableMessage);
  }
};

const requireRetentionSession = async (invoke: NativeInvoke): Promise<number> => {
  const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
  if (!result.available || !isObject(result.value) || hasForbiddenWireFields(result.value)) {
    throw new Error(unavailableMessage);
  }

  const snapshot = result.value as NativeSessionSnapshot & Record<string, unknown>;
  if (snapshot.status === 'logged_out') {
    if (!hasExactKeys(snapshot, ['status'])) throw new Error(unavailableMessage);
    throw new Error(unavailableMessage);
  }
  if (
    snapshot.status !== 'logged_in' ||
    !hasExactKeys(snapshot, [
      'status',
      'user_id',
      'device_id',
      'homeserver_url',
      'sessionGeneration',
    ]) ||
    typeof snapshot.user_id !== 'string' ||
    snapshot.user_id.length === 0 ||
    /\s/.test(snapshot.user_id) ||
    typeof snapshot.device_id !== 'string' ||
    snapshot.device_id.length === 0 ||
    /\s/.test(snapshot.device_id) ||
    typeof snapshot.homeserver_url !== 'string' ||
    snapshot.homeserver_url.length === 0 ||
    /\s/.test(snapshot.homeserver_url) ||
    !isSafeGeneration(snapshot.sessionGeneration)
  ) {
    throw new Error(unavailableMessage);
  }
  return snapshot.sessionGeneration;
};

export const parseRoomRetentionSnapshot = (
  value: unknown,
  roomId: string,
  sessionGeneration: number
): NativeRoomRetentionSnapshot => {
  if (!isObject(value) || hasForbiddenWireFields(value)) {
    throw new Error(unavailableMessage);
  }
  const required = [
    'status',
    'roomId',
    'sessionGeneration',
    'advertised',
    'summary',
    'distinction',
    'mediaCacheSummary',
  ] as const;
  const optional = ['maxLifetimeMs', 'minLifetimeMs'] as const;
  if (
    !hasExactKeys(value, [...required, ...optional], required) ||
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    typeof value.advertised !== 'boolean' ||
    typeof value.summary !== 'string' ||
    value.summary.length === 0 ||
    typeof value.distinction !== 'string' ||
    value.distinction.length === 0 ||
    typeof value.mediaCacheSummary !== 'string' ||
    value.mediaCacheSummary.length === 0
  ) {
    throw new Error(unavailableMessage);
  }
  if ('maxLifetimeMs' in value && !isSafeCount(value.maxLifetimeMs)) {
    throw new Error(unavailableMessage);
  }
  if ('minLifetimeMs' in value && !isSafeCount(value.minLifetimeMs)) {
    throw new Error(unavailableMessage);
  }
  const summary = value.summary.toLowerCase();
  if (summary.includes('forever') || summary.includes('history visibility')) {
    throw new Error(unavailableMessage);
  }
  return value as NativeRoomRetentionSnapshot;
};

export async function getRoomRetentionWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeRoomRetentionSnapshot> {
  if (!desktopAvailable || !isRoomId(roomId)) {
    throw new Error(unavailableMessage);
  }
  const sessionGeneration = await requireRetentionSession(invoke);
  const result = await invokeSafely(
    'matrix_room_retention',
    { roomId, sessionGeneration },
    invoke
  );
  if (!result.available) throw new Error(unavailableMessage);
  return parseRoomRetentionSnapshot(result.value, roomId, sessionGeneration);
}
