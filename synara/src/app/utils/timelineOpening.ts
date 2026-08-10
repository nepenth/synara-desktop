/**
 * SDK-neutral structural projections used by this utility boundary.
 *
 * These are narrow, read-only interfaces satisfied by live SDK runtime objects
 * and by the test doubles. They deliberately do not re-export any SDK type so
 * this file stays SDK-free, while callers that still hold live SDK objects
 * keep typechecking.
 */

import {
  getEventTimeline,
  getFirstLinkedTimeline,
  getLinkedTimelines,
  getLiveTimeline,
  getTimelinesEventsCount,
  TimelineDirection,
  type EventTimelineReading,
  type EventTimelineSetReading,
  type TimelineEventReading as LinksTimelineEventReading,
} from './timelineLinks';
import type { Unread } from '../../types/matrix/room';

type EventContentReading = { [key: string]: any };

/** Narrow structural projection of a room event used by timeline opening. */
export type TimelineEventReading = {
  getId(): string | undefined;
  getContent<T extends EventContentReading = EventContentReading>(): T;
};

/** Narrow structural projection of a wrapped read receipt. */
export type TimelineWrappedReceiptReading = {
  eventId: string;
  data: { ts: number };
};

/** Narrow structural projection of a room used by timeline opening. */
export type RoomReading = {
  client: { getUserId(): string | null };
  getAccountData(eventType: string): TimelineEventReading | undefined;
  getLiveTimeline(): EventTimelineReading;
  getUnfilteredTimelineSet(): EventTimelineSetReading;
  getReadReceiptForUserId(
    userId: string,
    ignoreSynthesized?: boolean,
    receiptType?: string
  ): TimelineWrappedReceiptReading | null;
  compareEventOrdering(leftEventId: string, rightEventId: string): number | null;
};

/**
 * String-literal mirrors of the SDK account-data event types used below, so
 * live account-data reads receive the exact same wire strings as the previous
 * enum constants.
 */
const EventTypeReading = {
  FullyRead: 'm.fully_read',
  MarkedUnread: 'm.marked_unread',
} as const;

/** String-literal mirror of the SDK receipt types used below. */
const ReceiptTypeReading = {
  Read: 'm.read',
  ReadPrivate: 'm.read.private',
} as const;

/** Structural shape of an m.receipt event content read from a room event. */
type ReceiptContent = {
  [eventId: string]: {
    [receiptType: string]: {
      [userId: string]: { ts: number };
    };
  };
};

export type TimelineRange = {
  start: number;
  end: number;
};

export type TimelineWindow = {
  linkedTimelines: EventTimelineReading[];
  range: TimelineRange;
};

export type RoomReadFrontierSource =
  | 'marked-unread-anchor'
  | 'private-receipt'
  | 'public-receipt'
  | 'fully-read'
  | 'absent';

export type RoomReadFrontier = {
  eventId?: string;
  source: RoomReadFrontierSource;
  isExplicitlyMarkedUnread: boolean;
  isAtLiveTail: boolean;
};

type RoomReadFrontierCandidate = {
  eventId: string;
  source: Exclude<RoomReadFrontierSource, 'marked-unread-anchor' | 'absent'>;
  receiptTimestamp?: number;
};

export type TimelineWindowIdentitySnapshot = {
  provider?: EventTimelineReading;
  eventCount: number;
  range: TimelineRange;
  tailEventId?: string;
};

/**
 * A tiny latest-value queue used by the production timeline while wheel/touch
 * momentum is active. Structural updates are applied once after scroll idle,
 * when the component can measure and restore the user's current event anchor.
 */
export class LatestTimelineStructuralUpdateQueue<T> {
  private pending?: T;

  public enqueue(update: T): void {
    this.pending = update;
  }

  public take(): T | undefined {
    const update = this.pending;
    this.pending = undefined;
    return update;
  }

  public clear(): void {
    this.pending = undefined;
  }

  public hasPending(): boolean {
    return this.pending !== undefined;
  }
}

export const preserveTimelineStructuralUpdateAnchor = <T extends { preserveAnchor: boolean }>(
  update: T
): T => (update.preserveAnchor ? update : { ...update, preserveAnchor: true });

