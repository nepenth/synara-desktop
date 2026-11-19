import dayjs from 'dayjs';
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

const ordinalDay = (day: number): string => {
  const teens = day % 100;
  if (teens >= 11 && teens <= 13) return `${day}th`;
  return `${day}${['th', 'st', 'nd', 'rd'][day % 10] ?? 'th'}`;
};

/**
 * A day heading in the timeline, short as Slack's: "Today", "Yesterday",
 * "Nov 14th", and "Nov 14th, 2025" for a day in another year. Messages under
 * it show only their time, so this is where the date lives.
 */
export const formatTimelineDayHeading = (timestampMs: number, now: number = Date.now()): string => {
  if (today(timestampMs)) return 'Today';
  if (yesterday(timestampMs)) return 'Yesterday';
  const day = dayjs(timestampMs);
  const label = `${day.format('MMM')} ${ordinalDay(day.date())}`;
  return day.year() === dayjs(now).year() ? label : `${label}, ${day.year()}`;
};
