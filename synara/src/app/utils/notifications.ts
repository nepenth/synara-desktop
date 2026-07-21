import type { MatrixClient } from 'matrix-js-sdk/lib/client';
import { ReceiptType } from 'matrix-js-sdk/lib/@types/read_receipts';
import type { MatrixEvent } from 'matrix-js-sdk/lib/models/event';
import type { Room } from 'matrix-js-sdk/lib/models/room';
import { EventType } from 'matrix-js-sdk/lib/@types/event';
import { AccountDataEvent, SynaraUnreadAnchorContent } from '../../types/matrix/accountData';
import { isFoundationFeatureEnabled } from '../config/foundationFeatures';
import { recordFoundationDiagnostic } from './foundationDiagnostics';
import { isNotificationEvent, isRoomMarkedUnread } from './room';
import {
  getLatestReceiptEventFromEvents,
  getLatestRoomTimeline,
  getLoadedLiveTimelineEvents,
} from './timelineLifecycle';

const UNREAD_ANCHOR_ACCOUNT_DATA_VERSION = 1;
const unreadAnchorWriteQueues = new WeakMap<MatrixClient, Promise<void>>();
type MarkAsReadMode = 'latest-room' | 'loaded-live-tail';
type ReadMarkerSource = MarkAsReadMode | 'confirmed-event';

type ReadMarkerRequest = {
  event: MatrixEvent;
  privateReceipt: boolean;
};

type ReadMarkerWaiter = {
  resolve: () => void;
  reject: (error: unknown) => void;
};

type ReadMarkerBatch = {
  request: ReadMarkerRequest;
  waiters: Set<ReadMarkerWaiter>;
};

type RoomReadMarkerQueue = {
  active?: ReadMarkerBatch;
  pending?: ReadMarkerBatch;
  running: boolean;
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
    queue = { running: false };
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
  if (!eventId) throw new Error('Cannot set a read marker without an event ID');

  await mx.setRoomReadMarkers(
    room.roomId,
    eventId,
    request.privateReceipt ? undefined : request.event,
    request.privateReceipt ? request.event : undefined
  );

  await clearCustomUnread(mx, room);
  recordFoundationDiagnostic('read', 'marker.commit-success', {
    roomId: room.roomId,
    eventId,
    fields: { privateReceipt: request.privateReceipt },
  });
};

const clearCustomUnread = async (mx: MatrixClient, room: Room): Promise<void> => {
  if (isRoomMarkedUnread(room)) {
    await setRoomMarkedUnread(mx, room.roomId, false);
  }
  await clearUnreadAnchor(mx, room.roomId);
};

const resolveReadMarkerBatch = (batch: ReadMarkerBatch): void => {
  batch.waiters.forEach((waiter) => waiter.resolve());
  batch.waiters.clear();
};

const rejectReadMarkerBatch = (batch: ReadMarkerBatch, error: unknown): void => {
  batch.waiters.forEach((waiter) => waiter.reject(error));
  batch.waiters.clear();
};

const drainReadMarkerQueue = async (
  mx: MatrixClient,
  room: Room,
  queue: RoomReadMarkerQueue
): Promise<void> => {
  try {
    while (queue.pending) {
      const batch = queue.pending;
      queue.pending = undefined;
      queue.active = batch;
      try {
        await commitReadMarker(mx, room, batch.request);
        if (
          !queue.lastCompleted ||
          compareReadMarkerEvents(room, batch.request.event, queue.lastCompleted) >= 0
        ) {
          queue.lastCompleted = batch.request.event;
          queue.lastCompletedPrivateReceipt = batch.request.privateReceipt;
        }
        resolveReadMarkerBatch(batch);
      } catch (error) {
        recordFoundationDiagnostic('read', 'marker.commit-failed', {
          roomId: room.roomId,
          eventId: batch.request.event.getId(),
          fields: {
            privateReceipt: batch.request.privateReceipt,
            errorType: error instanceof Error ? error.name : typeof error,
            waiterCount: batch.waiters.size,
          },
        });
        rejectReadMarkerBatch(batch, error);
      } finally {
        queue.active = undefined;
      }
    }
  } finally {
    queue.running = false;
    if (queue.pending) {
      queue.running = true;
      void drainReadMarkerQueue(mx, room, queue);
    }
  }
};

const startReadMarkerQueue = (mx: MatrixClient, room: Room, queue: RoomReadMarkerQueue): void => {
  if (queue.running) return;
  queue.running = true;
  void drainReadMarkerQueue(mx, room, queue);
};

