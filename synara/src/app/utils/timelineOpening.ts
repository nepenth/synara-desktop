import { Direction, type EventTimeline, type Room } from 'matrix-js-sdk';
import { type Unread } from '../../types/matrix/room';
import {
  getEventTimeline,
  getFirstLinkedTimeline,
  getLinkedTimelines,
  getLiveTimeline,
  getTimelinesEventsCount,
} from './timelineLinks';

export type TimelineRange = {
  start: number;
  end: number;
};

export type TimelineWindow = {
  linkedTimelines: EventTimeline[];
  range: TimelineRange;
};

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
  linkedTimelines: EventTimeline[],
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

export const getInitialTimeline = (room: Room, windowLimit: number): TimelineWindow => {
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

export const canRestoreViewportFromInitialTimeline = (
  viewport: TimelineViewportSnapshot | undefined,
  timelineWindow: TimelineWindow
): boolean => {
  if (!viewport || viewport.atBottom) return true;
  const eventId = viewport.anchor?.eventId;
  if (!eventId) return false;
  return timelineWindowContainsEventId(timelineWindow, eventId);
};

export const getRoomUnreadInfo = (
  room: Room,
  anchorEventId?: string,
  scrollTo = false
): RoomUnreadInfo | undefined => {
  const readUptoEventId = anchorEventId ?? room.getEventReadUpTo(room.client.getUserId() ?? '');
  if (!readUptoEventId) return undefined;
  const eventTimeline = getEventTimeline(room, readUptoEventId);
  const latestTimeline = eventTimeline && getFirstLinkedTimeline(eventTimeline, Direction.Forward);
  return {
    readUptoEventId,
    inLiveTimeline: latestTimeline === room.getLiveTimeline(),
    scrollTo,
  };
};

export const getRoomUnreadInfoInTimelineWindow = (
  room: Room,
  timelineWindow: TimelineWindow,
  anchorEventId?: string,
  scrollTo = false
): RoomUnreadInfo | undefined => {
  const unreadInfo = getRoomUnreadInfo(room, anchorEventId, scrollTo);
  if (!unreadInfo?.inLiveTimeline) return undefined;
  if (!timelineWindowContainsEventId(timelineWindow, unreadInfo.readUptoEventId)) {
    return undefined;
  }
  return unreadInfo;
};

export const hasUnreadForInitialScroll = (
  unread: Unread | undefined,
  unreadAnchorEventId?: string
): boolean =>
  Boolean(unreadAnchorEventId || (unread && (unread.total > 0 || unread.highlight > 0)));

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

export const getTimelineWindowTailEventId = (
  timelineWindow: TimelineWindow
): string | undefined => {
  if (timelineWindow.range.end <= timelineWindow.range.start) return undefined;

  const targetIndex = timelineWindow.range.end - 1;
  let absoluteIndex = 0;
  for (const timeline of timelineWindow.linkedTimelines) {
    for (const event of timeline.getEvents()) {
      if (absoluteIndex === targetIndex) return event.getId();
      absoluteIndex += 1;
    }
  }
  return undefined;
};

/**
 * `MatrixClient.getLatestTimeline` may return a detached `/context` timeline
 * around the latest event rather than the room's current live timeline object.
 * The fetched tail event is therefore the authority for a jump until `/sync`
 * links that event into a replacement live timeline.
 */
export const timelineWindowEndsAtEventId = (
  timelineWindow: TimelineWindow,
  eventId: string | undefined
): boolean => Boolean(eventId && getTimelineWindowTailEventId(timelineWindow) === eventId);

export const shouldCancelTimelineNavigationForRouteChange = (
  previousRouteKey: string,
  currentRouteKey: string,
  expectedRouteKey?: string
): boolean => previousRouteKey !== currentRouteKey && currentRouteKey !== expectedRouteKey;

export const getPersistedLiveTailEventId = (
  authoritativeTailEventId: string | undefined,
  loadedLiveTailEventId: string | undefined
): string | undefined => authoritativeTailEventId ?? loadedLiveTailEventId;

export const shouldApplyLiveTailRefresh = (
  requestId: number,
  currentRequestId: number,
  jumpPhase: 'idle' | 'loading' | 'settling' | 'error'
): boolean => requestId === currentRequestId && jumpPhase !== 'loading' && jumpPhase !== 'settling';

/**
 * Jump-to-Unread visibility.
 *
 * v1.2.28 only auto-opens at unread when the marker is inside the initial live-end
 * window. Markers that sit in the live chain but outside that window must still
 * expose Jump to Unread so the user can recover without walking history on open.
 *
 * Bounded row rendering uses this range as the authoritative visible SDK window.
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
