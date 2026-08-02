import type { DesktopInvokeResult } from '../../utils/desktop';
import { hasForbiddenWireFields, isObject } from '../matrix-dto/parseUtil';

const unavailableMessage = 'Native Matrix space hierarchy is unavailable.';
const MAX_ROOM_COUNT = 5_000;
const MAX_MEMBER_COUNT = 1_000_000_000;
const MAX_ROOM_ID_LENGTH = 512;
const MAX_ALIAS_LENGTH = 512;
const MAX_NAME_LENGTH = 4_096;
const MAX_TOPIC_LENGTH = 4_096;
const MAX_AVATAR_URI_LENGTH = 2_048;
const MAX_ROOM_TYPE_LENGTH = 64;

const SESSION_LOGGED_OUT_KEYS = ['status'] as const;
const SESSION_LOGGED_IN_KEYS = [
  'status',
  'user_id',
  'device_id',
  'homeserver_url',
  'sessionGeneration',
] as const;
const SNAPSHOT_KEYS = ['sessionGeneration', 'rooms'] as const;
const ROOM_KEYS = [
  'roomId',
  'name',
  'canonicalAlias',
  'topic',
  'avatarUrl',
  'roomType',
  'numJoinedMembers',
  'joinRule',
  'worldReadable',
  'guestCanJoin',
] as const;
const REQUIRED_ROOM_KEYS = [
  'roomId',
  'numJoinedMembers',
  'joinRule',
  'worldReadable',
  'guestCanJoin',
] as const;

const SUPPORTED_JOIN_RULES = new Set([
  'public',
  'knock',
  'invite',
  'private',
  'restricted',
  'knock_restricted',
]);

const isSafeGeneration = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const isSafeCount = (value: unknown, maximum: number): value is number =>
  typeof value === 'number' &&
  Number.isSafeInteger(value) &&
  value >= 0 &&
  value <= maximum;

const hasExactKeys = (
  value: Record<string, unknown>,
  keys: readonly string[],
  requiredKeys: readonly string[] = keys
): boolean => {
  const allowed = new Set(keys);
  return (
    Object.keys(value).every((key) => allowed.has(key)) &&
    requiredKeys.every((key) => key in value)
  );
};

const isMatrixIdentifier = (
  value: unknown,
  prefix: '!' | '#',
  maxLength: number
): value is string => {
  if (typeof value !== 'string' || value.length === 0 || value.length > maxLength) return false;
  if (!value.startsWith(prefix) || /\s/.test(value)) return false;
  const separator = value.indexOf(':', 1);
  return separator > 1 && separator < value.length - 1;
};

export const isNativeRoomId = (value: unknown): value is string =>
  isMatrixIdentifier(value, '!', MAX_ROOM_ID_LENGTH);

const isNativeRoomAlias = (value: string): boolean =>
  isMatrixIdentifier(value, '#', MAX_ALIAS_LENGTH);

const optionalBoundedString = (
  value: Record<string, unknown>,
  key: string,
  maxLength: number
): string | undefined => {
  if (!(key in value) || value[key] === null) return undefined;
  if (typeof value[key] !== 'string' || [...value[key]].length > maxLength) {
    throw new Error(unavailableMessage);
  }
  return value[key];
};

const optionalAvatarUri = (value: Record<string, unknown>): string | undefined => {
  const avatarUrl = optionalBoundedString(value, 'avatarUrl', MAX_AVATAR_URI_LENGTH);
  if (avatarUrl === undefined) return undefined;
  if (!/^mxc:\/\/[^/\s]+\/[^/\s]+$/.test(avatarUrl)) {
    throw new Error(unavailableMessage);
  }
  return avatarUrl;
};

const parseLoggedInSessionGeneration = (value: unknown): number => {
  if (!isObject(value) || hasForbiddenWireFields(value)) throw new Error(unavailableMessage);
  if (value.status === 'logged_out') {
    if (!hasExactKeys(value, SESSION_LOGGED_OUT_KEYS)) throw new Error(unavailableMessage);
    throw new Error('Native Matrix space hierarchy requires a logged-in session.');
  }
  if (
    value.status !== 'logged_in' ||
    !hasExactKeys(value, SESSION_LOGGED_IN_KEYS) ||
    !isSafeGeneration(value.sessionGeneration) ||
    typeof value.user_id !== 'string' ||
    value.user_id.length === 0 ||
    /\s/.test(value.user_id) ||
    typeof value.device_id !== 'string' ||
    value.device_id.length === 0 ||
    /\s/.test(value.device_id) ||
    typeof value.homeserver_url !== 'string' ||
    value.homeserver_url.length === 0 ||
    /\s/.test(value.homeserver_url)
  ) {
    throw new Error(unavailableMessage);
  }
  return value.sessionGeneration;
};

