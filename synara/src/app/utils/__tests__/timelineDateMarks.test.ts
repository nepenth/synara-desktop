import assert from 'node:assert/strict';
import test from 'node:test';
import { formatTimelineDayLabel, rowTimestampMs } from '../timelineDateMarks';

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
