import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  shouldRestoreNativeTimelineViewport,
  shouldShowJumpToLatest,
} from '../nativeTimelineViewportPolicy';

test('restores at-bottom when the room has no unread', () => {
  assert.equal(
    shouldRestoreNativeTimelineViewport(
      { atBottom: true, liveTailEventId: '$tail:example.org' },
      { hasUnread: false, nowMs: 10_000 }
    ),
    true
  );
});

test('restores historical anchors only inside the TTL window', () => {
  const nowMs = 10_000 + NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS;
  assert.equal(
    shouldRestoreNativeTimelineViewport(
      {
        restoredAnchorEventId: '$anchor:example.org',
        updatedAtMs: nowMs - NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
      },
      { hasUnread: false, nowMs }
    ),
    true
  );
  assert.equal(
    shouldRestoreNativeTimelineViewport(
      {
        restoredAnchorEventId: '$anchor:example.org',
        updatedAtMs: nowMs - NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS - 1,
      },
      { hasUnread: false, nowMs }
    ),
    false
  );
});

test('unread blocks historical restore unless live-tail at-bottom still matches', () => {
  assert.equal(
    shouldRestoreNativeTimelineViewport(
      {
        restoredAnchorEventId: '$anchor:example.org',
        updatedAtMs: 10_000,
      },
      { hasUnread: true, nowMs: 10_000, currentLiveTailEventId: '$tail:example.org' }
    ),
    false
  );
  assert.equal(
    shouldRestoreNativeTimelineViewport(
      {
        atBottom: true,
        liveTailEventId: '$tail:example.org',
      },
      { hasUnread: true, nowMs: 10_000, currentLiveTailEventId: '$tail:example.org' }
    ),
    true
  );
});

test('jump to latest stays available until the live tail is the loaded window', () => {
  assert.equal(shouldShowJumpToLatest('unread', true), true);
  assert.equal(shouldShowJumpToLatest('focused', true), true);
  assert.equal(shouldShowJumpToLatest('restored', true), true);
  assert.equal(shouldShowJumpToLatest('live_bottom', false), true);
  assert.equal(shouldShowJumpToLatest('live_bottom', true), false);
  assert.equal(shouldShowJumpToLatest(undefined, false), true);
  assert.equal(shouldShowJumpToLatest(undefined, true), false);
});
