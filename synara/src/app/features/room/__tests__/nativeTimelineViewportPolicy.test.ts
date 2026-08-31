import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  nativeLiveReadAttemptKey,
  nativeLiveReadTarget,
  latestNativeReadEventId,
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

const liveReadInput = {
  selectedRoomId: '!room:example.org',
  snapshotRoomId: '!room:example.org',
  documentActive: true,
  hideActivity: false,
  atLiveBottom: true,
  positionKind: 'live_bottom' as const,
  canMarkRead: true,
  latestVisibleEventId: '$tail:example.org',
  ownReadEventId: '$previous:example.org',
  isMarkedUnread: false,
};

test('live-tail read target keys work by visible event rather than snapshot revision', () => {
  assert.equal(nativeLiveReadTarget(liveReadInput), '$tail:example.org');
  assert.equal(nativeLiveReadTarget({ ...liveReadInput }), '$tail:example.org');
  assert.equal(
    nativeLiveReadTarget({ ...liveReadInput, ownReadEventId: '$tail:example.org' }),
    undefined
  );
});

test('live-tail read target rejects background, stale-room, and non-live views', () => {
  assert.equal(nativeLiveReadTarget({ ...liveReadInput, documentActive: false }), undefined);
  assert.equal(
    nativeLiveReadTarget({ ...liveReadInput, snapshotRoomId: '!old:example.org' }),
    undefined
  );
  assert.equal(nativeLiveReadTarget({ ...liveReadInput, atLiveBottom: false }), undefined);
  assert.equal(nativeLiveReadTarget({ ...liveReadInput, positionKind: 'focused' }), undefined);
});

test('explicit marked-unread state is cleared even when the receipt already covers the tail', () => {
  assert.equal(
    nativeLiveReadTarget({
      ...liveReadInput,
      ownReadEventId: '$tail:example.org',
      isMarkedUnread: true,
    }),
    '$tail:example.org'
  );
});

test('manual unread creates a new attempt identity on an already-covered tail', () => {
  assert.notEqual(
    nativeLiveReadAttemptKey('!room:example.org', '$tail:example.org', false),
    nativeLiveReadAttemptKey('!room:example.org', '$tail:example.org', true)
  );
});

test('hidden SDK-projected events still advance the exact read target', () => {
  const visibleMessage = '$message:example.org';
  const hiddenMembershipTail = '$membership:example.org';
  assert.equal(
    latestNativeReadEventId([visibleMessage, undefined, hiddenMembershipTail]),
    hiddenMembershipTail
  );
});
