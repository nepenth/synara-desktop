import test from 'node:test';
import assert from 'node:assert/strict';
import {
  estimateTimelineRowSize,
  getRestoredVirtualScrollTop,
  getTimelineRowKey,
  getVirtualAnchorOffset,
  isVirtualRangeAtEnd,
  shouldPaginateVirtualRange,
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