export const getTimelineRangeAfterPagination = ({
  currentRange,
  totalEvents,
  offsetRange,
  backwards,
  limit,
  maxRows,
}: {
  currentRange: TimelineRange;
  totalEvents: number;
  offsetRange: number;
  backwards: boolean;
  limit: number;
  maxRows: number;
}): TimelineRange => {
  const boundedTotal = Math.max(0, totalEvents);
  const shiftedStart = currentRange.start + Math.max(0, offsetRange);
  const shiftedEnd = currentRange.end + Math.max(0, offsetRange);
  const expanded = backwards
    ? {
        start: shiftedStart - Math.max(0, limit),
        end: shiftedEnd,
      }
    : {
        start: shiftedStart,
        end: shiftedEnd + Math.max(0, limit),
      };
  const normalized = {
    start: Math.max(0, Math.min(expanded.start, boundedTotal)),
    end: Math.max(0, Math.min(expanded.end, boundedTotal)),
  };
  if (normalized.end < normalized.start) {
    normalized.start = normalized.end;
  }

  const boundedMaxRows = Math.max(1, maxRows);
  if (normalized.end - normalized.start <= boundedMaxRows) {
    return normalized;
  }

  // Keep the edge the user is moving toward. The visible event anchor is
  // restored separately after rows are inserted or removed.
  if (backwards) {
    return {
      start: normalized.start,
      end: Math.min(boundedTotal, normalized.start + boundedMaxRows),
    };
  }
  return {
    start: Math.max(0, normalized.end - boundedMaxRows),
    end: normalized.end,
  };
};

export const getTimelineFocusRange = ({
  targetIndex,
  totalEvents,
  contextLimit,
  maxRows,
}: {
  targetIndex: number;
  totalEvents: number;
  contextLimit: number;
  maxRows: number;
}): TimelineRange => {
  const boundedTotal = Math.max(0, totalEvents);
  const boundedTarget = Math.max(0, Math.min(targetIndex, Math.max(0, boundedTotal - 1)));
  const boundedContext = Math.max(0, contextLimit);
  const start = Math.max(0, boundedTarget - boundedContext);
  const end = Math.min(boundedTotal, boundedTarget + boundedContext + 1);
  if (end - start <= Math.max(1, maxRows)) {
    return { start, end };
  }

  const halfWindow = Math.floor(Math.max(1, maxRows) / 2);
  const centeredStart = Math.max(0, boundedTarget - halfWindow);
  const centeredEnd = Math.min(boundedTotal, centeredStart + Math.max(1, maxRows));
  return {
    start: Math.max(0, centeredEnd - Math.max(1, maxRows)),
    end: centeredEnd,
  };
};

type TimelineViewportAnchorSnapshot = {
  eventId?: string;
};

type TimelineViewportSnapshot = {
  atBottom: boolean;
  anchor?: TimelineViewportAnchorSnapshot;
};

export type RoomUnreadInfo = {
  readUptoEventId: string;
  inLiveTimeline: boolean;
  scrollTo: boolean;
};

export const getTimelineEndWindow = (
  linkedTimelines: EventTimelineReading[],
  windowLimit: number
): TimelineWindow => {
  const eventsLength = getTimelinesEventsCount(linkedTimelines);
  return {
    linkedTimelines,
    range: {
      start: Math.max(eventsLength - windowLimit, 0),
      end: eventsLength,
    },
  };
};

export const getInitialTimeline = (room: RoomReading, windowLimit: number): TimelineWindow => {
  const linkedTimelines = getLinkedTimelines(getLiveTimeline(room));
  return getTimelineEndWindow(linkedTimelines, windowLimit);
};

export const getEmptyTimeline = (): TimelineWindow => ({
  range: { start: 0, end: 0 },
  linkedTimelines: [],
});

export const timelineWindowContainsEventId = (
  timelineWindow: TimelineWindow,
  eventId: string
): boolean => {
  let absoluteIndex = 0;
  for (const timeline of timelineWindow.linkedTimelines) {
    for (const event of timeline.getEvents()) {
      if (event.getId() === eventId) {
        return (
          absoluteIndex >= timelineWindow.range.start && absoluteIndex < timelineWindow.range.end
        );
      }
      absoluteIndex += 1;
    }
  }
  return false;
};

