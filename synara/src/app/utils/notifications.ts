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
import { isFoundationFeatureEnabled } from '../config/foundationFeatures';
import { recordFoundationDiagnostic } from './foundationDiagnostics';
import { isNotificationEvent, isRoomMarkedUnread } from './room';
import {
  getLatestReceiptEventFromEvents,
  getLatestRoomTimeline,
  getLoadedLiveTimelineEvents,
} from './timelineLifecycle';
import { invokeDesktopWithAvailability, isSynaraDesktop } from './desktop';
import { setRoomReadStateWithNativeOwner } from './nativeRoomReadStateOwner';

const UNREAD_ANCHOR_ACCOUNT_DATA_VERSION = 1;
const unreadAnchorWriteQueues = new WeakMap<ReceiptClientReading, Promise<void>>();
type MarkAsReadMode = 'latest-room' | 'loaded-live-tail';
type ReadMarkerSource = MarkAsReadMode | 'confirmed-event';

type ReadMarkerRequest = {
  event: MatrixEventReading;
  privateReceipt: boolean;
};

type ReadMarkerWaiter = {
  resolve: () => void;
  reject: (error: unknown) => void;
};

type ReadReceiptChannel = 'public' | 'private';

type ReadMarkerChannelTarget = {
  event: MatrixEventReading;
  waiters: Set<ReadMarkerWaiter>;
};

type ReadMarkerChannels = Partial<Record<ReadReceiptChannel, ReadMarkerChannelTarget>>;

type PendingReadMarkerChannels = Partial<Record<ReadReceiptChannel, ReadMarkerChannelTarget[]>>;

type ReadMarkerBatch = {
  fullyReadEvent: MatrixEventReading;
  channels: ReadMarkerChannels;
};

type RoomReadMarkerQueue = {
  active?: ReadMarkerBatch;
  pending: PendingReadMarkerChannels;
  completed: Partial<Record<ReadReceiptChannel, MatrixEventReading>>;
  fullyReadCompleted?: MatrixEventReading;
  furthestKnown?: MatrixEventReading;
  running: boolean;
};

const roomReadMarkerQueues = new WeakMap<ReceiptClientReading, Map<string, RoomReadMarkerQueue>>();

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

const compareReadMarkerEvents = (
  room: ReceiptRoomReading,
  left: MatrixEventReading,
  right: MatrixEventReading
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
  room: ReceiptRoomReading,
  current: MatrixEventReading | undefined,
  candidate: MatrixEventReading
): MatrixEventReading => {
  if (!current) return candidate;
  const ordering = compareReadMarkerEvents(room, candidate, current);
  return ordering !== null && ordering > 0 ? candidate : current;
};

const eventSatisfiesTarget = (
  room: ReceiptRoomReading,
  event: MatrixEventReading,
  target: MatrixEventReading
): boolean => {
  const ordering = compareReadMarkerEvents(room, event, target);
  return ordering !== null && ordering >= 0;
};

const getReceiptChannel = (privateReceipt: boolean): ReadReceiptChannel =>
  privateReceipt ? 'private' : 'public';

