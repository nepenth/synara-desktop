import assert from 'node:assert/strict';
import test from 'node:test';
import { Direction, type EventTimeline, type MatrixEvent, type Room } from 'matrix-js-sdk';
import {
  buildRoomTimelineOpenDiagnostics,
  getEmptyTimeline,
  getInitialTimeline,
  getRoomTimelineOpenMode,
  getRoomUnreadInfo,
  getRoomUnreadInfoInTimelineWindow,
  getTimelineEndWindow,
  getTimelineFocusRange,
  getTimelineRangeAfterPagination,
  hasUnreadForInitialScroll,
  canRestoreViewportFromInitialTimeline,
  shouldGateViewportRestoreOnUnread,
  shouldShowJumpToUnread,
  timelineWindowContainsEventId,
  timelineHasEvents,
} from '../timelineOpening';

type TimelineStub = EventTimeline & {
  id: string;
  stubEvents: MatrixEvent[];
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
    stubEvents: eventIds.map(event),
    backward: undefined as TimelineStub | undefined,
    forward: undefined as TimelineStub | undefined,
    getEvents(): MatrixEvent[] {
      return stub.stubEvents;
    },
    getNeighbouringTimeline(direction: Direction): EventTimeline | null {
      return direction === Direction.Backward ? stub.backward ?? null : stub.forward ?? null;
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
  } as unknown as Room);

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

test('backward pagination keeps the newly exposed history edge when the range is capped', () => {
  assert.deepEqual(
    getTimelineRangeAfterPagination({
      currentRange: { start: 4_850, end: 5_050 },
      totalEvents: 5_100,
      offsetRange: 50,
      backwards: true,
      limit: 80,
      maxRows: 200,
    }),
    { start: 4_820, end: 5_020 }
  );
});

test('forward pagination advances a capped range toward newer history', () => {
  assert.deepEqual(
    getTimelineRangeAfterPagination({
      currentRange: { start: 4_820, end: 5_020 },
      totalEvents: 5_100,
      offsetRange: 0,
      backwards: false,
      limit: 80,
      maxRows: 200,
    }),
    { start: 4_900, end: 5_100 }
  );
});

test('pagination offsets preserve existing event coordinates after a prepend', () => {
  const range = getTimelineRangeAfterPagination({
    currentRange: { start: 4_920, end: 5_000 },
    totalEvents: 5_050,
    offsetRange: 50,
    backwards: true,
    limit: 80,
    maxRows: 200,
  });

  assert.deepEqual(range, { start: 4_890, end: 5_050 });
  assert.ok(4_950 >= range.start && 4_950 < range.end, 'shifted visible anchor stays rendered');
});

test('focused timeline ranges keep targets rendered and clamp at both ends', () => {
  assert.deepEqual(
    getTimelineFocusRange({ targetIndex: 500, totalEvents: 1_000, contextLimit: 80, maxRows: 200 }),
    { start: 420, end: 581 }
  );
  assert.deepEqual(
    getTimelineFocusRange({ targetIndex: 2, totalEvents: 1_000, contextLimit: 80, maxRows: 100 }),
    { start: 0, end: 83 }
  );
  assert.deepEqual(
    getTimelineFocusRange({ targetIndex: 999, totalEvents: 1_000, contextLimit: 80, maxRows: 100 }),
    { start: 919, end: 1_000 }
  );
});

test('timeline opening helpers represent empty and non-empty timelines', () => {
  const emptyTimeline = getEmptyTimeline();
  const nonEmptyTimeline = getTimelineEndWindow([timeline('live', ['$1'])], 80);

  assert.deepEqual(emptyTimeline, { range: { start: 0, end: 0 }, linkedTimelines: [] });
  assert.equal(timelineHasEvents(emptyTimeline), false);
  assert.equal(timelineHasEvents(nonEmptyTimeline), true);
});

