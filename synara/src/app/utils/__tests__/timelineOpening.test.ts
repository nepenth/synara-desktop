import assert from 'node:assert/strict';
import test from 'node:test';
import { Direction, type EventTimeline, type MatrixEvent, type Room } from 'matrix-js-sdk';
import { EventType } from 'matrix-js-sdk/lib/@types/event';
import { ReceiptType, type WrappedReceipt } from 'matrix-js-sdk/lib/@types/read_receipts';
import {
  buildRoomTimelineOpenDiagnostics,
  canReplaceTimelineWindowPreservingAnchor,
  getEmptyTimeline,
  getInitialTimeline,
  getRoomTimelineOpenMode,
  getRoomReadFrontierRevisionKey,
  getRoomUnreadInfo,
  getRoomUnreadInfoInTimelineWindow,
  getTimelineEndWindow,
  getTimelineFocusRange,
  getTimelineRangeAfterPagination,
  getTimelineWindowTailEvent,
  getTimelineWindowTailEventId,
  getTimelineWindowIdentitySnapshot,
  hasUnreadForInitialScroll,
  LatestTimelineStructuralUpdateQueue,
  preserveTimelineStructuralUpdateAnchor,
  receiptEventContainsUser,
  resolveRoomReadFrontier,
  canRestoreViewportFromInitialTimeline,
  shouldAdoptTimelineRefresh,
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
  fullyRead,
  markedUnread = false,
  publicReceipt,
  privateReceipt,
  userId = '@alice:example.org',
}: {
  live: TimelineStub;
  eventTimelines?: Record<string, TimelineStub | null>;
  fullyRead?: string;
  markedUnread?: boolean;
  publicReceipt?: WrappedReceipt;
  privateReceipt?: WrappedReceipt;
  userId?: string;
}): Room => {
  const linkedTimelines: TimelineStub[] = [];
  let first: TimelineStub = live;
  while (first.backward) first = first.backward;
  let current: TimelineStub | undefined = first;
  while (current) {
    linkedTimelines.push(current);
    current = current.forward;
  }
  const linkedEvents = linkedTimelines.flatMap((linkedTimeline) => linkedTimeline.getEvents());
  const eventIndex = new Map(
    linkedEvents.flatMap((matrixEvent, index) => {
      const eventId = matrixEvent.getId();
      return eventId ? [[eventId, index] as const] : [];
    })
  );
  const accountData = (type: EventType | string): MatrixEvent | undefined => {
    if (type === EventType.FullyRead && fullyRead) {
      return { getContent: () => ({ event_id: fullyRead }) } as MatrixEvent;
    }
    if (type === EventType.MarkedUnread) {
      return { getContent: () => ({ unread: markedUnread }) } as MatrixEvent;
    }
    return undefined;
  };

  return {
    client: {
      getUserId: () => userId,
    },
    getAccountData: accountData,
    accountData: new Map(),
    getReadReceiptForUserId: (
      _receiptUserId: string,
      _ignoreSynthesized: boolean,
      receiptType: ReceiptType
    ) => (receiptType === ReceiptType.ReadPrivate ? privateReceipt : publicReceipt) ?? null,
    compareEventOrdering: (leftEventId: string, rightEventId: string) => {
      const leftIndex = eventIndex.get(leftEventId);
      const rightIndex = eventIndex.get(rightEventId);
      if (leftIndex === undefined || rightIndex === undefined) return null;
      return Math.sign(leftIndex - rightIndex);
    },
    findEventById: (eventId: string) => linkedEvents.find((evt) => evt.getId() === eventId),
    getLiveTimeline: () => live,
    getUnfilteredTimelineSet: () => ({
      getLiveTimeline: () => live,
      getTimelineForEvent: (eventId: string) => eventTimelines[eventId] ?? null,
    }),
  } as unknown as Room;
};

const receipt = (eventId: string, ts: number): WrappedReceipt => ({
  eventId,
  data: { ts },
});

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

