/**
 * SDK-neutral structural projections used by this utility boundary.
 *
 * These are narrow, read-only interfaces satisfied by live SDK runtime objects
 * and by the test doubles. They deliberately do not re-export any SDK type so
 * this file stays SDK-free, while callers that still hold live SDK objects
 * keep typechecking.
 */

/** String-literal mirror of the SDK Direction enum. */
export type TimelineDirection = 'b' | 'f';

/** Narrow string-literal mirror of the SDK Direction enum values. */
export const TimelineDirection = {
  Backward: 'b',
  Forward: 'f',
} as const;

/** Narrow structural projection of a room event used by the timeline links. */
export type TimelineEventReading = {
  getId(): string | undefined;
};

/** Narrow structural projection of an event timeline. */
export type EventTimelineReading = {
  getNeighbouringTimeline(direction: TimelineDirection): EventTimelineReading | null;
  getEvents(): TimelineEventReading[];
};

/** Narrow structural projection of an event timeline set. */
export type EventTimelineSetReading = {
  getLiveTimeline(): EventTimelineReading;
  getTimelineForEvent(eventId: string): EventTimelineReading | null;
};

/** Narrow structural projection of a room used by the timeline links. */
export type RoomReading = {
  getUnfilteredTimelineSet(): EventTimelineSetReading;
};

export const getLiveTimeline = (room: RoomReading): EventTimelineReading =>
  room.getUnfilteredTimelineSet().getLiveTimeline();

export const getEventTimeline = (
  room: RoomReading,
  eventId: string
): EventTimelineReading | undefined => {
  const timelineSet = room.getUnfilteredTimelineSet();
  return timelineSet.getTimelineForEvent(eventId) ?? undefined;
};

export const getFirstLinkedTimeline = (
  timeline: EventTimelineReading,
  direction: TimelineDirection
): EventTimelineReading => {
  const linkedTimeline = timeline.getNeighbouringTimeline(direction);
  if (!linkedTimeline) return timeline;
  return getFirstLinkedTimeline(linkedTimeline, direction);
};

export const getLinkedTimelines = (timeline: EventTimelineReading): EventTimelineReading[] => {
  const firstTimeline = getFirstLinkedTimeline(timeline, TimelineDirection.Backward);
  const timelines: EventTimelineReading[] = [];

  for (
    let nextTimeline: EventTimelineReading | null = firstTimeline;
    nextTimeline;
    nextTimeline = nextTimeline.getNeighbouringTimeline(TimelineDirection.Forward)
  ) {
    timelines.push(nextTimeline);
  }
  return timelines;
};

export const timelineToEventsCount = (timeline: EventTimelineReading): number =>
  timeline.getEvents().length;

export const getTimelinesEventsCount = (timelines: EventTimelineReading[]): number => {
  const timelineEventCountReducer = (count: number, timeline: EventTimelineReading) =>
    count + timelineToEventsCount(timeline);
  return timelines.reduce(timelineEventCountReducer, 0);
};

export const getTimelineAndBaseIndex = (
  timelines: EventTimelineReading[],
  index: number
): [EventTimelineReading | undefined, number] => {
  let uptoTimelineLength = 0;
  const timeline = timelines.find((candidateTimeline) => {
    uptoTimelineLength += candidateTimeline.getEvents().length;
    if (index < uptoTimelineLength) return true;
    return false;
  });
  if (!timeline) return [undefined, 0];
  return [timeline, uptoTimelineLength - timeline.getEvents().length];
};

export const getTimelineRelativeIndex = (
  absoluteIndex: number,
  timelineBaseIndex: number
): number => absoluteIndex - timelineBaseIndex;

export const getTimelineEvent = (
  timeline: EventTimelineReading,
  index: number
): TimelineEventReading | undefined => timeline.getEvents()[index];

export const getEventIdAbsoluteIndex = (
  timelines: EventTimelineReading[],
  eventTimeline: EventTimelineReading,
  eventId: string
): number | undefined => {
  const timelineIndex = timelines.findIndex((timeline) => timeline === eventTimeline);
  if (timelineIndex === -1) return undefined;
  const eventIndex = eventTimeline.getEvents().findIndex((event) => event.getId() === eventId);
  if (eventIndex === -1) return undefined;
  const baseIndex = timelines
    .slice(0, timelineIndex)
    .reduce((accValue, timeline) => timeline.getEvents().length + accValue, 0);
  return baseIndex + eventIndex;
};
