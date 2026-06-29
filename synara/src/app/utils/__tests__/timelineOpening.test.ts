import assert from 'node:assert/strict';
import test from 'node:test';
import { Direction, type EventTimeline, type MatrixEvent, type Room } from 'matrix-js-sdk';
import {
  getEmptyTimeline,
  getInitialTimeline,
  getRoomUnreadInfo,
  getTimelineEndWindow,
  hasUnreadForInitialScroll,
  timelineHasEvents,
} from '../timelineOpening';

type TimelineStub = EventTimeline & {
  id: string;
  events: MatrixEvent[];
  backward?: TimelineStub;
  forward?: TimelineStub;
};

const event = (id: string): MatrixEvent =>
  ({
    getId: () => id,
  } as MatrixEvent);

const timeline = (id: string, eventIds: string[]): TimelineStub => {
  const stub = {
    id,
    events: eventIds.map(event),
    getEvents() {
      return this.events;
    },
    getNeighbouringTimeline(direction: Direction) {
      return direction === Direction.Backward ? this.backward ?? null : this.forward ?? null;
    },
  };
  return stub as TimelineStub;
};

const link = (...timelines: TimelineStub[]): TimelineStub[] => {
  timelines.forEach((current, index) => {
    current.backward = timelines[index - 1];
    current.forward = timelines[index + 1];
  });
  return timelines;
};

const roomWithTimelines = ({
  live,
  eventTimelines = {},
  readUpTo,
  userId = '@alice:example.org',
}: {
  live: TimelineStub;
  eventTimelines?: Record<string, TimelineStub | null>;
  readUpTo?: string;
  userId?: string;
}): Room =>
  ({
    client: {
      getUserId: () => userId,
    },
    getEventReadUpTo: () => readUpTo,
    getLiveTimeline: () => live,
    getUnfilteredTimelineSet: () => ({
      getLiveTimeline: () => live,
      getTimelineForEvent: (eventId: string) => eventTimelines[eventId] ?? null,
    }),
  } as Room);

test('timeline opening helpers create bounded live-end windows', () => {
  const [older, live] = link(timeline('older', ['$1', '$2']), timeline('live', ['$3', '$4']));

  assert.deepEqual(getTimelineEndWindow([older, live], 3), {
    linkedTimelines: [older, live],
    range: { start: 1, end: 4 },
  });
  assert.deepEqual(getTimelineEndWindow([older, live], 10), {
    linkedTimelines: [older, live],
    range: { start: 0, end: 4 },
  });
  assert.deepEqual(getInitialTimeline(roomWithTimelines({ live }), 3), {
    linkedTimelines: [older, live],
    range: { start: 1, end: 4 },
  });
});

test('timeline opening helpers represent empty and non-empty timelines', () => {
  const emptyTimeline = getEmptyTimeline();
  const nonEmptyTimeline = getTimelineEndWindow([timeline('live', ['$1'])], 80);

  assert.deepEqual(emptyTimeline, { range: { start: 0, end: 0 }, linkedTimelines: [] });
  assert.equal(timelineHasEvents(emptyTimeline), false);
  assert.equal(timelineHasEvents(nonEmptyTimeline), true);
});

test('room unread info resolves explicit anchors and read receipts', () => {
  const [older, live] = link(timeline('older', ['$1']), timeline('live', ['$2']));
  const room = roomWithTimelines({
    live,
    readUpTo: '$1',
    eventTimelines: {
      $1: older,
      $2: live,
    },
  });

  assert.deepEqual(getRoomUnreadInfo(room, undefined, true), {
    readUptoEventId: '$1',
    inLiveTimeline: true,
    scrollTo: true,
  });
  assert.deepEqual(getRoomUnreadInfo(room, '$2'), {
    readUptoEventId: '$2',
    inLiveTimeline: true,
    scrollTo: false,
  });
});

test('room unread info tolerates missing read markers and detached timeline lookups', () => {
  const live = timeline('live', ['$2']);
  const detached = timeline('detached', ['$1']);
  const room = roomWithTimelines({
    live,
    eventTimelines: {
      $1: detached,
    },
  });

  assert.equal(getRoomUnreadInfo(room), undefined);
  assert.deepEqual(getRoomUnreadInfo(room, '$1'), {
    readUptoEventId: '$1',
    inLiveTimeline: false,
    scrollTo: false,
  });
});

test('initial unread detection accepts unread counts and explicit anchors', () => {
  assert.equal(hasUnreadForInitialScroll(undefined), false);
  assert.equal(hasUnreadForInitialScroll({ total: 0, highlight: 0, from: null }), false);
  assert.equal(hasUnreadForInitialScroll({ total: 1, highlight: 0, from: null }), true);
  assert.equal(hasUnreadForInitialScroll({ total: 0, highlight: 1, from: null }), true);
  assert.equal(hasUnreadForInitialScroll(undefined, '$anchor'), true);
});
