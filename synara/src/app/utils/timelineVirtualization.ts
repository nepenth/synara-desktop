import { inSameDay, minuteDifference } from './time';

export type TimelineVirtualRowKind = 'intro' | 'loader' | 'divider' | 'event' | 'bottom';

export type TimelineVirtualRow = {
  key: string;
  kind: TimelineVirtualRowKind;
  eventId?: string;
  eventIndex?: number;
  direction?: 'backward' | 'forward';
  observe?: boolean;
};

export type TimelineVirtualAnchor = {
  eventId: string;
  offsetTop: number;
};

export type TimelineVirtualRange = {
  startIndex: number;
  endIndex: number;
};

export const TIMELINE_VIRTUAL_OVERSCAN = 12;
export const TIMELINE_PAGINATION_THRESHOLD = 12;
export const TIMELINE_MAX_EXPECTED_RENDERED_ROWS = 200;

export const getTimelineRowKey = (row: TimelineVirtualRow): string => {
  if (row.kind === 'event' && row.eventId) return `event:${row.eventId}`;
  return row.key;
};

export const estimateTimelineRowSize = (
  row: TimelineVirtualRow,
  compact: boolean,
  fallbackSize = 96
): number => {
  if (row.kind === 'intro') return 260;
  if (row.kind === 'divider') return 38;
  if (row.kind === 'loader') return compact ? 34 : 76;
  if (row.kind === 'bottom') return 1;
  return compact ? 42 : fallbackSize;
};

export const getVirtualAnchorOffset = (
  scrollViewportTop: number,
  eventElementTop: number
): number => eventElementTop - scrollViewportTop;

export const getRestoredVirtualScrollTop = (
  currentScrollTop: number,
  anchor: TimelineVirtualAnchor,
  scrollViewportTop: number,
  eventElementTop: number
): number => currentScrollTop + eventElementTop - scrollViewportTop - anchor.offsetTop;

export const shouldPaginateVirtualRange = (
  range: TimelineVirtualRange | undefined,
  rows: TimelineVirtualRow[],
  eventsLength: number,
  threshold = TIMELINE_PAGINATION_THRESHOLD
): { backward: boolean; forward: boolean } => {
  if (!range || rows.length === 0 || eventsLength === 0) {
    return { backward: false, forward: false };
  }

  const renderedRows = rows.slice(range.startIndex, range.endIndex + 1);
  const eventIndexes = renderedRows
    .map((row) => row.eventIndex)
    .filter((index): index is number => typeof index === 'number');
  const observedLoaderDirections = renderedRows
    .filter((row) => row.kind === 'loader' && row.observe)
    .map((row) => row.direction);

  if (eventIndexes.length === 0) {
    return {
      backward: observedLoaderDirections.includes('backward'),
      forward: observedLoaderDirections.includes('forward'),
    };
  }

  return {
    backward:
      observedLoaderDirections.includes('backward') || Math.min(...eventIndexes) <= threshold,
    forward:
      observedLoaderDirections.includes('forward') ||
      Math.max(...eventIndexes) >= eventsLength - threshold - 1,
  };
};

export const isVirtualRangeAtEnd = (
  range: TimelineVirtualRange | undefined,
  rowsLength: number
): boolean =>
  rowsLength > 0 && typeof range?.endIndex === 'number' && range.endIndex >= rowsLength - 1;

export type TimelineRowBuildEvent = {
  getId(): string | undefined;
  getSender(): string | undefined;
  getTs(): number;
  getType(): string;
  isRedacted(): boolean;
};

export type TimelineRowBuildTimeline<TEvt extends TimelineRowBuildEvent> = {
  getEvents(): TEvt[];
};

export type TimelineLoaderRow = TimelineVirtualRow & {
  kind: 'loader';
  direction: 'backward' | 'forward';
  observe: boolean;
  placeholderIndex: number;
};

export type TimelineDividerRow = TimelineVirtualRow & {
  kind: 'divider';
  divider: 'server-unread' | 'client-unread' | 'day';
  ts?: number;
};

