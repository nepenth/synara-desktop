/** Structural projections of the js-sdk receipt/account-data surface. */
import type { MatrixClientReading, MatrixEventReading, RoomReading } from './room';

/** Client surface used by the read-marker engine (js-sdk ReceiptClientReading satisfies). */
export type ReceiptClientReading = MatrixClientReading & {
  setAccountData(eventType: string, content: unknown): Promise<unknown>;
  setRoomAccountData(roomId: string, eventType: string, content: unknown): Promise<unknown>;
  setRoomReadMarkers(
    roomId: string,
    fullyReadEventId: string,
    publicReceipt?: MatrixEventReading,
    privateReceipt?: MatrixEventReading
  ): Promise<unknown>;
  sendReadReceipt(event: unknown, receiptType?: string): Promise<unknown>;
  getLatestTimeline(
    timelineSet: unknown
  ): Promise<{ getEvents(): MatrixEventReading[] } | null | undefined>;
};

/** ReceiptRoomReading surface used by the read-marker engine (js-sdk ReceiptRoomReading satisfies). */
export type ReceiptRoomReading = RoomReading & {
  getAccountData?(eventType: string): MatrixEventReading | undefined;
  getReadReceiptForUserId?(
    userId: string,
    ignoreSynthesized?: boolean,
    receiptType?: string
  ): { eventId?: string } | null | undefined;
  compareEventOrdering(a: string, b: string): number | null;
  getUnfilteredTimelineSet(): unknown;
};
import { AccountDataEvent, SynaraUnreadAnchorContent } from '../../types/matrix/accountData';
import { isNotificationEvent } from './room';
import { getLoadedLiveTimelineEvents } from './timelineLifecycle';
import { invokeDesktopWithAvailability, isSynaraDesktop } from './desktop';
import { setRoomReadStateWithNativeOwner } from './nativeRoomReadStateOwner';

const UNREAD_ANCHOR_ACCOUNT_DATA_VERSION = 1;
const unreadAnchorWriteQueues = new WeakMap<ReceiptClientReading, Promise<void>>();
const normalizeUnreadAnchorContent = (
  content?: Partial<SynaraUnreadAnchorContent>
): SynaraUnreadAnchorContent => ({
  version: UNREAD_ANCHOR_ACCOUNT_DATA_VERSION,
  anchors: content?.anchors && typeof content.anchors === 'object' ? content.anchors : {},
});

const updateUnreadAnchorContent = (
  mx: ReceiptClientReading,
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

export const setUnreadAnchor = (
  mx: ReceiptClientReading,
  roomId: string,
  eventId: string
): Promise<void> =>
  updateUnreadAnchorContent(mx, (content) => ({
    ...content,
    anchors: {
      ...content.anchors,
      [roomId]: { eventId, ts: Date.now() },
    },
  }));

export const clearUnreadAnchor = (mx: ReceiptClientReading, roomId: string): Promise<void> =>
  updateUnreadAnchorContent(mx, (content) => {
    if (!content.anchors?.[roomId]) return content;
    const anchors = { ...(content.anchors ?? {}) };
    delete anchors[roomId];
    return { ...content, anchors };
  });

export async function setRoomMarkedUnread(
  mx: ReceiptClientReading,
  roomId: string,
  unread: boolean
) {
  await mx.setRoomAccountData(roomId, 'm.marked_unread', { unread });
}

export async function markAsUnread(mx: ReceiptClientReading, roomId: string) {
  if (typeof window !== 'undefined' && isSynaraDesktop()) {
    await setRoomReadStateWithNativeOwner(
      roomId,
      'mark_unread',
      true,
      invokeDesktopWithAvailability
    );
    return;
  }
  await setRoomMarkedUnread(mx, roomId, true);
}

export async function markEventAsUnread(
  mx: ReceiptClientReading,
  room: ReceiptRoomReading,
  eventId: string
) {
  const timeline = (room.getTimelineForEvent?.(eventId)?.getEvents() ??
    getLoadedLiveTimelineEvents(room)) as MatrixEventReading[];
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

/**
 * Mark a room read through the native Core owner. Core writes the fully-read
 * marker and the private read receipt for the newest receipt-capable event; the
 * renderer no longer resolves read targets itself. Only the desktop shell ships
 * this renderer, so a missing native owner fails closed instead of guessing.
 */
export async function markAsRead(mx: ReceiptClientReading, roomId: string): Promise<void> {
  if (typeof window === 'undefined' || !isSynaraDesktop()) {
    throw new Error('Mark as read requires the Synara desktop app.');
  }
  await setRoomReadStateWithNativeOwner(roomId, 'mark_read', true, invokeDesktopWithAvailability);
}

/**
 * Starts a read-marker update from UI code that cannot await it. The core
 * markAsRead API remains awaitable so workflows and tests can observe failures.
 */
export function markAsReadInBackground(mx: ReceiptClientReading, roomId: string): void {
  void markAsRead(mx, roomId).catch(() => undefined);
}

/**
 * Executes an explicit user-facing Mark Read command. It always uses the
 * private receipt channel plus Matrix's fully-read marker in native Core.
 */
export async function markAsReadFromExplicitUserAction(
  mx: ReceiptClientReading,
  roomId: string
): Promise<void> {
  await markAsRead(mx, roomId);
}

export function markAsReadFromExplicitUserActionInBackground(
  mx: ReceiptClientReading,
  roomId: string
): void {
  void markAsReadFromExplicitUserAction(mx, roomId).catch(() => undefined);
}