test('server-latest detached context windows remain authoritative at their fetched tail', () => {
  const detachedContext = timeline('detached-context', ['$before', '$latest']);
  const latestWindow = getTimelineEndWindow([detachedContext], 80);

  assert.equal(getTimelineWindowTailEvent(latestWindow)?.getId(), '$latest');
  assert.equal(getTimelineWindowTailEventId(latestWindow), '$latest');
  assert.equal(getTimelineWindowTailEventId(getEmptyTimeline()), undefined);
  assert.equal(getTimelineWindowTailEvent(getEmptyTimeline()), undefined);
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
    publicReceipt: receipt('$1', 1),
    markedUnread: true,
    eventTimelines: {
      $1: older,
      $2: live,
    },
  });

  assert.deepEqual(getRoomUnreadInfo(room, resolveRoomReadFrontier(room), true), {
    readUptoEventId: '$1',
    inLiveTimeline: true,
    scrollTo: true,
  });
  assert.deepEqual(getRoomUnreadInfo(room, resolveRoomReadFrontier(room, '$2')), {
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
  const markedRoom = roomWithTimelines({
    live,
    markedUnread: true,
    eventTimelines: { $1: detached },
  });
  assert.deepEqual(getRoomUnreadInfo(markedRoom, resolveRoomReadFrontier(markedRoom, '$1')), {
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
    publicReceipt: receipt('$2', 2),
    markedUnread: true,
    eventTimelines: {
      $2: older,
      $3: live,
      $4: live,
    },
  });

  assert.equal(getRoomUnreadInfoInTimelineWindow(room, window), undefined);
  assert.deepEqual(
    getRoomUnreadInfoInTimelineWindow(room, window, resolveRoomReadFrontier(room, '$4'), true),
    {
      readUptoEventId: '$4',
      inLiveTimeline: true,
      scrollTo: true,
    }
  );
});

test('initial unread detection accepts unread counts and explicit anchors', () => {
  const unreadFrontier = {
    eventId: '$anchor',
    source: 'marked-unread-anchor' as const,
    isExplicitlyMarkedUnread: true,
    isAtLiveTail: false,
  };
  const normalFrontier = {
    source: 'absent' as const,
    isExplicitlyMarkedUnread: false,
    isAtLiveTail: false,
  };
  assert.equal(hasUnreadForInitialScroll(undefined, normalFrontier), false);
  assert.equal(
    hasUnreadForInitialScroll({ total: 0, highlight: 0, from: null }, normalFrontier),
    false
  );
  assert.equal(
    hasUnreadForInitialScroll({ total: 1, highlight: 0, from: null }, normalFrontier),
    true
  );
  assert.equal(
    hasUnreadForInitialScroll({ total: 0, highlight: 1, from: null }, normalFrontier),
    true
  );
  assert.equal(hasUnreadForInitialScroll(undefined, unreadFrontier), true);
});

test('read frontier chooses the newest SDK-ordered fully-read or real receipt position', () => {
  const live = timeline('live', ['$1', '$2', '$3', '$4', '$5']);
  const receiptAhead = roomWithTimelines({
    live,
    fullyRead: '$2',
    publicReceipt: receipt('$3', 30),
    privateReceipt: receipt('$4', 40),
  });
  assert.deepEqual(resolveRoomReadFrontier(receiptAhead), {
    eventId: '$4',
    source: 'private-receipt',
    isExplicitlyMarkedUnread: false,
    isAtLiveTail: false,
  });

  const fullyReadAhead = roomWithTimelines({
    live,
    fullyRead: '$4',
    publicReceipt: receipt('$2', 50),
  });
  assert.deepEqual(resolveRoomReadFrontier(fullyReadAhead), {
    eventId: '$4',
    source: 'fully-read',
    isExplicitlyMarkedUnread: false,
    isAtLiveTail: false,
  });
});

test('read frontier ignores stale Synara anchors unless the room is explicitly marked unread', () => {
  const live = timeline('live', ['$old-anchor', '$tail']);
  const normalRoom = roomWithTimelines({
    live,
    publicReceipt: receipt('$tail', 100),
  });
  const normalFrontier = resolveRoomReadFrontier(normalRoom, '$old-anchor');
  assert.deepEqual(normalFrontier, {
    eventId: '$tail',
    source: 'public-receipt',
    isExplicitlyMarkedUnread: false,
    isAtLiveTail: true,
  });
  assert.equal(
    hasUnreadForInitialScroll({ total: 17, highlight: 2, from: null }, normalFrontier),
    false,
    'stale unread counters do not pull a current receipt away from the live end'
  );

  const markedRoom = roomWithTimelines({
    live,
    markedUnread: true,
    publicReceipt: receipt('$tail', 100),
  });
  assert.deepEqual(resolveRoomReadFrontier(markedRoom, '$old-anchor'), {
    eventId: '$old-anchor',
    source: 'marked-unread-anchor',
    isExplicitlyMarkedUnread: true,
    isAtLiveTail: false,
  });
});

