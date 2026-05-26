import type { MatrixClient } from 'matrix-js-sdk/lib/client';
import { ReceiptType } from 'matrix-js-sdk/lib/@types/read_receipts';
import type { MatrixEvent } from 'matrix-js-sdk/lib/models/event';
import type { Room } from 'matrix-js-sdk/lib/models/room';
import { AccountDataEvent, SynaraUnreadAnchorContent } from '../../types/matrix/accountData';
import { isNotificationEvent, isRoomMarkedUnread } from './room';

const UNREAD_ANCHOR_ACCOUNT_DATA_VERSION = 1;
const unreadAnchorWriteQueues = new WeakMap<MatrixClient, Promise<void>>();

const normalizeUnreadAnchorContent = (
  content?: Partial<SynaraUnreadAnchorContent>
): SynaraUnreadAnchorContent => ({
  version: UNREAD_ANCHOR_ACCOUNT_DATA_VERSION,
  anchors: content?.anchors && typeof content.anchors === 'object' ? content.anchors : {},
});

const updateUnreadAnchorContent = (
  mx: MatrixClient,
  update: (current: SynaraUnreadAnchorContent) => SynaraUnreadAnchorContent
): Promise<void> => {
  const previous = unreadAnchorWriteQueues.get(mx) ?? Promise.resolve();
  const next = previous.then(async () => {
    const event = mx.getAccountData(AccountDataEvent.SynaraUnreadAnchor as any);
    const content = normalizeUnreadAnchorContent(
      event?.getContent() as SynaraUnreadAnchorContent | undefined
    );
    const nextContent = update(content);
    if (nextContent !== content) {
      await mx.setAccountData(AccountDataEvent.SynaraUnreadAnchor as any, nextContent as any);
    }
  });
  unreadAnchorWriteQueues.set(
    mx,
    next.catch(() => undefined)
  );
  return next;
};

export const setUnreadAnchor = (mx: MatrixClient, roomId: string, eventId: string): Promise<void> =>
  updateUnreadAnchorContent(mx, (content) => ({
    ...content,
    anchors: {
      ...content.anchors,
      [roomId]: { eventId, ts: Date.now() },
    },
  }));

export const clearUnreadAnchor = (mx: MatrixClient, roomId: string): Promise<void> =>
  updateUnreadAnchorContent(mx, (content) => {
    if (!content.anchors?.[roomId]) return content;
    const anchors = { ...(content.anchors ?? {}) };
    delete anchors[roomId];
    return { ...content, anchors };
  });

export async function setRoomMarkedUnread(mx: MatrixClient, roomId: string, unread: boolean) {
  await mx.setRoomAccountData(roomId, AccountDataEvent.MarkedUnread, { unread });
}

export async function markAsUnread(mx: MatrixClient, roomId: string) {
  await setRoomMarkedUnread(mx, roomId, true);
}

export async function markEventAsUnread(mx: MatrixClient, room: Room, eventId: string) {
  const timeline = (room.getTimelineForEvent(eventId)?.getEvents() ??
    room.getLiveTimeline().getEvents()) as MatrixEvent[];
  const eventIndex = timeline.findIndex((event) => event.getId() === eventId);
  const anchorEvent =
    eventIndex > 0
      ? timeline
          .slice(0, eventIndex)
          .reverse()
          .find((event) => event.getId() && !event.isSending() && isNotificationEvent(event))
      : undefined;
  const anchorEventId = anchorEvent?.getId() ?? eventId;

  await setUnreadAnchor(mx, room.roomId, anchorEventId);
  await markAsUnread(mx, room.roomId);
}

export async function markAsRead(mx: MatrixClient, roomId: string, privateReceipt: boolean) {
  const room = mx.getRoom(roomId);
  if (!room) return;

  if (isRoomMarkedUnread(room)) {
    await setRoomMarkedUnread(mx, roomId, false);
  }
  await clearUnreadAnchor(mx, roomId);

  const timeline = room.getLiveTimeline().getEvents() as MatrixEvent[];
  const readEventId = room.getEventReadUpTo(mx.getUserId()!);

  const getLatestValidEvent = () => {
    for (let i = timeline.length - 1; i >= 0; i -= 1) {
      const latestEvent = timeline[i];
      if (latestEvent.getId() === readEventId) return null;
      if (!latestEvent.isSending()) return latestEvent;
    }
    return null;
  };
  if (timeline.length === 0) return;
  const latestEvent = getLatestValidEvent();
  if (latestEvent === null) return;

  await mx.sendReadReceipt(
    latestEvent,
    privateReceipt ? ReceiptType.ReadPrivate : ReceiptType.Read
  );
}