export type TimelineIntroRow = TimelineVirtualRow & {
  kind: 'intro';
};

export type TimelineBottomRow = TimelineVirtualRow & {
  kind: 'bottom';
};

export type TimelineBuildRow =
  | TimelineVirtualRow
  | TimelineLoaderRow
  | TimelineDividerRow
  | TimelineIntroRow
  | TimelineBottomRow;

export type TimelineBuildOptions = {
  showIntro: boolean;
  showBackLoader: boolean;
  showFrontLoader: boolean;
  compact: boolean;
  ignoredUsersSet: Set<string>;
  showHiddenEvents: boolean;
  readUptoEventId?: string;
  unreadAnchorEventId?: string;
  currentUserId?: string | null;
};

export type TimelineRowBuildContext<TEvt extends TimelineRowBuildEvent> = {
  prevEvent?: TEvt;
  isPrevRendered: boolean;
  pendingNewDivider: boolean;
  absoluteIndex: number;
  anchorEventId?: string;
};

export type TimelineBuildFingerprint = {
  timelineToken: string;
  eventsLength: number;
  optionsKey: string;
};

export type TimelineRowsBuildState<
  TRow extends TimelineBuildRow,
  TEvt extends TimelineRowBuildEvent = TimelineRowBuildEvent
> = {
  fingerprint: TimelineBuildFingerprint;
  rows: TRow[];
  context: TimelineRowBuildContext<TEvt>;
};

export type TimelineRowBuildInstrumentation = {
  eventsVisited: number;
  fullBuilds: number;
  incrementalBuilds: number;
  skippedBuilds: number;
};

export type TimelineRowBuildDeps<TEvt extends TimelineRowBuildEvent, TTimeline, TRow> = {
  getTimelinesEventsCount: (timelines: TTimeline[]) => number;
  isReactionOrEditEvent: (event: TEvt) => boolean;
  createEventRow: (args: {
    mEvent: TEvt;
    eventId: string;
    eventIndex: number;
    eventTimeline: TTimeline;
    collapse: boolean;
  }) => TRow;
};

let timelineRowBuildInstrumentation: TimelineRowBuildInstrumentation = {
  eventsVisited: 0,
  fullBuilds: 0,
  incrementalBuilds: 0,
  skippedBuilds: 0,
};

export const resetTimelineRowBuildInstrumentation = (): void => {
  timelineRowBuildInstrumentation = {
    eventsVisited: 0,
    fullBuilds: 0,
    incrementalBuilds: 0,
    skippedBuilds: 0,
  };
};

export const getTimelineRowBuildInstrumentation = (): TimelineRowBuildInstrumentation => ({
  ...timelineRowBuildInstrumentation,
});

const recordTimelineEventVisit = (): void => {
  timelineRowBuildInstrumentation.eventsVisited += 1;
};

const getTimelineToken = <TTimeline extends TimelineRowBuildTimeline<TimelineRowBuildEvent>>(
  linkedTimelines: TTimeline[]
): string => linkedTimelines.map((timeline) => timeline.getEvents().length).join(':');

const getTimelineBuildOptionsKey = (options: TimelineBuildOptions): string =>
  [
    options.showIntro ? '1' : '0',
    options.showBackLoader ? '1' : '0',
    options.showFrontLoader ? '1' : '0',
    options.compact ? '1' : '0',
    options.showHiddenEvents ? '1' : '0',
    options.readUptoEventId ?? '',
    options.unreadAnchorEventId ?? '',
    options.currentUserId ?? '',
    [...options.ignoredUsersSet].sort().join(','),
  ].join('|');

export const createTimelineBuildFingerprint = <
  TTimeline extends TimelineRowBuildTimeline<TimelineRowBuildEvent>
>(
  linkedTimelines: TTimeline[],
  options: TimelineBuildOptions,
  getTimelinesEventsCount: (timelines: TTimeline[]) => number
): TimelineBuildFingerprint => ({
  timelineToken: getTimelineToken(linkedTimelines),
  eventsLength: getTimelinesEventsCount(linkedTimelines),
  optionsKey: getTimelineBuildOptionsKey(options),
});

