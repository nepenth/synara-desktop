import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildTimelineRows,
  buildTimelineRowsWithState,
  estimateTimelineRowSize,
  getVirtualAnchorCorrection,
  getRestoredVirtualScrollTop,
  getTimelineRowBuildInstrumentation,
  getTimelineRowKey,
  getVirtualAnchorOffset,
  isVirtualRangeAtEnd,
  resetTimelineRowBuildInstrumentation,
  shouldPaginateVirtualRange,
  TimelineBuildOptions,
  TimelineBuildRow,
  TimelineDividerRow,
  TimelineRowBuildEvent,
  TimelineRowBuildTimeline,
  TimelineVirtualRow,
} from '../timelineVirtualization';

test('timeline virtual rows use stable event keys', () => {
  assert.equal(getTimelineRowKey({ kind: 'event', key: '$a', eventId: '$a' }), 'event:$a');
  assert.equal(getTimelineRowKey({ kind: 'divider', key: 'day:1' }), 'day:1');
});

test('timeline row estimates keep synthetic rows smaller than message rows', () => {
  assert.equal(estimateTimelineRowSize({ kind: 'bottom', key: 'bottom' }, false), 1);
  assert.equal(estimateTimelineRowSize({ kind: 'divider', key: 'day:1' }, false), 38);
  assert.ok(
    estimateTimelineRowSize({ kind: 'event', key: '$a', eventId: '$a' }, false) >
      estimateTimelineRowSize({ kind: 'event', key: '$a', eventId: '$a' }, true)
  );
});

test('timeline anchor offset restores viewport after prepending rows', () => {
  const anchorOffset = getVirtualAnchorOffset(100, 180);
  assert.equal(anchorOffset, 80);

  assert.equal(
    getVirtualAnchorCorrection({ eventId: '$a', offsetTop: anchorOffset }, 100, 180),
    0
  );
  assert.equal(
    getVirtualAnchorCorrection({ eventId: '$a', offsetTop: anchorOffset }, 100, 280),
    100
  );

  assert.equal(
    getRestoredVirtualScrollTop(500, { eventId: '$a', offsetTop: anchorOffset }, 100, 280),
    600
  );
});

test('virtual range pagination thresholds use visible event indexes', () => {
  const rows: TimelineVirtualRow[] = [
    { kind: 'loader', key: 'loader:back' },
    { kind: 'divider', key: 'day:1' },
    { kind: 'event', key: '$0', eventId: '$0', eventIndex: 0 },
    { kind: 'event', key: '$1', eventId: '$1', eventIndex: 1 },
    { kind: 'event', key: '$98', eventId: '$98', eventIndex: 98 },
    { kind: 'event', key: '$99', eventId: '$99', eventIndex: 99 },
    { kind: 'bottom', key: 'bottom' },
  ];

  assert.deepEqual(shouldPaginateVirtualRange({ startIndex: 0, endIndex: 3 }, rows, 100, 2), {
    backward: true,
    forward: false,
  });
  assert.deepEqual(shouldPaginateVirtualRange({ startIndex: 4, endIndex: 6 }, rows, 100, 2), {
    backward: false,
    forward: true,
  });
});

test('virtual range pagination triggers from visible loader rows', () => {
  const rows: TimelineVirtualRow[] = [
    { kind: 'loader', key: 'loader:back:0', direction: 'backward', observe: true },
    { kind: 'loader', key: 'loader:back:1', direction: 'backward', observe: false },
    { kind: 'event', key: '$0', eventId: '$0', eventIndex: 0 },
    { kind: 'loader', key: 'loader:front:0', direction: 'forward', observe: true },
  ];

  assert.deepEqual(shouldPaginateVirtualRange({ startIndex: 0, endIndex: 1 }, rows, 1, 0), {
    backward: true,
    forward: false,
  });
  assert.deepEqual(shouldPaginateVirtualRange({ startIndex: 3, endIndex: 3 }, rows, 1, 0), {
    backward: false,
    forward: true,
  });
});

test('virtual range end detection requires the rendered bottom row', () => {
  assert.equal(isVirtualRangeAtEnd(undefined, 10), false);
  assert.equal(isVirtualRangeAtEnd({ startIndex: 0, endIndex: 8 }, 10), false);
  assert.equal(isVirtualRangeAtEnd({ startIndex: 4, endIndex: 9 }, 10), true);
});

type HarnessEvent = TimelineRowBuildEvent & {
  id: string;
  sender: string;
  ts: number;
  type: string;
  redacted: boolean;
};

