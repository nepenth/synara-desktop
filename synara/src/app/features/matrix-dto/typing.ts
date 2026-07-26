/**
 * Typing notification DTO.
 */

import type { RoomId, UserId } from './ids';
import { hasForbiddenWireFields, isObject, reqString, stringArray } from './parseUtil';

export type TypingSnapshot = {
  roomId: RoomId;
  userIds: UserId[];
};

export function parseTypingSnapshot(value: unknown): TypingSnapshot | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const userIds = stringArray(value.userIds);
  if (roomId === null || userIds === null) return null;
  return { roomId, userIds };
}