const isLiveEndAppendToken = (previousToken: string, nextToken: string): boolean => {
  const previousCounts = previousToken.split(':').map((value) => Number(value));
  const nextCounts = nextToken.split(':').map((value) => Number(value));
  if (previousCounts.length !== nextCounts.length || previousCounts.length === 0) return false;

  for (let index = 0; index < previousCounts.length - 1; index += 1) {
    if (previousCounts[index] !== nextCounts[index]) return false;
  }

  return nextCounts[nextCounts.length - 1] > previousCounts[previousCounts.length - 1];
};

const getLoaderRows = (
  direction: 'backward' | 'forward',
  compact: boolean,
  count: number
): TimelineLoaderRow[] =>
  Array.from({ length: compact ? 5 : 3 }, (_, index) => ({
    kind: 'loader',
    key: `loader:${direction}:${count}:${index}`,
    direction,
    observe: direction === 'backward' ? index === (compact ? 4 : 2) : index === 0,
    placeholderIndex: index,
  }));

const getTrailingSyntheticRowCount = (rows: TimelineBuildRow[]): number => {
  let trailingRows = 0;
  for (let index = rows.length - 1; index >= 0; index -= 1) {
    const row = rows[index];
    if (row.kind === 'bottom' || row.kind === 'loader') {
      trailingRows += 1;
      continue;
    }
    break;
  }
  return trailingRows;
};

const getEventAtAbsoluteIndex = <
  TEvt extends TimelineRowBuildEvent,
  TTimeline extends TimelineRowBuildTimeline<TEvt>
>(
  linkedTimelines: TTimeline[],
  absoluteIndex: number
): TEvt | undefined => {
  let currentIndex = 0;
  for (const timeline of linkedTimelines) {
    const events = timeline.getEvents();
    if (absoluteIndex < currentIndex + events.length) {
      return events[absoluteIndex - currentIndex];
    }
    currentIndex += events.length;
  }
  return undefined;
};

const processTimelineEvent = <
  TEvt extends TimelineRowBuildEvent,
  TTimeline extends TimelineRowBuildTimeline<TEvt>,
  TRow extends TimelineBuildRow
>(args: {
  mEvent: TEvt;
  eventIndex: number;
  eventTimeline: TTimeline;
  options: TimelineBuildOptions;
  context: TimelineRowBuildContext<TEvt>;
  rows: TRow[];
  deps: TimelineRowBuildDeps<TEvt, TTimeline, TRow>;
}): TimelineRowBuildContext<TEvt> => {
  const { mEvent, eventIndex, eventTimeline, options, rows, deps } = args;
  const { prevEvent, isPrevRendered } = args.context;
  let { pendingNewDivider } = args.context;
  const { ignoredUsersSet, showHiddenEvents, readUptoEventId, unreadAnchorEventId, currentUserId } =
    options;

  const eventId = mEvent.getId();
  if (!eventId) {
    return {
      prevEvent: mEvent,
      isPrevRendered,
      pendingNewDivider,
      absoluteIndex: eventIndex + 1,
      anchorEventId: args.context.anchorEventId,
    };
  }

  const eventSender = mEvent.getSender();
  if (eventSender && ignoredUsersSet.has(eventSender)) {
    return {
      prevEvent: mEvent,
      isPrevRendered: false,
      pendingNewDivider,
      absoluteIndex: eventIndex + 1,
      anchorEventId: args.context.anchorEventId,
    };
  }
  if (mEvent.isRedacted() && !showHiddenEvents) {
    return {
      prevEvent: mEvent,
      isPrevRendered: false,
      pendingNewDivider,
      absoluteIndex: eventIndex + 1,
      anchorEventId: args.context.anchorEventId,
    };
  }

  if (!pendingNewDivider && readUptoEventId && prevEvent?.getId() === readUptoEventId) {
    pendingNewDivider = true;
  }
  const dayDividerTs =
    prevEvent && !inSameDay(prevEvent.getTs(), mEvent.getTs()) ? mEvent.getTs() : undefined;

  const collapsed =
    isPrevRendered &&
    !dayDividerTs &&
    (!pendingNewDivider || eventSender === currentUserId) &&
    prevEvent !== undefined &&
    prevEvent.getSender() === eventSender &&
    prevEvent.getType() === mEvent.getType() &&
    minuteDifference(prevEvent.getTs(), mEvent.getTs()) < 2;

  const renderable = !deps.isReactionOrEditEvent(mEvent);
  if (renderable) {
    if (pendingNewDivider && eventSender !== currentUserId) {
      rows.push({
        kind: 'divider',
        key: `divider:unread:${eventId}`,
        divider: readUptoEventId === unreadAnchorEventId ? 'client-unread' : 'server-unread',
      } as TRow);
      pendingNewDivider = false;
    }
    if (dayDividerTs) {
      rows.push({
        kind: 'divider',
        key: `divider:day:${eventId}`,
        divider: 'day',
        ts: dayDividerTs,
      } as TRow);
    }
    rows.push(
      deps.createEventRow({
        mEvent,
        eventId,
        eventIndex,
        eventTimeline,
        collapse: collapsed,
      })
    );
  }

  return {
    prevEvent: mEvent,
    isPrevRendered: renderable,
    pendingNewDivider,
    absoluteIndex: eventIndex + 1,
    anchorEventId: eventId,
  };
};