const optionalRoomType = (value: Record<string, unknown>): string | undefined => {
  const roomType = optionalBoundedString(value, 'roomType', MAX_ROOM_TYPE_LENGTH);
  if (roomType !== undefined && roomType !== 'm.space') throw new Error(unavailableMessage);
  return roomType;
};

const parseRoom = (value: unknown): NativeSpaceHierarchyRoom => {
  if (
    !isObject(value) ||
    hasForbiddenWireFields(value) ||
    !hasExactKeys(value, ROOM_KEYS, REQUIRED_ROOM_KEYS)
  ) {
    throw new Error(unavailableMessage);
  }
  if (!isNativeRoomId(value.roomId)) throw new Error(unavailableMessage);

  const name = optionalBoundedString(value, 'name', MAX_NAME_LENGTH);
  const canonicalAlias = optionalBoundedString(value, 'canonicalAlias', MAX_ALIAS_LENGTH);
  if (canonicalAlias !== undefined && !isNativeRoomAlias(canonicalAlias)) {
    throw new Error(unavailableMessage);
  }
  const topic = optionalBoundedString(value, 'topic', MAX_TOPIC_LENGTH);
  const avatarUrl = optionalAvatarUri(value);
  const roomType = optionalRoomType(value);

  if (
    !isSafeCount(value.numJoinedMembers, MAX_MEMBER_COUNT) ||
    typeof value.joinRule !== 'string' ||
    !SUPPORTED_JOIN_RULES.has(value.joinRule) ||
    typeof value.worldReadable !== 'boolean' ||
    typeof value.guestCanJoin !== 'boolean'
  ) {
    throw new Error(unavailableMessage);
  }

  return {
    roomId: value.roomId,
    name,
    canonicalAlias,
    topic,
    avatarUrl,
    roomType,
    numJoinedMembers: value.numJoinedMembers,
    joinRule: value.joinRule,
    worldReadable: value.worldReadable,
    guestCanJoin: value.guestCanJoin,
  };
};

const parseSnapshot = (
  value: unknown,
  sessionGeneration: number
): NativeSpaceHierarchySnapshot => {
  if (!isObject(value) || hasForbiddenWireFields(value) || !hasExactKeys(value, SNAPSHOT_KEYS)) {
    throw new Error(unavailableMessage);
  }
  if (!isSafeGeneration(value.sessionGeneration) || value.sessionGeneration !== sessionGeneration) {
    throw new Error(unavailableMessage);
  }
  if (!Array.isArray(value.rooms) || value.rooms.length > MAX_ROOM_COUNT) {
    throw new Error(unavailableMessage);
  }
  const rooms = value.rooms.map(parseRoom);
  const roomIds = new Set<string>();
  for (const room of rooms) {
    if (roomIds.has(room.roomId)) throw new Error(unavailableMessage);
    roomIds.add(room.roomId);
  }
  return { sessionGeneration, rooms };
};

export type NativeSpaceHierarchyRoom = {
  roomId: string;
  name?: string;
  canonicalAlias?: string;
  topic?: string;
  avatarUrl?: string;
  roomType?: string;
  numJoinedMembers: number;
  joinRule: string;
  worldReadable: boolean;
  guestCanJoin: boolean;
};

export type NativeSpaceHierarchySnapshot = {
  sessionGeneration: number;
  rooms: NativeSpaceHierarchyRoom[];
};

export type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

const unavailable = (): Error => new Error(unavailableMessage);

export async function readSpaceHierarchyWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeSpaceHierarchySnapshot> {
  if (!isNativeRoomId(roomId)) {
    throw new Error('Native Matrix space hierarchy requires a valid room ID.');
  }
  if (!desktopAvailable) throw unavailable();

  let session: DesktopInvokeResult<unknown>;
  try {
    session = await invoke('matrix_session_snapshot');
  } catch {
    throw unavailable();
  }
  if (!session.available || session.value === undefined) throw unavailable();
  const sessionGeneration = parseLoggedInSessionGeneration(session.value);

  let result: DesktopInvokeResult<unknown>;
  try {
    result = await invoke('matrix_space_hierarchy_snapshot', { roomId });
  } catch {
    throw unavailable();
  }
  if (!result.available || result.value == null) throw unavailable();
  return parseSnapshot(result.value, sessionGeneration);
}

export async function readSpaceHierarchyRoomWithNativeOwner(
  roomId: string,
  desktopAvailable: boolean,
  invoke: NativeInvoke
): Promise<NativeSpaceHierarchyRoom> {
  const snapshot = await readSpaceHierarchyWithNativeOwner(roomId, desktopAvailable, invoke);
  const room = snapshot.rooms.find((candidate) => candidate.roomId === roomId);
  if (!room) throw new Error(unavailableMessage);
  return room;
}
