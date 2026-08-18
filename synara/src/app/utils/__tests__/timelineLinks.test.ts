import assert from 'node:assert/strict';
import test from 'node:test';
// Direction literals are the probed js-sdk values 'b'/'f'; event/room shapes are stubbed below.
import {
  type EventTimelineReading,
  type RoomReading,
  type TimelineEventReading,
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

type StubEvent = TimelineEventReading;
type StubRoom = RoomReading;
type TimelineStub = EventTimelineReading & {
  id: string;
  stubEvents: StubEvent[];
  backward?: TimelineStub;
  forward?: TimelineStub;
};

const event = (id: string): StubEvent => ({ getId: () => id } as StubEvent);

const timeline = (id: string, eventIds: string[]): TimelineStub => {
  const stub = {
    id,
    stubEvents: eventIds.map(event),
    backward: undefined as TimelineStub | undefined,
    forward: undefined as TimelineStub | undefined,
    getEvents(): StubEvent[] {
      return stub.stubEvents;
    },
    getNeighbouringTimeline(direction: string): TimelineStub | null {
      return direction === 'b' ? stub.backward ?? null : stub.forward ?? null;
    },
  };
  return stub as unknown as TimelineStub;
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

  assert.equal(getFirstLinkedTimeline(live, 'b'), older);
  assert.equal(getFirstLinkedTimeline(older, 'f'), live);
  assert.deepEqual(getLinkedTimelines(middle), [older, middle, live]);
});

test('timeline link helpers handle isolated timelines', () => {
  const only = timeline('only', ['$1']);

  assert.equal(getFirstLinkedTimeline(only, 'b'), only);
  assert.equal(getFirstLinkedTimeline(only, 'f'), only);
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
  } as unknown as StubRoom;

  assert.equal(getLiveTimeline(room), live);
  assert.equal(getEventTimeline(room, '$0'), historic);
  assert.equal(getEventTimeline(room, '$missing'), undefined);
});
