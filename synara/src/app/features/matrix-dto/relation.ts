/**
 * Relation DTO — reactions, edits, references, threads.
 */

import type { EventId, RoomId, UserId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optString,
  reqString,
} from './parseUtil';

export const REL_TYPE_ANNOTATION = 'annotation';
export const REL_TYPE_REPLACE = 'm.replace';
export const REL_TYPE_REFERENCE = 'm.reference';
export const REL_TYPE_THREAD = 'm.thread';

/** Open string; well-known values listed in constants above. */
export type RelationType = string;

export type RelationRef = {
  relType: RelationType;
  eventId: EventId;
  roomId?: RoomId;
  sender?: UserId;
  key?: string;
};

export function parseRelationRef(value: unknown): RelationRef | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const relType = reqString(value, 'relType');
  const eventId = reqString(value, 'eventId');
  const roomId = optString(value, 'roomId');
  const sender = optString(value, 'sender');
  const key = optString(value, 'key');
  if (
    relType === null ||
    eventId === null ||
    roomId === null ||
    sender === null ||
    key === null
  ) {
    return null;
  }
  return { relType, eventId, roomId, sender, key };
}
