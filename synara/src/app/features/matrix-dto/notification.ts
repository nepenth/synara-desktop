/**
 * OS notification candidate DTO — privacy-filtered title/body only.
 */

import type { EventId, NotificationCandidateId, RoomId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optString,
  reqBoolean,
  reqString,
} from './parseUtil';

export const NOTIFICATION_KINDS = [
  'message',
  'invite',
  'agent_approval',
  'later_reminder',
] as const;
export type NotificationKind = (typeof NOTIFICATION_KINDS)[number];
const KIND_SET = new Set<string>(NOTIFICATION_KINDS);

export function isNotificationKind(value: unknown): value is NotificationKind {
  return typeof value === 'string' && KIND_SET.has(value);
}

export type NotificationCandidate = {
  candidateId: NotificationCandidateId;
  roomId: RoomId;
  eventId?: EventId;
  kind: NotificationKind;
  title: string;
  body: string;
  route?: string;
  suppressIfFocusedRoom: boolean;
  isEncrypted: boolean;
};

export function parseNotificationCandidate(
  value: unknown
): NotificationCandidate | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const candidateId = reqString(value, 'candidateId');
  const roomId = reqString(value, 'roomId');
  const eventId = optString(value, 'eventId');
  const title = reqString(value, 'title');
  const body = reqString(value, 'body');
  const route = optString(value, 'route');
  const suppressIfFocusedRoom = reqBoolean(value, 'suppressIfFocusedRoom');
  const isEncrypted = reqBoolean(value, 'isEncrypted');
  if (
    candidateId === null ||
    roomId === null ||
    eventId === null ||
    title === null ||
    body === null ||
    route === null ||
    suppressIfFocusedRoom === null ||
    isEncrypted === null ||
    !isNotificationKind(value.kind)
  ) {
    return null;
  }
  return {
    candidateId,
    roomId,
    eventId,
    kind: value.kind,
    title,
    body,
    route,
    suppressIfFocusedRoom,
    isEncrypted,
  };
}