const appendTimelineRowsFromIndex = <
  TEvt extends TimelineRowBuildEvent,
  TTimeline extends TimelineRowBuildTimeline<TEvt>,
  TRow extends TimelineBuildRow
>(args: {
  linkedTimelines: TTimeline[];
  options: TimelineBuildOptions;
  rows: TRow[];
  context: TimelineRowBuildContext<TEvt>;
  startAbsoluteIndex: number;
  deps: TimelineRowBuildDeps<TEvt, TTimeline, TRow>;
}): TimelineRowBuildContext<TEvt> => {
  const { linkedTimelines, options, rows, deps, startAbsoluteIndex } = args;
  let context = args.context;
  let absoluteIndex = 0;

  linkedTimelines.forEach((eventTimeline) => {
    eventTimeline.getEvents().forEach((mEvent) => {
      if (absoluteIndex < startAbsoluteIndex) {
        absoluteIndex += 1;
        return;
      }

      recordTimelineEventVisit();
      context = processTimelineEvent({
        mEvent,
        eventIndex: absoluteIndex,
        eventTimeline,
        options,
        context,
        rows,
        deps,
      });
      absoluteIndex = context.absoluteIndex;
    });
  });

  return context;
};

export const buildTimelineRows = <
  TEvt extends TimelineRowBuildEvent,
  TTimeline extends TimelineRowBuildTimeline<TEvt>,
  TRow extends TimelineBuildRow
>(
  linkedTimelines: TTimeline[],
  options: TimelineBuildOptions,
  deps: TimelineRowBuildDeps<TEvt, TTimeline, TRow>
): { rows: TRow[]; context: TimelineRowBuildContext<TEvt> } => {
  timelineRowBuildInstrumentation.fullBuilds += 1;

  const rows: TRow[] = [];
  const { showIntro, showBackLoader, showFrontLoader, compact } = options;

  if (showIntro) {
    rows.push({ kind: 'intro', key: 'intro' } as TRow);
  }
  if (showBackLoader) {
    rows.push(
      ...(getLoaderRows(
        'backward',
        compact,
        deps.getTimelinesEventsCount(linkedTimelines)
      ) as TRow[])
    );
  }

  const context = appendTimelineRowsFromIndex({
    linkedTimelines,
    options,
    rows,
    context: {
      isPrevRendered: false,
      pendingNewDivider: false,
      absoluteIndex: 0,
    },
    startAbsoluteIndex: 0,
    deps,
  });

  if (showFrontLoader) {
    rows.push(
      ...(getLoaderRows(
        'forward',
        compact,
        deps.getTimelinesEventsCount(linkedTimelines)
      ) as TRow[])
    );
  }

  rows.push({ kind: 'bottom', key: 'bottom' } as TRow);

  return { rows, context };
};

