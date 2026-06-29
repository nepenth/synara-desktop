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

export const hasUnreadForInitialScroll = (
  unread: Unread | undefined,
  unreadAnchorEventId?: string
): boolean =>
  Boolean(unreadAnchorEventId || (unread && (unread.total > 0 || unread.highlight > 0)));

export const timelineHasEvents = (timeline: TimelineWindow): boolean =>
  getTimelinesEventsCount(timeline.linkedTimelines) > 0;
