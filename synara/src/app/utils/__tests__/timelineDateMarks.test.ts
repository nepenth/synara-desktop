import assert from 'node:assert/strict';
import test from 'node:test';
import {
  formatTimelineDayHeading,
  formatTimelineDayLabel,
  rowTimestampMs,
} from '../timelineDateMarks';

test('row timestamps come from the row, the server time or the event', () => {
  assert.equal(rowTimestampMs({ kind: 'date_separator', timestampMs: 10 }), 10);
  assert.equal(rowTimestampMs({ kind: 'message', originServerTs: 20 }), 20);
  assert.equal(rowTimestampMs({ kind: 'message', event: { originServerTs: 30 } }), 30);
  assert.equal(rowTimestampMs({ kind: 'message' }), undefined);
  assert.equal(rowTimestampMs({ kind: 'message', timestampMs: Number.NaN }), undefined);
});

test('day labels say Today and Yesterday, then the date', () => {
  const now = Date.now();
  assert.equal(formatTimelineDayLabel(now), 'Today');
  // Calendar day, not 24 hours: on the day DST ends, now - 24h can still be today.
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  assert.equal(formatTimelineDayLabel(yesterday.getTime()), 'Yesterday');
  const older = formatTimelineDayLabel(now - 10 * 24 * 60 * 60 * 1000);
  assert.notEqual(older, 'Today');
  assert.notEqual(older, 'Yesterday');
});

test('day headings are short: Today, Yesterday, then the month and ordinal day', () => {
  const now = new Date(2026, 10, 18, 12, 0).getTime();
  const on = (year: number, month: number, day: number) =>
    new Date(year, month, day, 9, 0).getTime();
  assert.equal(formatTimelineDayHeading(on(2026, 10, 14), now), 'Nov 14th');
  assert.equal(formatTimelineDayHeading(on(2026, 10, 1), now), 'Nov 1st');
  assert.equal(formatTimelineDayHeading(on(2026, 10, 2), now), 'Nov 2nd');
  assert.equal(formatTimelineDayHeading(on(2026, 10, 3), now), 'Nov 3rd');
  assert.equal(formatTimelineDayHeading(on(2026, 10, 11), now), 'Nov 11th');
  assert.equal(formatTimelineDayHeading(on(2026, 10, 12), now), 'Nov 12th');
  assert.equal(formatTimelineDayHeading(on(2026, 10, 13), now), 'Nov 13th');
  assert.equal(formatTimelineDayHeading(on(2026, 9, 22), now), 'Oct 22nd');
  assert.equal(formatTimelineDayHeading(on(2026, 9, 23), now), 'Oct 23rd');
  assert.equal(formatTimelineDayHeading(on(2026, 9, 31), now), 'Oct 31st');
  // Another year says which.
  assert.equal(formatTimelineDayHeading(on(2025, 10, 14), now), 'Nov 14th, 2025');
  assert.equal(formatTimelineDayHeading(Date.now()), 'Today');
});