const canIncrementallyAppendRows = <
  TRow extends TimelineBuildRow,
  TEvt extends TimelineRowBuildEvent
>(
  previous: TimelineRowsBuildState<TRow, TEvt>,
  fingerprint: TimelineBuildFingerprint
): boolean => {
  if (previous.fingerprint.optionsKey !== fingerprint.optionsKey) return false;
  if (fingerprint.eventsLength < previous.fingerprint.eventsLength) return false;
  if (fingerprint.eventsLength === previous.fingerprint.eventsLength) return true;
  return isLiveEndAppendToken(previous.fingerprint.timelineToken, fingerprint.timelineToken);
};

const verifyIncrementalAnchor = <
  TTimeline extends TimelineRowBuildTimeline<TimelineRowBuildEvent>,
  TRow extends TimelineBuildRow,
  TEvt extends TimelineRowBuildEvent
>(
  linkedTimelines: TTimeline[],
  previous: TimelineRowsBuildState<TRow, TEvt>
): boolean => {
  const anchorEventId = previous.context.anchorEventId;
  if (!anchorEventId) return true;

  const anchorEvent = getEventAtAbsoluteIndex(
    linkedTimelines,
    previous.fingerprint.eventsLength - 1
  );
  return anchorEvent?.getId() === anchorEventId;
};

export type TimelineRowsBuildResult<
  TRow extends TimelineBuildRow,
  TEvt extends TimelineRowBuildEvent = TimelineRowBuildEvent
> = {
  rows: TRow[];
  state: TimelineRowsBuildState<TRow, TEvt>;
  strategy: 'full' | 'incremental' | 'skipped';
};

export const buildTimelineRowsWithState = <
  TEvt extends TimelineRowBuildEvent,
  TTimeline extends TimelineRowBuildTimeline<TEvt>,
  TRow extends TimelineBuildRow
>(
  linkedTimelines: TTimeline[],
  options: TimelineBuildOptions,
  deps: TimelineRowBuildDeps<TEvt, TTimeline, TRow>,
  previous?: TimelineRowsBuildState<TRow, TEvt>
): TimelineRowsBuildResult<TRow, TEvt> => {
  const fingerprint = createTimelineBuildFingerprint(
    linkedTimelines,
    options,
    deps.getTimelinesEventsCount
  );

  if (
    previous &&
    canIncrementallyAppendRows(previous, fingerprint) &&
    fingerprint.eventsLength === previous.fingerprint.eventsLength
  ) {
    timelineRowBuildInstrumentation.skippedBuilds += 1;
    return {
      rows: previous.rows,
      state: {
        fingerprint,
        rows: previous.rows,
        context: previous.context,
      },
      strategy: 'skipped',
    };
  }

  if (
    previous &&
    canIncrementallyAppendRows(previous, fingerprint) &&
    fingerprint.eventsLength > previous.fingerprint.eventsLength &&
    verifyIncrementalAnchor(linkedTimelines, previous)
  ) {
    timelineRowBuildInstrumentation.incrementalBuilds += 1;

    const trailingRows = getTrailingSyntheticRowCount(previous.rows);
    const rows = previous.rows.slice(0, previous.rows.length - trailingRows) as TRow[];
    const context = appendTimelineRowsFromIndex({
      linkedTimelines,
      options,
      rows,
      context: previous.context,
      startAbsoluteIndex: previous.fingerprint.eventsLength,
      deps,
    });

    if (options.showFrontLoader) {
      rows.push(
        ...(getLoaderRows(
          'forward',
          options.compact,
          deps.getTimelinesEventsCount(linkedTimelines)
        ) as TRow[])
      );
    }
    rows.push({ kind: 'bottom', key: 'bottom' } as TRow);

    return {
      rows,
      state: {
        fingerprint,
        rows,
        context,
      },
      strategy: 'incremental',
    };
  }

  const { rows, context } = buildTimelineRows(linkedTimelines, options, deps);

  return {
    rows,
    state: {
      fingerprint,
      rows,
      context,
    },
    strategy: 'full',
  };
};
