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

export type NativeRoomProfileWriteResult = {
  status: 'ok';
};

export type NativeRoomDirectoryVisibility = 'public' | 'private';

export type NativeRoomDirectoryVisibilityResult = {
  status: 'ok';
  roomId: string;
  sessionGeneration: number;
  visibility: NativeRoomDirectoryVisibility;
};

export type NativeRoomDirectoryVisibilityWriteResult = {
  status: 'ok';
  roomId: string;
  sessionGeneration: number;
  requestedVisibility: NativeRoomDirectoryVisibility;
};

const directoryVisibilityUnavailableMessage =
  'Native Matrix room directory visibility is unavailable.';
const roomIdPattern = /^![^:\s]+:[^\s]+$/;

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && value.length <= 512 && roomIdPattern.test(value);

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
    throw new Error(directoryVisibilityUnavailableMessage);
  }
};

const requireDirectoryVisibilitySession = async (invoke: NativeInvoke): Promise<number> => {
  const result = await invokeSafely('matrix_session_snapshot', undefined, invoke);
  if (!result.available || !isObject(result.value) || hasForbiddenWireFields(result.value)) {
    throw new Error(directoryVisibilityUnavailableMessage);
  }

  const snapshot = result.value as NativeSessionSnapshot & Record<string, unknown>;
  if (snapshot.status === 'logged_out') {
    if (!hasExactKeys(snapshot, ['status'])) throw new Error(directoryVisibilityUnavailableMessage);
    throw new Error(directoryVisibilityUnavailableMessage);
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
    throw new Error(directoryVisibilityUnavailableMessage);
  }
  return snapshot.sessionGeneration;
};

const parseDirectoryVisibilityResult = (
  value: unknown,
  roomId: string,
  sessionGeneration: number
): NativeRoomDirectoryVisibilityResult => {
  if (
    !isObject(value) ||
    hasForbiddenWireFields(value) ||
    !hasExactKeys(value, ['status', 'roomId', 'sessionGeneration', 'visibility']) ||
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    (value.visibility !== 'public' && value.visibility !== 'private')
  ) {
    throw new Error(directoryVisibilityUnavailableMessage);
  }
  return value as NativeRoomDirectoryVisibilityResult;
};

const parseDirectoryVisibilityWriteResult = (
  value: unknown,
  roomId: string,
  sessionGeneration: number,
  requestedVisibility: NativeRoomDirectoryVisibility
): NativeRoomDirectoryVisibilityWriteResult => {
  if (
    !isObject(value) ||
    hasForbiddenWireFields(value) ||
    !hasExactKeys(value, ['status', 'roomId', 'sessionGeneration', 'requestedVisibility']) ||
    value.status !== 'ok' ||
    value.roomId !== roomId ||
    value.sessionGeneration !== sessionGeneration ||
    !isSafeGeneration(value.sessionGeneration) ||
    value.requestedVisibility !== requestedVisibility
  ) {
    throw new Error(directoryVisibilityUnavailableMessage);
  }
  return value as NativeRoomDirectoryVisibilityWriteResult;
};

export async function getRoomDirectoryVisibilityWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeRoomDirectoryVisibilityResult> {
  if (!desktopAvailable || !isRoomId(roomId)) {
    throw new Error(directoryVisibilityUnavailableMessage);
  }
  const sessionGeneration = await requireDirectoryVisibilitySession(invoke);
  const result = await invokeSafely(
    'matrix_get_room_directory_visibility',
    { roomId, sessionGeneration },
    invoke
  );
  if (!result.available) throw new Error(directoryVisibilityUnavailableMessage);
  return parseDirectoryVisibilityResult(result.value, roomId, sessionGeneration);
}

export async function setRoomDirectoryVisibilityWithNativeOwner(
  roomId: string,
  visibility: NativeRoomDirectoryVisibility,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeRoomDirectoryVisibilityWriteResult> {
  if (
    !desktopAvailable ||
    !isRoomId(roomId) ||
    (visibility !== 'public' && visibility !== 'private')
  ) {
    throw new Error(directoryVisibilityUnavailableMessage);
  }
  const sessionGeneration = await requireDirectoryVisibilitySession(invoke);
  const result = await invokeSafely(
    'matrix_set_room_directory_visibility',
    { roomId, sessionGeneration, visibility },
    invoke
  );
  if (!result.available) throw new Error(directoryVisibilityUnavailableMessage);
  return parseDirectoryVisibilityWriteResult(result.value, roomId, sessionGeneration, visibility);
}

/**
 * R-ROOM-PROFILE: sole room profile write owner when a native Matrix session
 * is live. Fail-closed — never falls through to mx.sendStateEvent for
 * m.room.name / m.room.topic / m.room.avatar.
 */
export async function isNativeRoomProfileWriteSession(
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<boolean> {
  if (!desktopAvailable) return false;
  const session = await invoke('matrix_session_snapshot');
  if (!session.available) return false;
  const snapshot = session.value as NativeSessionSnapshot | undefined;
  return snapshot?.status === 'logged_in';
}

async function assertOk(result: DesktopInvokeResult<unknown>, label: string): Promise<void> {
  if (!result.available) {
    throw new Error(`Native Matrix room ${label} is unavailable.`);
  }
  const body = result.value as NativeRoomProfileWriteResult | undefined;
  if (body?.status !== 'ok') {
    throw new Error(`Native Matrix room ${label} is unavailable.`);
  }
}

export async function setRoomNameWithNativeOwner(
  roomId: string,
  name: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeRoomProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_room_name', { roomId, name });
  await assertOk(result, 'name update');
  return 'native';
}

export async function setRoomTopicWithNativeOwner(
  roomId: string,
  topic: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeRoomProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_room_topic', { roomId, topic });
  await assertOk(result, 'topic update');
  return 'native';
}

export async function setRoomAvatarWithNativeOwner(
  roomId: string,
  mxc: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<'native' | 'legacy'> {
  if (!(await isNativeRoomProfileWriteSession(desktopAvailable, invoke))) {
    return 'legacy';
  }
  const result = await invoke('matrix_set_room_avatar', { roomId, mxc });
  await assertOk(result, 'avatar update');
  return 'native';
}
