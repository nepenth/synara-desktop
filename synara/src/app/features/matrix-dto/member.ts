/**
 * Room member DTO.
 */

import type { RoomId, UserId } from './ids';
import { isMembership, type Membership } from './room';
import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqNumber,
  reqString,
} from './parseUtil';

export type RoomMember = {
  roomId: RoomId;
  userId: UserId;
  displayName?: string;
  avatarUrl?: string;
  membership: Membership;
  powerLevel: number;
  isDirectTarget?: boolean;
};

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