const getRoomReadMarkerQueue = (mx: ReceiptClientReading, roomId: string): RoomReadMarkerQueue => {
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
  mx: ReceiptClientReading,
  room: ReceiptRoomReading,
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

const clearCustomUnread = async (
  mx: ReceiptClientReading,
  room: ReceiptRoomReading
): Promise<void> => {
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

const createReadMarkerBatch = (
  room: ReceiptRoomReading,
  queue: RoomReadMarkerQueue
): ReadMarkerBatch => {
  const channels: ReadMarkerChannels = {};
  (['public', 'private'] as const).forEach((channel) => {
    const targets = queue.pending[channel];
    const target = targets?.shift();
    if (target) channels[channel] = target;
    if (targets?.length === 0) delete queue.pending[channel];
  });

  const pendingEvents = Object.values(channels)
    .map((target) => target?.event)
    .filter((event): event is MatrixEventReading => Boolean(event));
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
  mx: ReceiptClientReading,
  room: ReceiptRoomReading,
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

const startReadMarkerQueue = (
  mx: ReceiptClientReading,
  room: ReceiptRoomReading,
  queue: RoomReadMarkerQueue
): void => {
  if (queue.running) return;
  queue.running = true;
  void drainReadMarkerQueue(mx, room, queue);
};

const enqueueReadMarker = (
  mx: ReceiptClientReading,
  room: ReceiptRoomReading,
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
  mx: ReceiptClientReading,
  room: ReceiptRoomReading,
  privateReceipt: boolean,
  latestEvent: MatrixEventReading,
  source: ReadMarkerSource
): Promise<void> => {
  const roomId = room.roomId;
  const userId = mx.getUserId();
  if (!userId) return;

  const latestEventId = latestEvent.getId();
  if (!latestEventId || latestEvent.isSending()) return;
  const fullyReadEventId = room
    .getAccountData?.('m.fully_read')
    ?.getContent<{ event_id?: string }>().event_id;
  const receiptEventId = room.getReadReceiptForUserId?.(
    userId,
    false,
    privateReceipt ? 'm.read.private' : 'm.read'
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
      await mx.sendReadReceipt(latestEvent, privateReceipt ? 'm.read.private' : 'm.read');
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
  mx: ReceiptClientReading,
  roomId: string,
  privateReceipt: boolean,
  mode: MarkAsReadMode = 'latest-room'
): Promise<void> {
  if (typeof window !== 'undefined' && isSynaraDesktop()) {
    await setRoomReadStateWithNativeOwner(roomId, 'mark_read', true, invokeDesktopWithAvailability);
    return;
  }
  const room = mx.getRoom(roomId) as ReceiptRoomReading | null;
  if (!room) return;

  const timeline =
    mode === 'latest-room'
      ? (await getLatestRoomTimeline(mx, room))?.getEvents() ?? getLoadedLiveTimelineEvents(room)
      : getLoadedLiveTimelineEvents(room);
  const latestEvent = getLatestReceiptEventFromEvents(timeline);

  if (!latestEvent) return;

  await markResolvedEventAsRead(mx, room, privateReceipt, latestEvent as MatrixEventReading, mode);
}

/**
 * Starts a read-marker update from UI code that cannot await it. The core
 * markAsRead API remains awaitable so workflows and tests can observe failures.
 */
export function markAsReadInBackground(
  mx: ReceiptClientReading,
  roomId: string,
  privateReceipt: boolean,
  mode: MarkAsReadMode = 'latest-room'
): void {
  void markAsRead(mx, roomId, privateReceipt, mode).catch(() => undefined);
}

/**
 * Executes an explicit user-facing Mark Read command. This intent remains
 * available when automatic activity sharing is hidden, but always uses the
 * private receipt channel (plus Matrix's fully-read marker in native Core).
 */
export async function markAsReadFromExplicitUserAction(
  mx: ReceiptClientReading,
  roomId: string,
  mode: MarkAsReadMode = 'latest-room'
): Promise<void> {
  await markAsRead(mx, roomId, true, mode);
}

export function markAsReadFromExplicitUserActionInBackground(
  mx: ReceiptClientReading,
  roomId: string,
  mode: MarkAsReadMode = 'latest-room'
): void {
  void markAsReadFromExplicitUserAction(mx, roomId, mode).catch(() => undefined);
}

/**
 * Commits a caller-confirmed read target without resolving it through the room's
 * loaded live timeline. Use this after an authoritative SDK latest/context
 * operation has returned an event, including detached latest timelines.
 */
export async function markAsReadAtEvent(
  mx: ReceiptClientReading,
  roomId: string,
  privateReceipt: boolean,
  event: MatrixEventReading
): Promise<void> {
  const room = mx.getRoom(roomId) as ReceiptRoomReading | null;
  if (!room) return;

  await markResolvedEventAsRead(mx, room, privateReceipt, event, 'confirmed-event');
}
