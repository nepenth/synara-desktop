import { timeDayMonthYear, timeHourMinute, today, yesterday } from './time';

export type TimelineDateMarkSource = {
  kind: string;
  itemId?: string;
  timestampMs?: number;
  originServerTs?: number;
  event?: { originServerTs?: number };
};

export type TimelineHistoryMark = {
  key: string;
  index: number;
  timestampMs: number;
  kind: 'day' | 'time' | 'beginning';
};

export type TimedTimelineRow = {
  index: number;
  timestampMs: number;
};

export type TimelineRailAxis = {
  startMs: number;
  endMs: number;
  loadedMinMs: number;
  loadedMaxMs: number;
  fullRoom: boolean;
};

const MAX_SAMPLED_MARKS = 6;
const INTRA_DAY_AXIS_MS = 36 * 60 * 60 * 1000;

export const rowTimestampMs = (row: TimelineDateMarkSource): number | undefined => {
  if (typeof row.timestampMs === 'number' && Number.isFinite(row.timestampMs)) {
    return row.timestampMs;
  }
  if (typeof row.originServerTs === 'number' && Number.isFinite(row.originServerTs)) {
    return row.originServerTs;
  }
  if (typeof row.event?.originServerTs === 'number' && Number.isFinite(row.event.originServerTs)) {
    return row.event.originServerTs;
  }
  return undefined;
};