export const canReplaceTimelineWindowPreservingAnchor = (
  timelineWindow: TimelineWindow,
  anchorEventId?: string
): boolean => !anchorEventId || timelineWindowContainsEventId(timelineWindow, anchorEventId);

export const canRestoreViewportFromInitialTimeline = (
  viewport: TimelineViewportSnapshot | undefined,
  timelineWindow: TimelineWindow
): boolean => {
  if (!viewport || viewport.atBottom) return true;
  const eventId = viewport.anchor?.eventId;
  if (!eventId) return false;
  return timelineWindowContainsEventId(timelineWindow, eventId);
};

const readFrontierSourcePriority: Record<RoomReadFrontierCandidate['source'], number> = {
  'fully-read': 0,
  'public-receipt': 1,
  'private-receipt': 2,
};

const receiptCandidate = (
  receipt: TimelineWrappedReceiptReading | null | undefined,
  source: 'public-receipt' | 'private-receipt'
): RoomReadFrontierCandidate | undefined => {
  if (!receipt?.eventId) return undefined;
  return {
    eventId: receipt.eventId,
    source,
    receiptTimestamp: receipt.data.ts,
  };
};

const compareReadFrontierCandidates = (
  room: RoomReading,
  left: RoomReadFrontierCandidate,
  right: RoomReadFrontierCandidate
): number => {
  if (left.eventId === right.eventId) {
    return readFrontierSourcePriority[left.source] - readFrontierSourcePriority[right.source];
  }

  const sdkOrdering = room.compareEventOrdering?.(left.eventId, right.eventId);
  if (sdkOrdering !== null && sdkOrdering !== undefined && sdkOrdering !== 0) {
    return sdkOrdering;
  }

  if (
    left.receiptTimestamp !== undefined &&
    right.receiptTimestamp !== undefined &&
    left.receiptTimestamp !== right.receiptTimestamp
  ) {
    return left.receiptTimestamp - right.receiptTimestamp;
  }

  // m.fully_read has no timestamp of its own and is commonly left behind by
  // clients which only advance receipts. When the SDK cannot order the events,
  // prefer a real server receipt; private wins an otherwise ambiguous tie, in
  // line with the SDK's own receipt selection.
  return readFrontierSourcePriority[left.source] - readFrontierSourcePriority[right.source];
};

const getLoadedLiveTailEventId = (room: RoomReading): string | undefined => {
  const liveEvents = room.getLiveTimeline().getEvents();
  for (let index = liveEvents.length - 1; index >= 0; index -= 1) {
    const eventId = liveEvents[index]?.getId();
    if (eventId) return eventId;
  }
  return undefined;
};

/**
 * Resolve the newest durable Matrix read position known to this client.
 *
 * The Synara-specific anchor is intentionally authoritative only while the
 * standard m.marked_unread flag is true. This prevents a stale custom account
 * data entry from dragging future room opens back into old history.
 */
