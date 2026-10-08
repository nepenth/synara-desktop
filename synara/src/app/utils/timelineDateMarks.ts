import { timeDayMonthYear, today, yesterday } from './time';

export type TimelineDateMarkSource = {
  kind: string;
  itemId?: string;
  timestampMs?: number;
  originServerTs?: number;
  event?: { originServerTs?: number };
};

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

/** "Today", "Yesterday" or the date, for the day a timeline row belongs to. */
export const formatTimelineDayLabel = (timestampMs: number): string => {
  if (today(timestampMs)) return 'Today';
  if (yesterday(timestampMs)) return 'Yesterday';
  return timeDayMonthYear(timestampMs);
};
