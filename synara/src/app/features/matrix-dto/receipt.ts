/**
 * Read-receipt DTO.
 */

import type { EventId, RoomId, UserId } from './ids';
import { hasForbiddenWireFields, isObject, optNumber, optString, reqString } from './parseUtil';

export const RECEIPT_TYPES = ['read', 'read_private', 'fully_read'] as const;
export type ReceiptType = typeof RECEIPT_TYPES[number];
const RECEIPT_TYPE_SET = new Set<string>(RECEIPT_TYPES);

export function isReceiptType(value: unknown): value is ReceiptType {
  return typeof value === 'string' && RECEIPT_TYPE_SET.has(value);
}

export type Receipt = {
  roomId: RoomId;
  eventId: EventId;
  userId: UserId;
  receiptType: ReceiptType;
  ts?: number;
  threadId?: EventId;
};

export function parseReceipt(value: unknown): Receipt | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const roomId = reqString(value, 'roomId');
  const eventId = reqString(value, 'eventId');
  const userId = reqString(value, 'userId');
  const ts = optNumber(value, 'ts');
  const threadId = optString(value, 'threadId');
  if (
    roomId === null ||
    eventId === null ||
    userId === null ||
    ts === null ||
    threadId === null ||
    !isReceiptType(value.receiptType)
  ) {
    return null;
  }
  return {
    roomId,
    eventId,
    userId,
    receiptType: value.receiptType,
    ts,
    threadId,
  };
}
