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

type ReadReceiptChannel = 'public' | 'private';

type ReadMarkerChannelTarget = {
  event: MatrixEvent;
  waiters: Set<ReadMarkerWaiter>;
};

type ReadMarkerChannels = Partial<Record<ReadReceiptChannel, ReadMarkerChannelTarget>>;

type PendingReadMarkerChannels = Partial<Record<ReadReceiptChannel, ReadMarkerChannelTarget[]>>;

type ReadMarkerBatch = {
  fullyReadEvent: MatrixEvent;
  channels: ReadMarkerChannels;
};

type RoomReadMarkerQueue = {
  active?: ReadMarkerBatch;
  pending: PendingReadMarkerChannels;
  completed: Partial<Record<ReadReceiptChannel, MatrixEvent>>;
  fullyReadCompleted?: MatrixEvent;
  furthestKnown?: MatrixEvent;
  running: boolean;
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

const compareReadMarkerEvents = (
  room: Room,
  left: MatrixEvent,
  right: MatrixEvent
): number | null => {
  const leftId = left.getId();
  const rightId = right.getId();
  if (leftId && rightId) {
    if (leftId === rightId) return 0;
    const ordering = room.compareEventOrdering(leftId, rightId);
    if (ordering !== null) return ordering;
  }

  const leftTimestamp = left.getTs();
  const rightTimestamp = right.getTs();
  if (
    Number.isFinite(leftTimestamp) &&
    Number.isFinite(rightTimestamp) &&
    leftTimestamp !== rightTimestamp
  ) {
    return leftTimestamp - rightTimestamp;
  }

  return null;
};

const advanceKnownEvent = (
  room: Room,
  current: MatrixEvent | undefined,
  candidate: MatrixEvent
): MatrixEvent => {
  if (!current) return candidate;
  const ordering = compareReadMarkerEvents(room, candidate, current);
  return ordering !== null && ordering > 0 ? candidate : current;
};

const eventSatisfiesTarget = (room: Room, event: MatrixEvent, target: MatrixEvent): boolean => {
  const ordering = compareReadMarkerEvents(room, event, target);
  return ordering !== null && ordering >= 0;
};

const getReceiptChannel = (privateReceipt: boolean): ReadReceiptChannel =>
  privateReceipt ? 'private' : 'public';

const getRoomReadMarkerQueue = (mx: MatrixClient, roomId: string): RoomReadMarkerQueue => {
  let clientQueues = roomReadMarkerQueues.get(mx);
  if (!clientQueues) {
    clientQueues = new Map();
    roomReadMarkerQueues.set(mx, clientQueues);
  }

  let queue = clientQueues.get(roomId);
  if (!queue) {
    queue = { running: false, pending: {}, completed: {} };
    clientQueues.set(roomId, queue);
  }
  return queue;
};

const commitReadMarker = async (
  mx: MatrixClient,
  room: Room,
  batch: ReadMarkerBatch
): Promise<void> => {
  const eventId = batch.fullyReadEvent.getId();
  if (!eventId) throw new Error('Cannot set a read marker without an event ID');

  await mx.setRoomReadMarkers(
    room.roomId,
    eventId,
    batch.channels.public?.event,
    batch.channels.private?.event
  );

  await clearCustomUnread(mx, room);
  recordFoundationDiagnostic('read', 'marker.commit-success', {
    roomId: room.roomId,
    eventId,
    fields: {
      publicReceipt: Boolean(batch.channels.public),
      privateReceipt: Boolean(batch.channels.private),
    },
  });
};

const clearCustomUnread = async (mx: MatrixClient, room: Room): Promise<void> => {
  if (isRoomMarkedUnread(room)) {
    await setRoomMarkedUnread(mx, room.roomId, false);
  }
  await clearUnreadAnchor(mx, room.roomId);
};

const resolveReadMarkerBatch = (batch: ReadMarkerBatch): void => {
  Object.values(batch.channels).forEach((target) => {
    target?.waiters.forEach((waiter) => waiter.resolve());
    target?.waiters.clear();
  });
};

const rejectReadMarkerBatch = (batch: ReadMarkerBatch, error: unknown): void => {
  Object.values(batch.channels).forEach((target) => {
    target?.waiters.forEach((waiter) => waiter.reject(error));
    target?.waiters.clear();
  });
};

const hasPendingReadMarker = (queue: RoomReadMarkerQueue): boolean =>
  Boolean(queue.pending.public?.length || queue.pending.private?.length);

const createReadMarkerBatch = (room: Room, queue: RoomReadMarkerQueue): ReadMarkerBatch => {
  const channels: ReadMarkerChannels = {};
  (['public', 'private'] as const).forEach((channel) => {
    const targets = queue.pending[channel];
    const target = targets?.shift();
    if (target) channels[channel] = target;
    if (targets?.length === 0) delete queue.pending[channel];
  });

  const pendingEvents = Object.values(channels)
    .map((target) => target?.event)
    .filter((event): event is MatrixEvent => Boolean(event));
  const fullyReadEvent = pendingEvents.reduce(
    (furthest, event) => advanceKnownEvent(room, furthest, event),
    queue.furthestKnown ?? queue.fullyReadCompleted
  );
  if (!fullyReadEvent) throw new Error('Cannot create an empty read-marker batch');

  return { fullyReadEvent, channels };
};

const readMarkerWaiterCount = (batch: ReadMarkerBatch): number =>
  Object.values(batch.channels).reduce((count, target) => count + (target?.waiters.size ?? 0), 0);

const drainReadMarkerQueue = async (
  mx: MatrixClient,
  room: Room,
  queue: RoomReadMarkerQueue
): Promise<void> => {
  try {
    while (hasPendingReadMarker(queue)) {
      const batch = createReadMarkerBatch(room, queue);
      queue.active = batch;
      try {
        await commitReadMarker(mx, room, batch);
        queue.fullyReadCompleted = advanceKnownEvent(
          room,
          queue.fullyReadCompleted,
          batch.fullyReadEvent
        );
        Object.entries(batch.channels).forEach(([channel, target]) => {
          if (!target) return;
          queue.completed[channel as ReadReceiptChannel] = advanceKnownEvent(
            room,
            queue.completed[channel as ReadReceiptChannel],
            target.event
          );
        });
        resolveReadMarkerBatch(batch);
      } catch (error) {
        recordFoundationDiagnostic('read', 'marker.commit-failed', {
          roomId: room.roomId,
          eventId: batch.fullyReadEvent.getId(),
          fields: {
            publicReceipt: Boolean(batch.channels.public),
            privateReceipt: Boolean(batch.channels.private),
            errorType: error instanceof Error ? error.name : typeof error,
            waiterCount: readMarkerWaiterCount(batch),
          },
        });
        rejectReadMarkerBatch(batch, error);
      } finally {
        queue.active = undefined;
      }
    }
  } finally {
    queue.running = false;
    if (hasPendingReadMarker(queue)) {
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
  const channel = getReceiptChannel(request.privateReceipt);
  let waiter!: ReadMarkerWaiter;
  const result = new Promise<void>((resolve, reject) => {
    waiter = { resolve, reject };
  });

  queue.furthestKnown = advanceKnownEvent(room, queue.furthestKnown, request.event);

  const completed = queue.completed[channel];
  if (completed && eventSatisfiesTarget(room, completed, request.event)) {
    void clearCustomUnread(mx, room).then(waiter.resolve, waiter.reject);
    return result;
  }

  const active = queue.active?.channels[channel];
  if (active && eventSatisfiesTarget(room, active.event, request.event)) {
    active.waiters.add(waiter);
    return result;
  }

  const pending = queue.pending[channel] ?? [];
  const satisfyingTarget = pending.find((target) =>
    eventSatisfiesTarget(room, target.event, request.event)
  );
  if (satisfyingTarget) {
    satisfyingTarget.waiters.add(waiter);
    return result;
  }

  const supersededTargets = pending.filter((target) =>
    eventSatisfiesTarget(room, request.event, target.event)
  );
  const retainedTargets = pending.filter((target) => !supersededTargets.includes(target));
  queue.pending[channel] = [
    ...retainedTargets,
    {
      event: request.event,
      waiters: new Set([...supersededTargets.flatMap((target) => [...target.waiters]), waiter]),
    },
  ];

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
 * Starts a read-marker update from UI code that cannot await it. The core
 * markAsRead API remains awaitable so workflows and tests can observe failures.
 */
export function markAsReadInBackground(
  mx: MatrixClient,
  roomId: string,
  privateReceipt: boolean,
  mode: MarkAsReadMode = 'latest-room'
): void {
  void markAsRead(mx, roomId, privateReceipt, mode).catch(() => undefined);
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
