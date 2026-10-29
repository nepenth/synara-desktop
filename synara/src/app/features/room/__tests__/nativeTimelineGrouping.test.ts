import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_TIMELINE_GROUP_WINDOW_MS,
  nativeTimelineRowShowsSender,
  shouldGroupNativeTimelineRows,
} from '../nativeTimelineGrouping';

const row = (senderId: string, originServerTs: number, kind = 'message') => ({
  kind,
  senderId,
  originServerTs,
});

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

test("a message after the same sender's system or hidden row starts a new group", () => {
  // "Ian joined" followed by Ian's first message: the membership row carries
  // Ian's id but no sender header, so the message must show name and avatar.
  for (const kind of ['membership', 'state', 'call', 'poll', 'other', 'redacted']) {
    assert.equal(
      shouldGroupNativeTimelineRows(row('@ian:test', 1, kind), row('@ian:test', 2)),
      false,
      kind
    );
    assert.equal(
      shouldGroupNativeTimelineRows(row('@ian:test', 1), row('@ian:test', 2, kind)),
      false,
      kind
    );
  }
  assert.equal(
    shouldGroupNativeTimelineRows(
      { senderId: '@ian:test', originServerTs: 1 },
      row('@ian:test', 2)
    ),
    false
  );
});

test('messages and stickers are the sender-attributed rows that group', () => {
  assert.equal(nativeTimelineRowShowsSender('message'), true);
  assert.equal(nativeTimelineRowShowsSender('sticker'), true);
  assert.equal(nativeTimelineRowShowsSender('membership'), false);
  assert.equal(nativeTimelineRowShowsSender(undefined), false);
  assert.equal(
    shouldGroupNativeTimelineRows(row('@a:test', 1, 'sticker'), row('@a:test', 2)),
    true
  );
});
