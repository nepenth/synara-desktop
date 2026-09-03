/**
 * Room summary DTO — list/nav projection.
 */

import type { RoomId, UserId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optNumber,
  optString,
  optBoolean,
  reqBoolean,
  reqNumber,
  reqString,
} from './parseUtil';

export const MEMBERSHIPS = ['invite', 'join', 'knock', 'leave', 'ban'] as const;
export type Membership = typeof MEMBERSHIPS[number];
const MEMBERSHIP_SET = new Set<string>(MEMBERSHIPS);

export function isMembership(value: unknown): value is Membership {
  return typeof value === 'string' && MEMBERSHIP_SET.has(value);
}

export const NOTIFICATION_MODES = ['all', 'mentions', 'mute', 'default'] as const;
export type NotificationMode = typeof NOTIFICATION_MODES[number];
const NOTIFICATION_MODE_SET = new Set<string>(NOTIFICATION_MODES);

export function isNotificationMode(value: unknown): value is NotificationMode {
  return typeof value === 'string' && NOTIFICATION_MODE_SET.has(value);
}

export const ROOM_ENCRYPTION_STATUSES = ['encrypted', 'not_encrypted', 'unknown'] as const;
export type RoomEncryptionStatus = typeof ROOM_ENCRYPTION_STATUSES[number];
const ROOM_ENCRYPTION_STATUS_SET = new Set<string>(ROOM_ENCRYPTION_STATUSES);

export function isRoomEncryptionStatus(value: unknown): value is RoomEncryptionStatus {
  return typeof value === 'string' && ROOM_ENCRYPTION_STATUS_SET.has(value);
}

export type RoomHero = {
  userId: UserId;
  displayName?: string;
};

export type RoomSummary = {
  roomId: RoomId;
  name?: string;
  canonicalAlias?: string;
  avatarUrl?: string;
  membership: Membership;
  isDirect: boolean;
  isSpace: boolean;
  isCall: boolean;
  isFavorite: boolean;
  isEncrypted: boolean;
  /** Authoritative Core projection. Security decisions must not use `isEncrypted`. */
  encryptionStatus: RoomEncryptionStatus;
  joinRule?: string;
  unreadCount: number;
  highlightCount: number;
  markedUnread: boolean;
  notificationMode?: NotificationMode;
  lastActivityTs?: number;
  lastMessagePreview?: string;
  heroes?: RoomHero[];
  tombstoneSuccessorRoomId?: string;
};

function parseHero(value: unknown): RoomHero | null {
  if (!isObject(value)) return null;
  const userId = reqString(value, 'userId');
  const displayName = optString(value, 'displayName');
  if (userId === null || displayName === null) return null;
  return { userId, displayName };
}

export function parseRoomSummary(value: unknown): RoomSummary | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const name = optString(value, 'name');
  const canonicalAlias = optString(value, 'canonicalAlias');
  const avatarUrl = optString(value, 'avatarUrl');
  const isDirect = reqBoolean(value, 'isDirect');
  const isSpace = optBoolean(value, 'isSpace');
  const isCall = optBoolean(value, 'isCall') ?? false;
  const isFavorite = optBoolean(value, 'isFavorite') ?? false;
  const isEncrypted = reqBoolean(value, 'isEncrypted');
  const encryptionStatus = value.encryptionStatus;
  const joinRule = optString(value, 'joinRule');
  const unreadCount = reqNumber(value, 'unreadCount');
  const highlightCount = reqNumber(value, 'highlightCount');
  const markedUnread = reqBoolean(value, 'markedUnread');
  const lastActivityTs = optNumber(value, 'lastActivityTs');
  const lastMessagePreview = optString(value, 'lastMessagePreview');
  const tombstoneSuccessorRoomId = optString(value, 'tombstoneSuccessorRoomId');
  if (
    roomId === null ||
    name === null ||
    canonicalAlias === null ||
    avatarUrl === null ||
    isDirect === null ||
    isSpace === null ||
    isEncrypted === null ||
    !isRoomEncryptionStatus(encryptionStatus) ||
    joinRule === null ||
    unreadCount === null ||
    highlightCount === null ||
    markedUnread === null ||
    lastActivityTs === null ||
    lastMessagePreview === null ||
    tombstoneSuccessorRoomId === null ||
    !isMembership(value.membership)
  ) {
    return null;
  }
  // Keep the legacy display boolean internally consistent, but never infer the
  // authoritative tri-state from it. Unknown deliberately remains fail-closed.
  if (isEncrypted !== (encryptionStatus === 'encrypted')) return null;

  let notificationMode: NotificationMode | undefined;
  if (value.notificationMode !== undefined) {
    if (!isNotificationMode(value.notificationMode)) return null;
    notificationMode = value.notificationMode;
  }

  let heroes: RoomHero[] | undefined;
  if (value.heroes !== undefined) {
    if (!Array.isArray(value.heroes)) return null;
    heroes = [];
    for (const h of value.heroes) {
      const parsed = parseHero(h);
      if (!parsed) return null;
      heroes.push(parsed);
    }
  }

  return {
    roomId,
    name,
    canonicalAlias,
    avatarUrl,
    membership: value.membership,
    isDirect,
    isSpace: isSpace ?? false,
    isCall: isCall ?? false,
    isFavorite: isFavorite ?? false,
    isEncrypted,
    encryptionStatus,
    joinRule,
    unreadCount,
    highlightCount,
    markedUnread,
    notificationMode,
    lastActivityTs,
    lastMessagePreview,
    heroes,
    tombstoneSuccessorRoomId,
  };
}
