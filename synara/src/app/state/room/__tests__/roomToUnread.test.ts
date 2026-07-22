import assert from 'node:assert/strict';
import test from 'node:test';
import { type EventTimeline, type MatrixClient, type MatrixEvent, type Room } from 'matrix-js-sdk';
import { EventType } from 'matrix-js-sdk/lib/@types/event';
import { ReceiptType } from 'matrix-js-sdk/lib/@types/read_receipts';
import { shouldKeepRoomUnreadAfterReceipt } from '../roomToUnread';

const roomAtTail = (markedUnread: boolean): Room => {
  const tailEvent = { getId: () => '$tail' } as MatrixEvent;
  const liveTimeline = { getEvents: () => [tailEvent] } as EventTimeline;

  return {
    client: { getUserId: () => '@alice:example.org' },
    getLiveTimeline: () => liveTimeline,
    getAccountData: (type: EventType | string) => {
      if (type === EventType.MarkedUnread) {
        return { getContent: () => ({ unread: markedUnread }) } as MatrixEvent;
      }
      return undefined;
    },
    getReadReceiptForUserId: (_userId: string, _ignoreSynthesized: boolean, type: ReceiptType) =>
      type === ReceiptType.Read ? { eventId: '$tail', data: { ts: 100 } } : null,
    compareEventOrdering: () => 0,
  } as unknown as Room;
};

test('receipt cleanup discards stale unread counters when the durable frontier is at live tail', () => {
  const mx = { getUserId: () => '@alice:example.org' } as MatrixClient;
  assert.equal(shouldKeepRoomUnreadAfterReceipt(mx, roomAtTail(false)), false);
});

test('receipt cleanup preserves an explicit marked-unread room even when its receipt is current', () => {
  const mx = { getUserId: () => '@alice:example.org' } as MatrixClient;
  assert.equal(shouldKeepRoomUnreadAfterReceipt(mx, roomAtTail(true)), true);
});
