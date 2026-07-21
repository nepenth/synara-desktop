import type { MatrixClient } from 'matrix-js-sdk/lib/client';
import { ReceiptType } from 'matrix-js-sdk/lib/@types/read_receipts';
import type { MatrixEvent } from 'matrix-js-sdk/lib/models/event';
import type { Room } from 'matrix-js-sdk/lib/models/room';
import { EventType } from 'matrix-js-sdk/lib/@types/event';
import { AccountDataEvent, SynaraUnreadAnchorContent } from '../../types/matrix/accountData';
import { isNotificationEvent, isRoomMarkedUnread } from './room';
import {
  getLatestReceiptEventFromEvents,
  getLatestRoomTimeline,
  getLoadedLiveTimelineEvents,
} from './timelineLifecycle';

const UNREAD_ANCHOR_ACCOUNT_DATA_VERSION = 1;
const unreadAnchorWriteQueues = new WeakMap<MatrixClient, Promise<void>>();
type MarkAsReadMode = 'latest-room' | 'loaded-live-tail';

type ReadMarkerRequest = {
  event: MatrixEvent;
  privateReceipt: boolean;
};

type RoomReadMarkerQueue = {
  active?: ReadMarkerRequest;
  pending?: ReadMarkerRequest;
  running?: Promise<void>;
  lastCompleted?: MatrixEvent;
  lastCompletedPrivateReceipt?: boolean;
};

const roomReadMarkerQueues = new WeakMap<MatrixClient, Map<string, RoomReadMarkerQueue>>();

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
  await mx.setRoomAccountData(roomId, EventType.MarkedUnread, { unread });
}

export async function markAsUnread(mx: MatrixClient, roomId: string) {
  await setRoomMarkedUnread(mx, roomId, true);
}

export async function markEventAsUnread(mx: MatrixClient, room: Room, eventId: string) {
  const timeline = (room.getTimelineForEvent(eventId)?.getEvents() ??
    getLoadedLiveTimelineEvents(room)) as MatrixEvent[];
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

const compareReadMarkerEvents = (room: Room, left: MatrixEvent, right: MatrixEvent): number => {
  const leftId = left.getId();
  const rightId = right.getId();
  if (leftId && rightId) {
    if (leftId === rightId) return 0;
    const ordering = room.compareEventOrdering(leftId, rightId);
    if (ordering !== null) return ordering;
  }

  return left.getTs() - right.getTs();
};

const getRoomReadMarkerQueue = (mx: MatrixClient, roomId: string): RoomReadMarkerQueue => {
  let clientQueues = roomReadMarkerQueues.get(mx);
  if (!clientQueues) {
    clientQueues = new Map();
    roomReadMarkerQueues.set(mx, clientQueues);
  }

  let queue = clientQueues.get(roomId);
  if (!queue) {
    queue = {};
    clientQueues.set(roomId, queue);
  }
  return queue;
};

const commitReadMarker = async (
  mx: MatrixClient,
  room: Room,
  request: ReadMarkerRequest
): Promise<void> => {
  const eventId = request.event.getId();
  if (!eventId) return;

  await mx.setRoomReadMarkers(
    room.roomId,
    eventId,
    request.privateReceipt ? undefined : request.event,
    request.privateReceipt ? request.event : undefined
  );

  await clearCustomUnread(mx, room);
};

const clearCustomUnread = async (mx: MatrixClient, room: Room): Promise<void> => {
  if (isRoomMarkedUnread(room)) {
    await setRoomMarkedUnread(mx, room.roomId, false);
  }
  await clearUnreadAnchor(mx, room.roomId);
};

const enqueueReadMarker = (
  mx: MatrixClient,
  room: Room,
  request: ReadMarkerRequest
): Promise<void> => {
  const queue = getRoomReadMarkerQueue(mx, room.roomId);
  if (queue.lastCompleted) {
    const completedOrdering = compareReadMarkerEvents(room, request.event, queue.lastCompleted);
    if (completedOrdering < 0) return queue.running ?? Promise.resolve();
    if (completedOrdering === 0 && request.privateReceipt === queue.lastCompletedPrivateReceipt) {
      return clearCustomUnread(mx, room);
    }
  }

  if (queue.active) {
    const activeOrdering = compareReadMarkerEvents(room, request.event, queue.active.event);
    if (
      activeOrdering < 0 ||
      (activeOrdering === 0 && request.privateReceipt === queue.active.privateReceipt)
    ) {
      return queue.running ?? Promise.resolve();
    }
  }

  if (!queue.pending || compareReadMarkerEvents(room, request.event, queue.pending.event) >= 0) {
    queue.pending = request;
  }

  if (!queue.running) {
    queue.running = (async () => {
      let lastError: unknown;
      while (queue.pending) {
        const next = queue.pending;
        queue.pending = undefined;
        if (
          queue.lastCompleted &&
          compareReadMarkerEvents(room, next.event, queue.lastCompleted) < 0
        ) {
          continue;
        }
        queue.active = next;
        try {
          await commitReadMarker(mx, room, next);
          lastError = undefined;
          if (
            !queue.lastCompleted ||
            compareReadMarkerEvents(room, next.event, queue.lastCompleted) >= 0
          ) {
            queue.lastCompleted = next.event;
            queue.lastCompletedPrivateReceipt = next.privateReceipt;
          }
        } catch (error) {
          lastError = error;
        } finally {
          queue.active = undefined;
        }
      }
      if (lastError) throw lastError;
    })().finally(() => {
      queue.running = undefined;
    });
  }

  return queue.running;
};

export async function markAsRead(
  mx: MatrixClient,
  roomId: string,
  privateReceipt: boolean,
  mode: MarkAsReadMode = 'latest-room'
) {
  const room = mx.getRoom(roomId);
  if (!room) return;

  const userId = mx.getUserId();
  if (!userId) return;
  const timeline =
    mode === 'latest-room'
      ? (await getLatestRoomTimeline(mx, room))?.getEvents() ?? getLoadedLiveTimelineEvents(room)
      : getLoadedLiveTimelineEvents(room);
  const latestEvent = getLatestReceiptEventFromEvents(timeline);

  if (!latestEvent) return;

  const latestEventId = latestEvent.getId();
  if (!latestEventId) return;
  const fullyReadEventId = room
    .getAccountData?.(ReceiptType.FullyRead)
    ?.getContent<{ event_id?: string }>().event_id;
  const receiptEventId = room.getReadReceiptForUserId?.(
    userId,
    false,
    privateReceipt ? ReceiptType.ReadPrivate : ReceiptType.Read
  )?.eventId;
  if (fullyReadEventId === latestEventId && receiptEventId === latestEventId) {
    await clearCustomUnread(mx, room);
    return;
  }

  await enqueueReadMarker(mx, room, { event: latestEvent, privateReceipt });
}
