/**
 * Thread summary DTO.
 */

import {
  hasForbiddenWireFields,
  isObject,
  optNumber,
  optString,
  reqBoolean,
  reqNumber,
  reqString,
} from './parseUtil';
import type { ThreadSummary as WireThreadSummary } from './generated';
import type { NullsToOptional } from './wireTypes';

/** Parsed form of Core's `ThreadSummary`: absent instead of `null`. */
export type ThreadSummary = NullsToOptional<WireThreadSummary>;

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