type HarnessTimeline = TimelineRowBuildTimeline<HarnessEvent> & {
  events: HarnessEvent[];
};

const createHarnessEvent = (
  index: number,
  overrides: Partial<HarnessEvent> = {}
): HarnessEvent => ({
  id: `$event-${index}`,
  sender: '@alice:example.org',
  ts: 1_700_000_000_000 + index * 60_000,
  type: 'm.room.message',
  redacted: false,
  getId: function getId() {
    return this.id;
  },
  getSender: function getSender() {
    return this.sender;
  },
  getTs: function getTs() {
    return this.ts;
  },
  getType: function getType() {
    return this.type;
  },
  isRedacted: function isRedacted() {
    return this.redacted;
  },
  ...overrides,
});

const createHarnessTimeline = (eventCount: number): HarnessTimeline => {
  const events = Array.from({ length: eventCount }, (_, index) => createHarnessEvent(index));
  return {
    events,
    getEvents: function getEvents() {
      return this.events;
    },
  };
};

const harnessBuildOptions: TimelineBuildOptions = {
  showIntro: false,
  showBackLoader: false,
  showFrontLoader: false,
  compact: false,
  ignoredUsersSet: new Set<string>(),
  showHiddenEvents: false,
};

const harnessBuildDeps = {
  getTimelinesEventsCount: (timelines: HarnessTimeline[]) =>
    timelines.reduce((count, timeline) => count + timeline.getEvents().length, 0),
  isReactionOrEditEvent: () => false,
  createEventRow: ({
    mEvent,
    eventId,
    eventIndex,
    collapse,
  }: {
    mEvent: HarnessEvent;
    eventId: string;
    eventIndex: number;
    eventTimeline: HarnessTimeline;
    collapse: boolean;
  }) => ({
    kind: 'event' as const,
    key: eventId,
    eventId,
    eventIndex,
    collapse,
    mEvent,
  }),
};

test('incremental timeline row build visits at most 10% of events on live append', () => {
  const eventCount = 5_000;
  const timeline = createHarnessTimeline(eventCount);

  resetTimelineRowBuildInstrumentation();
  const initial = buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps);
  const baselineInstrumentation = getTimelineRowBuildInstrumentation();

  assert.equal(initial.strategy, 'full');
  assert.equal(baselineInstrumentation.fullBuilds, 1);
  assert.equal(baselineInstrumentation.eventsVisited, eventCount);

  timeline.events.push(
    createHarnessEvent(eventCount, {
      id: `$event-${eventCount}`,
      sender: '@bob:example.org',
      ts: 1_700_000_000_000 + eventCount * 60_000,
    })
  );

  resetTimelineRowBuildInstrumentation();
  const appended = buildTimelineRowsWithState(
    [timeline],
    harnessBuildOptions,
    harnessBuildDeps,
    initial.state
  );
  const appendInstrumentation = getTimelineRowBuildInstrumentation();

  assert.equal(appended.strategy, 'incremental');
  assert.equal(appendInstrumentation.incrementalBuilds, 1);
  assert.equal(appendInstrumentation.eventsVisited, 1);
  assert.equal(appendInstrumentation.revisionTokenEventsScanned, 1);
  assert.ok(
    appendInstrumentation.eventsVisited <= baselineInstrumentation.eventsVisited * 0.1,
    `expected <= ${baselineInstrumentation.eventsVisited * 0.1} visits, got ${
      appendInstrumentation.eventsVisited
    }`
  );
  assert.equal(appended.rows.filter((row) => row.kind === 'event').length, eventCount + 1);
});

test('timeline row build respects bounded event ranges for large rooms', () => {
  const eventCount = 5_000;
  const timeline = createHarnessTimeline(eventCount);

  resetTimelineRowBuildInstrumentation();
  const built = buildTimelineRowsWithState(
    [timeline],
    {
      ...harnessBuildOptions,
      eventRange: { start: 4_920, end: 5_000 },
    },
    harnessBuildDeps
  );
  const instrumentation = getTimelineRowBuildInstrumentation();
  const eventRows = built.rows.filter((row) => row.kind === 'event');

  assert.equal(built.strategy, 'full');
  assert.equal(instrumentation.eventsVisited, 80);
  assert.equal(instrumentation.revisionTokenEventsScanned, 80);
  assert.equal(eventRows.length, 80);
  assert.equal(eventRows[0]?.eventIndex, 4_920);
  assert.equal(eventRows[eventRows.length - 1]?.eventIndex, 4_999);
});

