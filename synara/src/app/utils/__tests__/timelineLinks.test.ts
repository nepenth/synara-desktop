import assert from 'node:assert/strict';
import test from 'node:test';
import { Direction, type EventTimeline, type MatrixEvent, type Room } from 'matrix-js-sdk';
import {
  getEventTimeline,
  getEventIdAbsoluteIndex,
  getFirstLinkedTimeline,
  getLinkedTimelines,
  getLiveTimeline,
  getTimelineAndBaseIndex,
  getTimelineEvent,
  getTimelineRelativeIndex,
  timelineToEventsCount,
  getTimelinesEventsCount,
} from '../timelineLinks';

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

test('timeline link helpers walk a linked timeline chain from any member', () => {
  const [older, middle, live] = link(
    timeline('older', ['$1', '$2']),
    timeline('middle', ['$3']),
    timeline('live', ['$4', '$5'])
  );

  assert.equal(getFirstLinkedTimeline(live, Direction.Backward), older);
  assert.equal(getFirstLinkedTimeline(older, Direction.Forward), live);
  assert.deepEqual(getLinkedTimelines(middle), [older, middle, live]);
});

test('timeline link helpers handle isolated timelines', () => {
  const only = timeline('only', ['$1']);

  assert.equal(getFirstLinkedTimeline(only, Direction.Backward), only);
  assert.equal(getFirstLinkedTimeline(only, Direction.Forward), only);
  assert.deepEqual(getLinkedTimelines(only), [only]);
  assert.equal(timelineToEventsCount(only), 1);
});

test('timeline link helpers map absolute event indexes across timelines', () => {
  const [older, middle, live] = link(
    timeline('older', ['$1', '$2']),
    timeline('middle', ['$3']),
    timeline('live', ['$4', '$5'])
  );
  const timelines = [older, middle, live];

  assert.equal(getTimelinesEventsCount(timelines), 5);
  assert.deepEqual(getTimelineAndBaseIndex(timelines, 0), [older, 0]);
  assert.deepEqual(getTimelineAndBaseIndex(timelines, 3), [live, 3]);
  assert.deepEqual(getTimelineAndBaseIndex(timelines, 4), [live, 3]);
  assert.deepEqual(getTimelineAndBaseIndex(timelines, 5), [undefined, 0]);
  assert.deepEqual(getTimelineAndBaseIndex(timelines, 99), [undefined, 0]);
  assert.equal(getTimelineRelativeIndex(4, 3), 1);
  assert.equal(getTimelineEvent(live, 1)?.getId(), '$5');
  assert.equal(getTimelineEvent(live, 99), undefined);
  assert.equal(getEventIdAbsoluteIndex(timelines, live, '$4'), 3);
  assert.equal(getEventIdAbsoluteIndex(timelines, middle, '$missing'), undefined);
  assert.equal(getEventIdAbsoluteIndex(timelines, timeline('detached', ['$6']), '$6'), undefined);
});

test('timeline link helpers use the unfiltered room timeline set wrappers', () => {
  const live = timeline('live', ['$1']);
  const historic = timeline('historic', ['$0']);
  const room = {
    getUnfilteredTimelineSet: () => ({
      getLiveTimeline: () => live,
      getTimelineForEvent: (eventId: string) => (eventId === '$0' ? historic : null),
    }),
  } as Room;

  assert.equal(getLiveTimeline(room), live);
  assert.equal(getEventTimeline(room, '$0'), historic);
  assert.equal(getEventTimeline(room, '$missing'), undefined);
});