const localDayKey = (timestampMs: number): string => {
  const date = new Date(timestampMs);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(
    date.getDate()
  ).padStart(2, '0')}`;
};

const sampleTimedMarks = (
  timed: readonly TimedTimelineRow[],
  kind: TimelineHistoryMark['kind']
): TimelineHistoryMark[] => {
  const sampleCount = Math.min(MAX_SAMPLED_MARKS, timed.length);
  if (sampleCount < 2) return [];
  const sampled: TimelineHistoryMark[] = [];
  const usedIndexes = new Set<number>();
  for (let step = 0; step < sampleCount; step += 1) {
    const timedIndex = Math.round((step / (sampleCount - 1)) * (timed.length - 1));
    const sample = timed[timedIndex];
    if (usedIndexes.has(sample.index)) continue;
    usedIndexes.add(sample.index);
    sampled.push({
      key: `${kind}:${sample.index}`,
      index: sample.index,
      timestampMs: sample.timestampMs,
      kind,
    });
  }
  return sampled;
};

export const collectTimedTimelineRows = (
  rows: readonly TimelineDateMarkSource[]
): TimedTimelineRow[] => {
  const timed: TimedTimelineRow[] = [];
  for (let index = 0; index < rows.length; index += 1) {
    const timestampMs = rowTimestampMs(rows[index]);
    if (timestampMs === undefined) continue;
    timed.push({ index, timestampMs });
  }
  return timed;
};

/**
 * High-level jump targets for the loaded timeline window only.
 * Prefers SDK date separators, then unique local days, then intra-day times.
 */
export const collectTimelineHistoryMarks = (
  rows: readonly TimelineDateMarkSource[]
): TimelineHistoryMark[] => {
  const separatorMarks: TimelineHistoryMark[] = [];
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index];
    if (row.kind !== 'date_separator') continue;
    const timestampMs = rowTimestampMs(row);
    if (timestampMs === undefined) continue;
    separatorMarks.push({
      key: row.itemId ?? `date:${index}`,
      index,
      timestampMs,
      kind: 'day',
    });
  }
  if (separatorMarks.length >= 2) return separatorMarks;

  const seenDays = new Set<string>();
  const dayMarks: TimelineHistoryMark[] = [];
  const timed = collectTimedTimelineRows(rows);
  for (const sample of timed) {
    const day = localDayKey(sample.timestampMs);
    if (seenDays.has(day)) continue;
    seenDays.add(day);
    dayMarks.push({
      key: `day:${day}`,
      index: sample.index,
      timestampMs: sample.timestampMs,
      kind: 'day',
    });
  }
  if (dayMarks.length >= 2) return dayMarks;
  if (timed.length < 2) return dayMarks;

  return sampleTimedMarks(timed, dayMarks.length <= 1 ? 'time' : 'day');
};

export const timelineRailAxis = (
  timed: readonly TimedTimelineRow[],
  roomCreatedTs: number | undefined,
  nowMs: number
): TimelineRailAxis | undefined => {
  if (timed.length === 0) return undefined;
  const loadedMinMs = timed[0].timestampMs;
  const loadedMaxMs = timed[timed.length - 1].timestampMs;
  const createTs =
    typeof roomCreatedTs === 'number' && Number.isFinite(roomCreatedTs) ? roomCreatedTs : undefined;
  const fullRoom = createTs !== undefined && createTs < loadedMinMs;
  return {
    startMs: fullRoom ? createTs : loadedMinMs,
    endMs: fullRoom ? Math.max(nowMs, loadedMaxMs) : loadedMaxMs,
    loadedMinMs,
    loadedMaxMs,
    fullRoom,
  };
};

export const withRoomBeginningMark = (
  marks: readonly TimelineHistoryMark[],
  axis: TimelineRailAxis | undefined
): TimelineHistoryMark[] => {
  if (!axis?.fullRoom) return [...marks];
  if (marks.some((mark) => mark.key === 'beginning')) return [...marks];
  return [
    {
      key: 'beginning',
      index: -1,
      timestampMs: axis.startMs,
      kind: 'beginning',
    },
    ...marks,
  ];
};

export const formatTimelineHistoryMarkLabel = (
  mark: TimelineHistoryMark,
  hour24Clock: boolean
): string => {
  if (mark.kind === 'beginning') return 'Beginning';
  if (mark.kind === 'time') return timeHourMinute(mark.timestampMs, hour24Clock);
  if (today(mark.timestampMs)) return 'Today';
  if (yesterday(mark.timestampMs)) return 'Yesterday';
  return timeDayMonthYear(mark.timestampMs);
};

export const formatTimelineRailTimestamp = (
  timestampMs: number,
  hour24Clock: boolean,
  axis: Pick<TimelineRailAxis, 'startMs' | 'endMs'>
): string => {
  if (axis.endMs - axis.startMs < INTRA_DAY_AXIS_MS) {
    return timeHourMinute(timestampMs, hour24Clock);
  }
  if (today(timestampMs)) return 'Today';
  if (yesterday(timestampMs)) return 'Yesterday';
  return timeDayMonthYear(timestampMs);
};

export const activeTimelineHistoryMarkIndex = (
  marks: readonly TimelineHistoryMark[],
  visibleStartIndex: number
): number => {
  if (marks.length === 0) return -1;
  let active = 0;
  for (let index = 0; index < marks.length; index += 1) {
    if (marks[index].index <= visibleStartIndex) active = index;
    else break;
  }
  return active;
};

export const activeTimelineHistoryMarkForTimestamp = (
  marks: readonly TimelineHistoryMark[],
  timestampMs: number
): number => {
  if (marks.length === 0) return -1;
  let active = 0;
  for (let index = 0; index < marks.length; index += 1) {
    if (marks[index].timestampMs <= timestampMs) active = index;
    else break;
  }
  return active;
};

export const timestampForRailRatio = (ratio: number, startMs: number, endMs: number): number => {
  const clamped = Math.min(1, Math.max(0, ratio));
  if (endMs <= startMs) return startMs;
  return Math.round(startMs + clamped * (endMs - startMs));
};

export const railRatioForTimestamp = (
  timestampMs: number,
  startMs: number,
  endMs: number
): number => {
  if (endMs <= startMs) return 0;
  return Math.min(1, Math.max(0, (timestampMs - startMs) / (endMs - startMs)));
};

export const rowIndexForRailRatio = (ratio: number, rowCount: number): number => {
  if (rowCount <= 1) return 0;
  const clamped = Math.min(1, Math.max(0, ratio));
  return Math.round(clamped * (rowCount - 1));
};

export const rowIndexForTimestamp = (
  timed: readonly TimedTimelineRow[],
  timestampMs: number
): number => {
  if (timed.length === 0) return 0;
  let lo = 0;
  let hi = timed.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (timed[mid].timestampMs < timestampMs) lo = mid + 1;
    else hi = mid;
  }
  if (lo === 0) return timed[0].index;
  if (lo >= timed.length) return timed[timed.length - 1].index;
  const previous = timed[lo - 1];
  const next = timed[lo];
  return timestampMs - previous.timestampMs <= next.timestampMs - timestampMs
    ? previous.index
    : next.index;
};

export const isTimestampInLoadedWindow = (
  timestampMs: number,
  axis: TimelineRailAxis,
  pagination: { backwardAvailable: boolean; forwardAvailable: boolean }
): boolean => {
  if (timestampMs >= axis.loadedMinMs && timestampMs <= axis.loadedMaxMs) return true;
  if (timestampMs < axis.loadedMinMs && !pagination.backwardAvailable) return true;
  if (timestampMs > axis.loadedMaxMs && !pagination.forwardAvailable) return true;
  return false;
};

export const shouldShowTimelineDateRail = (rowCount: number, markCount: number): boolean =>
  markCount >= 2 && rowCount >= 8;