test('bounded timeline row build does not incrementally append outside the rendered range', () => {
  const timeline = createHarnessTimeline(120);
  const options: TimelineBuildOptions = {
    ...harnessBuildOptions,
    eventRange: { start: 40, end: 80 },
  };
  const initial = buildTimelineRowsWithState([timeline], options, harnessBuildDeps);

  timeline.events.push(createHarnessEvent(120));

  resetTimelineRowBuildInstrumentation();
  const rebuilt = buildTimelineRowsWithState([timeline], options, harnessBuildDeps, initial.state);
  const instrumentation = getTimelineRowBuildInstrumentation();
  const eventRows = rebuilt.rows.filter((row) => row.kind === 'event');

  assert.equal(rebuilt.strategy, 'full');
  assert.equal(instrumentation.eventsVisited, 40);
  assert.equal(eventRows.length, 40);
  assert.equal(eventRows[0]?.eventIndex, 40);
  assert.equal(eventRows[eventRows.length - 1]?.eventIndex, 79);
});

test('bounded timeline rows preserve unread and day-divider context at the range boundary', () => {
  const timeline = createHarnessTimeline(4);
  timeline.events[1] = createHarnessEvent(1, {
    id: '$read-marker',
    sender: '@bob:example.org',
    ts: 1_700_000_000_000,
  });
  timeline.events[2] = createHarnessEvent(2, {
    sender: '@alice:example.org',
    ts: 1_700_086_400_000,
  });
  timeline.events[3] = createHarnessEvent(3, {
    sender: '@alice:example.org',
    ts: 1_700_086_460_000,
  });

  const { rows } = buildTimelineRows(
    [timeline],
    {
      ...harnessBuildOptions,
      eventRange: { start: 2, end: 4 },
      readUptoEventId: '$read-marker',
      unreadAnchorEventId: '$read-marker',
      currentUserId: '@bob:example.org',
    },
    harnessBuildDeps
  );
  const buildRows = rows as TimelineBuildRow[];

  assert.deepEqual(
    buildRows
      .filter((row): row is TimelineDividerRow => row.kind === 'divider')
      .map((row) => row.divider),
    ['client-unread', 'day']
  );
});

test('bounded timeline rows skip relation events without changing absolute message indexes', () => {
  const timeline = createHarnessTimeline(6);
  timeline.events[3] = createHarnessEvent(3, { type: 'm.reaction' });
  timeline.events[4] = createHarnessEvent(4, { type: 'm.room.message.edit' });
  const deps = {
    ...harnessBuildDeps,
    isReactionOrEditEvent: (item: HarnessEvent) =>
      item.type === 'm.reaction' || item.type === 'm.room.message.edit',
  };

  const { rows } = buildTimelineRows(
    [timeline],
    { ...harnessBuildOptions, eventRange: { start: 3, end: 6 } },
    deps
  );
  const eventRows = rows.filter((row) => row.kind === 'event');

  assert.deepEqual(
    eventRows.map((row) => row.eventIndex),
    [5]
  );
  assert.deepEqual(
    eventRows.map((row) => row.eventId),
    ['$event-5']
  );
});

test('first message in a bounded range never collapses into an unrendered predecessor', () => {
  const timeline = createHarnessTimeline(3);
  const { rows } = buildTimelineRows(
    [timeline],
    { ...harnessBuildOptions, eventRange: { start: 1, end: 3 } },
    harnessBuildDeps
  );
  const eventRows = rows.filter((row) => row.kind === 'event');

  assert.equal(eventRows[0]?.collapse, false);
});

test('incremental timeline row build preserves row order for appended events', () => {
  const timeline = createHarnessTimeline(3);
  const { rows: initialRows, state } = buildTimelineRowsWithState(
    [timeline],
    harnessBuildOptions,
    harnessBuildDeps
  );

  timeline.events.push(createHarnessEvent(3));
  const { rows: appendedRows, strategy } = buildTimelineRowsWithState(
    [timeline],
    harnessBuildOptions,
    harnessBuildDeps,
    state
  );

  assert.equal(strategy, 'incremental');
  assert.deepEqual(
    appendedRows.filter((row) => row.kind === 'event').map((row) => row.eventId),
    initialRows
      .filter((row) => row.kind === 'event')
      .map((row) => row.eventId)
      .concat(`$event-3`)
  );
});

