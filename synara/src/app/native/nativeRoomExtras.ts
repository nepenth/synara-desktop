import { isObject } from '../features/matrix-dto/parseUtil';
import type {
  NativeBulkRedactResult,
  NativeMutualRooms,
  NativeRoomAliasAvailability,
  NativeRoomAliasCheck,
  NativeRoomLocalAliases,
  NativeRoomUpgradeResult,
} from '../features/matrix-dto/generated';
import { invokeDesktopWithAvailability } from '../utils/desktop';
import { UNAVAILABLE_MESSAGE, type NativeInvoke } from './nativeWire';

/**
 * Room aliases, mutual rooms, room upgrade and bulk redaction through Core.
 * Every reply is checked before use; a malformed reply is an error.
 */

let invoke: NativeInvoke = (command, args) => invokeDesktopWithAvailability(command, args);

/** Route commands through a fake invoke (tests). */
export const setNativeRoomExtrasInvokeForTests = (next: NativeInvoke): void => {
  invoke = next;
};

const call = async (command: string, args: Record<string, unknown>): Promise<unknown> => {
  const result = await invoke(command, args);
  if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
  return result.value;
};

const isStringArray = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every((item) => typeof item === 'string');

export const parseMutualRooms = (value: unknown): string[] | undefined => {
  if (!isObject(value)) return undefined;
  const { roomIds } = value as Partial<NativeMutualRooms>;
  return isStringArray(roomIds) ? roomIds : undefined;
};

export const parseLocalAliases = (value: unknown): string[] | undefined => {
  if (!isObject(value)) return undefined;
  const { aliases } = value as Partial<NativeRoomLocalAliases>;
  return isStringArray(aliases) ? aliases : undefined;
};

export const parseAliasAvailability = (value: unknown): NativeRoomAliasAvailability | undefined => {
  if (!isObject(value)) return undefined;
  const { availability } = value as Partial<NativeRoomAliasCheck>;
  return availability === 'available' || availability === 'taken' ? availability : undefined;
};

export const parseRoomUpgrade = (value: unknown): string | undefined => {
  if (!isObject(value)) return undefined;
  const { replacementRoomId } = value as Partial<NativeRoomUpgradeResult>;
  return typeof replacementRoomId === 'string' && replacementRoomId.startsWith('!')
    ? replacementRoomId
    : undefined;
};

export const parseBulkRedact = (value: unknown): NativeBulkRedactResult | undefined => {
  if (!isObject(value)) return undefined;
  const reply = value as Partial<NativeBulkRedactResult>;
  const counts = [reply.scanned, reply.redacted, reply.failed];
  if (typeof reply.roomId !== 'string' || typeof reply.truncated !== 'boolean') return undefined;
  if (!counts.every((count) => typeof count === 'number' && count >= 0)) return undefined;
  return reply as NativeBulkRedactResult;
};

const required = <T>(parsed: T | undefined, what: string): T => {
  if (parsed === undefined) throw new Error(`Invalid ${what} response.`);
  return parsed;
};

/** Joined rooms where `userId` is a known joined member. */
export const fetchMutualRooms = async (userId: string): Promise<string[]> =>
  required(parseMutualRooms(await call('matrix_user_mutual_rooms', { userId })), 'mutual rooms');

/** The room's aliases on this homeserver. */
export const fetchLocalAliases = async (roomId: string): Promise<string[]> =>
  required(parseLocalAliases(await call('matrix_room_local_aliases', { roomId })), 'room aliases');

export const createLocalAlias = async (alias: string, roomId: string): Promise<void> => {
  await call('matrix_room_alias_create', { alias, roomId });
};

export const deleteLocalAlias = async (alias: string): Promise<void> => {
  await call('matrix_room_alias_delete', { alias });
};

/** `available` only when the homeserver reports the alias as unknown. */
export const checkAliasAvailability = async (alias: string): Promise<NativeRoomAliasAvailability> =>
  required(parseAliasAvailability(await call('matrix_room_alias_check', { alias })), 'alias check');

/** Upgrade a room and return the replacement room id. */
export const upgradeRoom = async (
  roomId: string,
  newVersion: string,
  additionalCreators?: string[]
): Promise<string> =>
  required(
    parseRoomUpgrade(
      await call('matrix_room_upgrade', {
        roomId,
        newVersion,
        additionalCreators: additionalCreators?.length ? additionalCreators : undefined,
      })
    ),
    'room upgrade'
  );

export type BulkRedactRequest = {
  roomId: string;
  userIds: string[];
  sinceTs: number;
  eventTypes?: string[];
  reason?: string;
};

/** Redact the listed users' events since `sinceTs` (bounded by Core). */
export const bulkRedact = async (request: BulkRedactRequest): Promise<NativeBulkRedactResult> =>
  required(
    parseBulkRedact(
      await call('matrix_room_bulk_redact', {
        roomId: request.roomId,
        userIds: request.userIds,
        sinceTs: Math.max(0, Math.floor(request.sinceTs)),
        eventTypes: request.eventTypes,
        reason: request.reason,
      })
    ),
    'bulk redaction'
  );
