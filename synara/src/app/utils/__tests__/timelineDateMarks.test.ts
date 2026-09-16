import assert from 'node:assert/strict';
import test from 'node:test';
import {
  activeTimelineHistoryMarkIndex,
  collectTimelineHistoryMarks,
  formatTimelineHistoryMarkLabel,
  rowIndexForRailRatio,
  shouldShowTimelineDateRail,
} from '../timelineDateMarks';

const day = (year: number, month: number, date: number, hour = 12): number =>
  new Date(year, month - 1, date, hour, 0, 0).getTime();

test('collects SDK date separators as day marks in row order', () => {
  const marks = collectTimelineHistoryMarks([
    { kind: 'date_separator', itemId: 'd1', timestampMs: day(2026, 9, 14) },
    { kind: 'message', originServerTs: day(2026, 9, 14, 13) },
    { kind: 'date_separator', itemId: 'd2', timestampMs: day(2026, 9, 15) },
    { kind: 'message', originServerTs: day(2026, 9, 15, 9) },
  ]);
  assert.deepEqual(
    marks.map((mark) => ({ key: mark.key, index: mark.index, kind: mark.kind })),
    [
      { key: 'd1', index: 0, kind: 'day' },
      { key: 'd2', index: 2, kind: 'day' },
    ]
  );
});

test('falls back to unique local days when separators are absent', () => {
  const marks = collectTimelineHistoryMarks([
    { kind: 'message', originServerTs: day(2026, 9, 13, 8) },
    { kind: 'message', originServerTs: day(2026, 9, 13, 18) },
    { kind: 'sticker', event: { originServerTs: day(2026, 9, 14, 11) } },
    { kind: 'message', originServerTs: day(2026, 9, 14, 20) },
  ]);
  assert.equal(marks.length, 2);
  assert.equal(marks[0].index, 0);
  assert.equal(marks[1].index, 2);
  assert.equal(
    marks.every((mark) => mark.kind === 'day'),
    true
  );
});

test('samples intra-day times when the loaded window is a single day', () => {
  const rows = Array.from({ length: 12 }, (_, index) => ({
    kind: 'message',
    originServerTs: day(2026, 9, 16, 8) + index * 60 * 60 * 1000,
  }));
  const marks = collectTimelineHistoryMarks(rows);
  assert.ok(marks.length >= 2);
  assert.equal(marks[0].index, 0);
  assert.equal(marks[marks.length - 1].index, 11);
  assert.equal(
    marks.every((mark) => mark.kind === 'time'),
    true
  );
});

test('active mark follows the first visible row without running past it', () => {
  const marks = [
    { key: 'a', index: 0, timestampMs: 1, kind: 'day' as const },
    { key: 'b', index: 10, timestampMs: 2, kind: 'day' as const },
    { key: 'c', index: 20, timestampMs: 3, kind: 'day' as const },
  ];
  assert.equal(activeTimelineHistoryMarkIndex(marks, 0), 0);
  assert.equal(activeTimelineHistoryMarkIndex(marks, 10), 1);
  assert.equal(activeTimelineHistoryMarkIndex(marks, 19), 1);
  assert.equal(activeTimelineHistoryMarkIndex(marks, 40), 2);
  assert.equal(activeTimelineHistoryMarkIndex([], 0), -1);
});

test('rail ratio maps onto loaded row indexes only', () => {
  assert.equal(rowIndexForRailRatio(0, 80), 0);
  assert.equal(rowIndexForRailRatio(1, 80), 79);
  assert.equal(rowIndexForRailRatio(0.5, 81), 40);
  assert.equal(rowIndexForRailRatio(-1, 10), 0);
  assert.equal(rowIndexForRailRatio(2, 10), 9);
});

test('date rail stays hidden for sparse windows', () => {
  assert.equal(shouldShowTimelineDateRail(6, 3), false);
  assert.equal(shouldShowTimelineDateRail(40, 1), false);
  assert.equal(shouldShowTimelineDateRail(40, 2), true);
});

test('day labels use today/yesterday when applicable', () => {
  const now = Date.now();
  assert.equal(
    formatTimelineHistoryMarkLabel({ key: 't', index: 0, timestampMs: now, kind: 'day' }, true),
    'Today'
  );
  const localAfternoon = new Date(2026, 8, 16, 15, 0, 0).getTime();
  assert.equal(
    formatTimelineHistoryMarkLabel(
      { key: 'h', index: 0, timestampMs: localAfternoon, kind: 'time' },
      true
    ),
    '15:00'
  );
});
