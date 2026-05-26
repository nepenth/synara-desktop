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