test('timeline row build rebuilds when same-length events mutate in place', () => {
  const timeline = createHarnessTimeline(6);
  const initial = buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps);

  timeline.events[2] = createHarnessEvent(2, {
    type: 'm.room.message',
    ts: 1_700_000_000_000 + 2 * 60_000 + 1,
  });

  resetTimelineRowBuildInstrumentation();
  const rebuilt = buildTimelineRowsWithState(
    [timeline],
    harnessBuildOptions,
    harnessBuildDeps,
    initial.state
  );
  const instrumentation = getTimelineRowBuildInstrumentation();

  assert.equal(rebuilt.strategy, 'full');
  assert.equal(instrumentation.fullBuilds, 1);
  assert.equal(instrumentation.skippedBuilds, 0);
  assert.notEqual(rebuilt.rows, initial.rows);
});

test('incremental timeline row build skips work for no-op refresh', () => {
  const timeline = createHarnessTimeline(12);
  const initial = buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps);

  resetTimelineRowBuildInstrumentation();
  const refreshed = buildTimelineRowsWithState(
    [timeline],
    harnessBuildOptions,
    harnessBuildDeps,
    initial.state
  );
  const instrumentation = getTimelineRowBuildInstrumentation();

  assert.equal(refreshed.strategy, 'skipped');
  assert.equal(instrumentation.skippedBuilds, 1);
  assert.equal(instrumentation.eventsVisited, 0);
  assert.equal(refreshed.rows, initial.rows);
});

test('timeline row build falls back to full scan when options change', () => {
  const timeline = createHarnessTimeline(8);
  const initial = buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps);

  resetTimelineRowBuildInstrumentation();
  const rebuilt = buildTimelineRowsWithState(
    [timeline],
    { ...harnessBuildOptions, showIntro: true },
    harnessBuildDeps,
    initial.state
  );
  const instrumentation = getTimelineRowBuildInstrumentation();

  assert.equal(rebuilt.strategy, 'full');
  assert.equal(instrumentation.fullBuilds, 1);
  assert.equal(instrumentation.eventsVisited, 8);
  assert.equal(rebuilt.rows[0]?.kind, 'intro');
});

test('ignored events do not shift day divider comparison to the next visible row', () => {
  const timeline = createHarnessTimeline(2);
  timeline.events[0] = createHarnessEvent(0, {
    sender: '@bob:example.org',
    ts: 1_700_000_000_000,
  });
  timeline.events[1] = createHarnessEvent(1, {
    sender: '@alice:example.org',
    ts: 1_700_086_400_000,
  });

  const ignoredOptions: TimelineBuildOptions = {
    ...harnessBuildOptions,
    ignoredUsersSet: new Set(['@bob:example.org']),
  };

  const { rows } = buildTimelineRows([timeline], ignoredOptions, harnessBuildDeps);
  const buildRows = rows as TimelineBuildRow[];
  assert.equal(
    buildRows.filter((row) => row.kind === 'divider').length,
    0,
    'ignored events must not introduce day dividers before the next visible row'
  );
});

test('incremental revision fingerprint matches full rebuild after live append', () => {
  const timeline = createHarnessTimeline(4);
  const initial = buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps);

  timeline.events.push(createHarnessEvent(4));
  const incremental = buildTimelineRowsWithState(
    [timeline],
    harnessBuildOptions,
    harnessBuildDeps,
    initial.state
  );
  const full = buildTimelineRows([timeline], harnessBuildOptions, harnessBuildDeps);

  assert.equal(incremental.strategy, 'incremental');
  assert.equal(
    incremental.state.fingerprint.revisionToken,
    buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps).state.fingerprint
      .revisionToken
  );
  assert.deepEqual(
    incremental.rows.filter((row) => row.kind === 'event').map((row) => row.eventId),
    full.rows.filter((row) => row.kind === 'event').map((row) => row.eventId)
  );
});

test('buildTimelineRows full scan matches incremental baseline for same fixture', () => {
  const timeline = createHarnessTimeline(120);
  const full = buildTimelineRows([timeline], harnessBuildOptions, harnessBuildDeps);
  const incremental = buildTimelineRowsWithState([timeline], harnessBuildOptions, harnessBuildDeps);

  assert.deepEqual(
    full.rows
      .filter((row) => row.kind === 'event')
      .map((row) => ({ eventId: row.eventId, eventIndex: row.eventIndex, collapse: row.collapse })),
    incremental.rows
      .filter((row) => row.kind === 'event')
      .map((row) => ({ eventId: row.eventId, eventIndex: row.eventIndex, collapse: row.collapse }))
  );
});