test('timeline opening restore gate only accepts anchors inside the initial window', () => {
  const [older, live] = link(timeline('older', ['$1', '$2']), timeline('live', ['$3', '$4', '$5']));
  const window = getTimelineEndWindow([older, live], 3);

  assert.equal(timelineWindowContainsEventId(window, '$1'), false);
  assert.equal(timelineWindowContainsEventId(window, '$3'), true);
  assert.equal(canRestoreViewportFromInitialTimeline(undefined, window), true);
  assert.equal(canRestoreViewportFromInitialTimeline({ atBottom: true }, window), true);
  assert.equal(
    canRestoreViewportFromInitialTimeline({ atBottom: false, anchor: { eventId: '$1' } }, window),
    false
  );
  assert.equal(
    canRestoreViewportFromInitialTimeline({ atBottom: false, anchor: { eventId: '$3' } }, window),
    true
  );
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

test('room unread info only becomes an initial placement anchor inside the live-end window', () => {
  const [older, live] = link(timeline('older', ['$1', '$2']), timeline('live', ['$3', '$4', '$5']));
  const window = getTimelineEndWindow([older, live], 3);
  const room = roomWithTimelines({
    live,
    readUpTo: '$2',
    eventTimelines: {
      $2: older,
      $3: live,
      $4: live,
    },
  });

  assert.equal(getRoomUnreadInfoInTimelineWindow(room, window), undefined);
  assert.deepEqual(getRoomUnreadInfoInTimelineWindow(room, window, '$4', true), {
    readUptoEventId: '$4',
    inLiveTimeline: true,
    scrollTo: true,
  });
});

test('initial unread detection accepts unread counts and explicit anchors', () => {
  assert.equal(hasUnreadForInitialScroll(undefined), false);
  assert.equal(hasUnreadForInitialScroll({ total: 0, highlight: 0, from: null }), false);
  assert.equal(hasUnreadForInitialScroll({ total: 1, highlight: 0, from: null }), true);
  assert.equal(hasUnreadForInitialScroll({ total: 0, highlight: 1, from: null }), true);
  assert.equal(hasUnreadForInitialScroll(undefined, '$anchor'), true);
});

test('viewport restore gate uses the actual unread signal, not live-window placement', () => {
  // Deep unread still gates historical restore even when auto-open is suppressed.
  assert.equal(shouldGateViewportRestoreOnUnread(true), true);
  assert.equal(shouldGateViewportRestoreOnUnread(false), false);
});

test('jump to unread shows for live-chain markers outside the current window', () => {
  const [older, live] = link(timeline('older', ['$1', '$2']), timeline('live', ['$3', '$4', '$5']));
  const window = getTimelineEndWindow([older, live], 3);

  // Outside live chain entirely.
  assert.equal(
    shouldShowJumpToUnread(
      { readUptoEventId: '$detached', inLiveTimeline: false, scrollTo: false },
      window
    ),
    true
  );

  // In live chain but outside the initial live-end window (v1.2.28 gap).
  assert.equal(
    shouldShowJumpToUnread(
      { readUptoEventId: '$2', inLiveTimeline: true, scrollTo: false },
      window
    ),
    true
  );

  // Already inside the rendered live-end window: no jump affordance needed.
  assert.equal(
    shouldShowJumpToUnread(
      { readUptoEventId: '$4', inLiveTimeline: true, scrollTo: false },
      window
    ),
    false
  );

  assert.equal(shouldShowJumpToUnread(undefined, window), false);
});

test('room timeline open mode prefers focused, unread window, then viewport, then live end', () => {
  assert.equal(
    getRoomTimelineOpenMode({
      focusedEventId: '$focus',
      shouldOpenAtUnread: true,
      shouldRestoreSavedViewport: true,
    }),
    'focused-event'
  );
  assert.equal(
    getRoomTimelineOpenMode({
      shouldOpenAtUnread: true,
      shouldRestoreSavedViewport: true,
    }),
    'unread-window'
  );
  assert.equal(
    getRoomTimelineOpenMode({
      shouldOpenAtUnread: false,
      shouldRestoreSavedViewport: true,
    }),
    'saved-viewport'
  );
  assert.equal(
    getRoomTimelineOpenMode({
      shouldOpenAtUnread: false,
      shouldRestoreSavedViewport: false,
    }),
    'live-end'
  );
});

test('room timeline open diagnostics capture unread window presence', () => {
  assert.deepEqual(
    buildRoomTimelineOpenDiagnostics({
      openMode: 'live-end',
      unreadTargetEventId: '$2',
      unreadInInitialWindow: false,
      linkedEventCount: 40,
      loadedAtEnd: true,
    }),
    {
      openMode: 'live-end',
      hasUnreadTarget: true,
      unreadInInitialWindow: false,
      linkedEventCount: 40,
      loadedAtEnd: true,
    }
  );
});