export const resolveRoomReadFrontier = (
  room: RoomReading,
  unreadAnchorEventId?: string
): RoomReadFrontier => {
  const isExplicitlyMarkedUnread =
    room.getAccountData?.(EventTypeReading.MarkedUnread)?.getContent<{ unread?: boolean }>()
      .unread === true;

  let eventId: string | undefined;
  let source: RoomReadFrontierSource = 'absent';

  if (isExplicitlyMarkedUnread && unreadAnchorEventId) {
    eventId = unreadAnchorEventId;
    source = 'marked-unread-anchor';
  } else {
    const userId = room.client.getUserId() ?? '';
    const candidates: RoomReadFrontierCandidate[] = [];
    if (userId) {
      const publicReceipt = receiptCandidate(
        room.getReadReceiptForUserId?.(userId, true, ReceiptTypeReading.Read),
        'public-receipt'
      );
      const privateReceipt = receiptCandidate(
        room.getReadReceiptForUserId?.(userId, true, ReceiptTypeReading.ReadPrivate),
        'private-receipt'
      );
      if (publicReceipt) candidates.push(publicReceipt);
      if (privateReceipt) candidates.push(privateReceipt);
    }

    const fullyReadEventId = room
      .getAccountData?.(EventTypeReading.FullyRead)
      ?.getContent<{ event_id?: string }>().event_id;
    if (fullyReadEventId) {
      candidates.push({ eventId: fullyReadEventId, source: 'fully-read' });
    }

    const newest = candidates.reduce<RoomReadFrontierCandidate | undefined>(
      (current, candidate) =>
        !current || compareReadFrontierCandidates(room, candidate, current) > 0
          ? candidate
          : current,
      undefined
    );
    eventId = newest?.eventId;
    source = newest?.source ?? 'absent';
  }

  const liveTailEventId = getLoadedLiveTailEventId(room);
  const tailOrdering =
    eventId && liveTailEventId && eventId !== liveTailEventId
      ? room.compareEventOrdering?.(eventId, liveTailEventId)
      : undefined;
  const isAtLiveTail = Boolean(
    eventId &&
      liveTailEventId &&
      (eventId === liveTailEventId || (tailOrdering !== null && (tailOrdering ?? -1) >= 0))
  );

  return { eventId, source, isExplicitlyMarkedUnread, isAtLiveTail };
};

export const getRoomReadFrontierRevisionKey = (frontier: RoomReadFrontier): string =>
  `${frontier.source} ${frontier.eventId ?? ''} ${frontier.isExplicitlyMarkedUnread ? 1 : 0} ${
    frontier.isAtLiveTail ? 1 : 0
  }`;

export const receiptEventContainsUser = (event: TimelineEventReading, userId: string): boolean => {
  const content = event.getContent<ReceiptContent>();
  return Object.values(content).some((receiptTypes) =>
    Object.values(receiptTypes).some((receiptsByUser) => Boolean(receiptsByUser?.[userId]))
  );
};

export const getRoomUnreadInfo = (
  room: RoomReading,
  readFrontier: RoomReadFrontier = resolveRoomReadFrontier(room),
  scrollTo = false
): RoomUnreadInfo | undefined => {
  const readUptoEventId = readFrontier.eventId;
  if (!readUptoEventId) return undefined;
  const eventTimeline = getEventTimeline(room, readUptoEventId);
  const latestTimeline =
    eventTimeline && getFirstLinkedTimeline(eventTimeline, TimelineDirection.Forward);
  return {
    readUptoEventId,
    inLiveTimeline: latestTimeline === room.getLiveTimeline(),
    scrollTo,
  };
};

export const getRoomUnreadInfoInTimelineWindow = (
  room: RoomReading,
  timelineWindow: TimelineWindow,
  readFrontier: RoomReadFrontier = resolveRoomReadFrontier(room),
  scrollTo = false
): RoomUnreadInfo | undefined => {
  const unreadInfo = getRoomUnreadInfo(room, readFrontier, scrollTo);
  if (!unreadInfo?.inLiveTimeline) return undefined;
  if (!timelineWindowContainsEventId(timelineWindow, unreadInfo.readUptoEventId)) {
    return undefined;
  }
  return unreadInfo;
};

export const hasUnreadForInitialScroll = (
  unread: Unread | undefined,
  readFrontier: RoomReadFrontier
): boolean => {
  if (readFrontier.isExplicitlyMarkedUnread) return true;
  if (readFrontier.isAtLiveTail) return false;
  return Boolean(unread && (unread.total > 0 || unread.highlight > 0));
};

/**
 * Whether saved historical viewport restore should be gated by unread state.
 *
 * Distinct from auto-open-at-unread: deep unread outside the initial live-end
 * window still blocks historical restore (`hasUnreadSignal`), but does not force
 * unread placement unless the marker is already in that window (v1.2.28).
 */
export const shouldGateViewportRestoreOnUnread = (hasUnreadSignal: boolean): boolean =>
  Boolean(hasUnreadSignal);

export const timelineHasEvents = (timeline: TimelineWindow): boolean =>
  getTimelinesEventsCount(timeline.linkedTimelines) > 0;