test('unorderable fully-read markers yield to durable receipts without using event timestamps', () => {
  const live = timeline('live', ['$tail']);
  const room = roomWithTimelines({
    live,
    fullyRead: '$unloaded-fully-read',
    publicReceipt: receipt('$tail', 100),
  });

  assert.deepEqual(resolveRoomReadFrontier(room), {
    eventId: '$tail',
    source: 'public-receipt',
    isExplicitlyMarkedUnread: false,
    isAtLiveTail: true,
  });
});

test('timeline refresh identity includes provider and tail event ID', () => {
  const provider = timeline('live', ['$old-tail']);
  const currentWindow = getTimelineEndWindow([provider], 80);
  const currentIdentity = getTimelineWindowIdentitySnapshot(currentWindow);

  assert.equal(shouldAdoptTimelineRefresh(currentIdentity, currentWindow), false);

  provider.stubEvents = [event('$new-tail')];
  const replacedTailWindow = getTimelineEndWindow([provider], 80);
  assert.equal(
    shouldAdoptTimelineRefresh(currentIdentity, replacedTailWindow),
    true,
    'equal count/range with a new event ID must refresh'
  );

  const replacementProvider = timeline('replacement', ['$old-tail']);
  assert.equal(
    shouldAdoptTimelineRefresh(currentIdentity, getTimelineEndWindow([replacementProvider], 80)),
    true,
    'provider replacement must refresh even if rows look identical'
  );
});

test('structural update queue coalesces to the latest anchored update', () => {
  type Update = { id: number; preserveAnchor: boolean };
  const queue = new LatestTimelineStructuralUpdateQueue<Update>();
  queue.enqueue(preserveTimelineStructuralUpdateAnchor({ id: 1, preserveAnchor: false }));
  queue.enqueue(preserveTimelineStructuralUpdateAnchor({ id: 2, preserveAnchor: false }));

  assert.equal(queue.hasPending(), true);
  assert.deepEqual(queue.take(), { id: 2, preserveAnchor: true });
  assert.equal(queue.take(), undefined);
  assert.equal(queue.hasPending(), false);
});

test('bounded live refresh cannot replace a viewport anchored outside the latest 80 events', () => {
  const live = timeline(
    'live',
    Array.from({ length: 100 }, (_, index) => `$event-${index}`)
  );
  const latestWindow = getTimelineEndWindow([live], 80);

  assert.equal(canReplaceTimelineWindowPreservingAnchor(latestWindow, '$event-19'), false);
  assert.equal(canReplaceTimelineWindowPreservingAnchor(latestWindow, '$event-20'), true);
  assert.equal(canReplaceTimelineWindowPreservingAnchor(latestWindow, '$event-99'), true);
});

test('receipt advancement changes the frontier revision even when unread counts are unchanged', () => {
  const live = timeline('live', ['$1', '$2', '$3']);
  const first = resolveRoomReadFrontier(
    roomWithTimelines({ live, publicReceipt: receipt('$1', 10) })
  );
  const advanced = resolveRoomReadFrontier(
    roomWithTimelines({ live, publicReceipt: receipt('$2', 20) })
  );

  assert.notEqual(getRoomReadFrontierRevisionKey(first), getRoomReadFrontierRevisionKey(advanced));

  const ownReceiptEvent = {
    getContent: () => ({
      $2: {
        [ReceiptType.Read]: {
          '@alice:example.org': { ts: 20 },
        },
      },
    }),
  } as unknown as MatrixEvent;
  assert.equal(receiptEventContainsUser(ownReceiptEvent, '@alice:example.org'), true);
  assert.equal(receiptEventContainsUser(ownReceiptEvent, '@bob:example.org'), false);
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
