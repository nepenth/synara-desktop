/**
 * Room member DTO.
 */

import { isMembership } from './room';
import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqNumber,
  reqString,
} from './parseUtil';
import type { RoomMember as WireRoomMember } from './generated';
import type { NullsToOptional } from './wireTypes';

/** Parsed form of Core's `RoomMember`: absent instead of `null`. */
export type RoomMember = NullsToOptional<WireRoomMember>;

export function parseRoomMember(value: unknown): RoomMember | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const userId = reqString(value, 'userId');
  const displayName = optString(value, 'displayName');
  const avatarUrl = optString(value, 'avatarUrl');
  const powerLevel = reqNumber(value, 'powerLevel');
  const isDirectTarget = optBoolean(value, 'isDirectTarget');
  if (
    roomId === null ||
    userId === null ||
    displayName === null ||
    avatarUrl === null ||
    powerLevel === null ||
    isDirectTarget === null ||
    !isMembership(value.membership)
  ) {
    return null;
  }
  return {
    roomId,
    userId,
    displayName,
    avatarUrl,
    membership: value.membership,
    powerLevel,
    isDirectTarget,
  };
}