export const getTimelineWindowTailEvent = (
  timelineWindow: TimelineWindow
): LinksTimelineEventReading | undefined => {
  if (timelineWindow.range.end <= timelineWindow.range.start) return undefined;

  const targetIndex = timelineWindow.range.end - 1;
  let absoluteIndex = 0;
  for (const timeline of timelineWindow.linkedTimelines) {
    for (const event of timeline.getEvents()) {
      if (absoluteIndex === targetIndex) return event;
      absoluteIndex += 1;
    }
  }
  return undefined;
};

export const getTimelineWindowTailEventId = (timelineWindow: TimelineWindow): string | undefined =>
  getTimelineWindowTailEvent(timelineWindow)?.getId();

export const getTimelineWindowIdentitySnapshot = (
  timelineWindow: TimelineWindow
): TimelineWindowIdentitySnapshot => ({
  provider: timelineWindow.linkedTimelines[timelineWindow.linkedTimelines.length - 1],
  eventCount: getTimelinesEventsCount(timelineWindow.linkedTimelines),
  range: { ...timelineWindow.range },
  tailEventId: getTimelineWindowTailEventId(timelineWindow),
});

/**
 * A refresh with the same row count can still represent a different live tail
 * after a limited-sync replacement. Provider and tail identity are therefore
 * part of the equality check, not just count/range.
 */
export const shouldAdoptTimelineRefresh = (
  current: TimelineWindowIdentitySnapshot,
  next: TimelineWindow
): boolean => {
  const nextIdentity = getTimelineWindowIdentitySnapshot(next);
  return !(
    current.provider === nextIdentity.provider &&
    current.eventCount === nextIdentity.eventCount &&
    current.range.start === nextIdentity.range.start &&
    current.range.end === nextIdentity.range.end &&
    current.tailEventId === nextIdentity.tailEventId
  );
};

/**
 * Jump-to-Unread visibility.
 *
 * v1.2.28 only auto-opens at unread when the marker is inside the initial live-end
 * window. Markers that sit in the live chain but outside that window must still
 * expose Jump to Unread so the user can recover without walking history on open.
 *
 * Bounded row rendering uses this range as the authoritative visible window.
 * Markers outside the window remain explicit navigation targets rather than
 * triggering an unbounded history walk during room open.
 */
export const shouldShowJumpToUnread = (
  unreadInfo: RoomUnreadInfo | undefined,
  timelineWindow: TimelineWindow
): boolean => {
  if (!unreadInfo?.readUptoEventId) return false;
  if (!unreadInfo.inLiveTimeline) return true;
  return !timelineWindowContainsEventId(timelineWindow, unreadInfo.readUptoEventId);
};

export type RoomTimelineOpenMode =
  | 'focused-event'
  | 'unread-window'
  | 'saved-viewport'
  | 'live-end';

export type RoomTimelineOpenDiagnostics = {
  openMode: RoomTimelineOpenMode;
  hasUnreadTarget: boolean;
  unreadInInitialWindow: boolean;
  linkedEventCount: number;
  loadedAtEnd: boolean;
};

export const getRoomTimelineOpenMode = ({
  focusedEventId,
  shouldOpenAtUnread,
  shouldRestoreSavedViewport,
}: {
  focusedEventId?: string;
  shouldOpenAtUnread: boolean;
  shouldRestoreSavedViewport: boolean;
}): RoomTimelineOpenMode => {
  if (focusedEventId) return 'focused-event';
  if (shouldOpenAtUnread) return 'unread-window';
  if (shouldRestoreSavedViewport) return 'saved-viewport';
  return 'live-end';
};

export const buildRoomTimelineOpenDiagnostics = ({
  openMode,
  unreadTargetEventId,
  unreadInInitialWindow,
  linkedEventCount,
  loadedAtEnd,
}: {
  openMode: RoomTimelineOpenMode;
  unreadTargetEventId?: string;
  unreadInInitialWindow: boolean;
  linkedEventCount: number;
  loadedAtEnd: boolean;
}): RoomTimelineOpenDiagnostics => ({
  openMode,
  hasUnreadTarget: Boolean(unreadTargetEventId),
  unreadInInitialWindow,
  linkedEventCount,
  loadedAtEnd,
});
