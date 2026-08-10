/**
 * Thread summary DTO.
 */

import type { EventId, RoomId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optNumber,
  optString,
  reqBoolean,
  reqNumber,
  reqString,
} from './parseUtil';

export type ThreadSummary = {
  roomId: RoomId;
  rootEventId: EventId;
  replyCount: number;
  latestEventId?: EventId;
  latestOriginServerTs?: number;
  participated: boolean;
  unreadCount?: number;
};

export function parseThreadSummary(value: unknown): ThreadSummary | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const rootEventId = reqString(value, 'rootEventId');
  const replyCount = reqNumber(value, 'replyCount');
  const latestEventId = optString(value, 'latestEventId');
  const latestOriginServerTs = optNumber(value, 'latestOriginServerTs');
  const participated = reqBoolean(value, 'participated');
  const unreadCount = optNumber(value, 'unreadCount');
  if (
    roomId === null ||
    rootEventId === null ||
    replyCount === null ||
    latestEventId === null ||
    latestOriginServerTs === null ||
    participated === null ||
    unreadCount === null
  ) {
    return null;
  }
  return {
    roomId,
    rootEventId,
    replyCount,
    latestEventId,
    latestOriginServerTs,
    participated,
    unreadCount,
  };
}
