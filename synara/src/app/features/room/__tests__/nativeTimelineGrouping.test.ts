import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_TIMELINE_GROUP_WINDOW_MS,
  shouldGroupNativeTimelineRows,
} from '../nativeTimelineGrouping';

const row = (senderId: string, originServerTs: number) => ({ senderId, originServerTs });

test('groups consecutive messages from one sender for up to two hours', () => {
  assert.equal(shouldGroupNativeTimelineRows(row('@a:test', 1), row('@a:test', 2)), true);
  assert.equal(
    shouldGroupNativeTimelineRows(
      row('@a:test', 1),
      row('@a:test', 1 + NATIVE_TIMELINE_GROUP_WINDOW_MS - 1)
    ),
    true
  );
});

test('starts a new visual group at the two-hour boundary or on sender change', () => {
  assert.equal(
    shouldGroupNativeTimelineRows(
      row('@a:test', 1),
      row('@a:test', 1 + NATIVE_TIMELINE_GROUP_WINDOW_MS)
    ),
    false
  );
  assert.equal(shouldGroupNativeTimelineRows(row('@a:test', 1), row('@b:test', 2)), false);
});

test('does not group missing, invalid, or reverse-ordered metadata', () => {
  assert.equal(shouldGroupNativeTimelineRows(undefined, row('@a:test', 2)), false);
  assert.equal(shouldGroupNativeTimelineRows({}, row('@a:test', 2)), false);
  assert.equal(shouldGroupNativeTimelineRows(row('@a:test', 2), row('@a:test', 1)), false);
});
