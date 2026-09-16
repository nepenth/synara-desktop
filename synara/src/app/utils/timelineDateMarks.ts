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
  kind: 'day' | 'time';
};

const MAX_SAMPLED_MARKS = 6;

const rowTimestampMs = (row: TimelineDateMarkSource): number | undefined => {
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
  timed: readonly { index: number; timestampMs: number }[],
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
  const timed: { index: number; timestampMs: number }[] = [];
  for (let index = 0; index < rows.length; index += 1) {
    const timestampMs = rowTimestampMs(rows[index]);
    if (timestampMs === undefined) continue;
    timed.push({ index, timestampMs });
    const day = localDayKey(timestampMs);
    if (seenDays.has(day)) continue;
    seenDays.add(day);
    dayMarks.push({
      key: `day:${day}`,
      index,
      timestampMs,
      kind: 'day',
    });
  }
  if (dayMarks.length >= 2) return dayMarks;
  if (timed.length < 2) return dayMarks;

  return sampleTimedMarks(timed, dayMarks.length <= 1 ? 'time' : 'day');
};

export const formatTimelineHistoryMarkLabel = (
  mark: TimelineHistoryMark,
  hour24Clock: boolean
): string => {
  if (mark.kind === 'time') return timeHourMinute(mark.timestampMs, hour24Clock);
  if (today(mark.timestampMs)) return 'Today';
  if (yesterday(mark.timestampMs)) return 'Yesterday';
  return timeDayMonthYear(mark.timestampMs);
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

export const rowIndexForRailRatio = (ratio: number, rowCount: number): number => {
  if (rowCount <= 1) return 0;
  const clamped = Math.min(1, Math.max(0, ratio));
  return Math.round(clamped * (rowCount - 1));
};

export const shouldShowTimelineDateRail = (rowCount: number, markCount: number): boolean =>
  markCount >= 2 && rowCount >= 8;