const enqueueReadMarker = (
  mx: MatrixClient,
  room: Room,
  request: ReadMarkerRequest
): Promise<void> => {
  const queue = getRoomReadMarkerQueue(mx, room.roomId);
  let waiter!: ReadMarkerWaiter;
  const result = new Promise<void>((resolve, reject) => {
    waiter = { resolve, reject };
  });

  if (queue.lastCompleted) {
    const completedOrdering = compareReadMarkerEvents(room, request.event, queue.lastCompleted);
    if (
      completedOrdering < 0 ||
      (completedOrdering === 0 && request.privateReceipt === queue.lastCompletedPrivateReceipt)
    ) {
      void clearCustomUnread(mx, room).then(waiter.resolve, waiter.reject);
      return result;
    }
  }

  if (queue.active) {
    const activeOrdering = compareReadMarkerEvents(room, request.event, queue.active.request.event);
    if (
      activeOrdering < 0 ||
      (activeOrdering === 0 && request.privateReceipt === queue.active.request.privateReceipt)
    ) {
      queue.active.waiters.add(waiter);
      return result;
    }
  }

  if (queue.pending) {
    const pendingOrdering = compareReadMarkerEvents(
      room,
      request.event,
      queue.pending.request.event
    );
    if (
      pendingOrdering < 0 ||
      (pendingOrdering === 0 && request.privateReceipt === queue.pending.request.privateReceipt)
    ) {
      queue.pending.waiters.add(waiter);
      return result;
    }
    queue.pending = {
      request,
      waiters: new Set([...queue.pending.waiters, waiter]),
    };
  } else {
    queue.pending = { request, waiters: new Set([waiter]) };
  }

  startReadMarkerQueue(mx, room, queue);
  return result;
};

const markResolvedEventAsRead = async (
  mx: MatrixClient,
  room: Room,
  privateReceipt: boolean,
  latestEvent: MatrixEvent,
  source: ReadMarkerSource
): Promise<void> => {
  const roomId = room.roomId;
  const userId = mx.getUserId();
  if (!userId) return;

  const latestEventId = latestEvent.getId();
  if (!latestEventId || latestEvent.isSending()) return;
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
    recordFoundationDiagnostic('read', 'marker.already-current', {
      roomId,
      eventId: latestEventId,
      fields: { privateReceipt, mode: source },
    });
    return;
  }

  if (!isFoundationFeatureEnabled('exactReadMarkers')) {
    try {
      await mx.sendReadReceipt(
        latestEvent,
        privateReceipt ? ReceiptType.ReadPrivate : ReceiptType.Read
      );
      await clearCustomUnread(mx, room);
      recordFoundationDiagnostic('read', 'marker.legacy-success', {
        roomId,
        eventId: latestEventId,
        fields: { privateReceipt, mode: source },
      });
    } catch (error) {
      recordFoundationDiagnostic('read', 'marker.legacy-failed', {
        roomId,
        eventId: latestEventId,
        fields: {
          privateReceipt,
          mode: source,
          errorType: error instanceof Error ? error.name : typeof error,
        },
      });
      throw error;
    }
    return;
  }

  await enqueueReadMarker(mx, room, { event: latestEvent, privateReceipt });
};

export async function markAsRead(
  mx: MatrixClient,
  roomId: string,
  privateReceipt: boolean,
  mode: MarkAsReadMode = 'latest-room'
): Promise<void> {
  const room = mx.getRoom(roomId);
  if (!room) return;

  const timeline =
    mode === 'latest-room'
      ? (await getLatestRoomTimeline(mx, room))?.getEvents() ?? getLoadedLiveTimelineEvents(room)
      : getLoadedLiveTimelineEvents(room);
  const latestEvent = getLatestReceiptEventFromEvents(timeline);

  if (!latestEvent) return;

  await markResolvedEventAsRead(mx, room, privateReceipt, latestEvent, mode);
}

/**
 * Commits a caller-confirmed read target without resolving it through the room's
 * loaded live timeline. Use this after an authoritative SDK latest/context
 * operation has returned an event, including detached latest timelines.
 */
export async function markAsReadAtEvent(
  mx: MatrixClient,
  roomId: string,
  privateReceipt: boolean,
  event: MatrixEvent
): Promise<void> {
  const room = mx.getRoom(roomId);
  if (!room) return;

  await markResolvedEventAsRead(mx, room, privateReceipt, event, 'confirmed-event');
}
