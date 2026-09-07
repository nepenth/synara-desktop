import assert from 'node:assert/strict';
import test from 'node:test';

import {
  NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  nativeFollowLiveAttemptKey,
  nativeFollowLiveTarget,
  nativeLiveReadAttemptKey,
  nativeLiveReadTarget,
  nativeVisibleReadFrontier,
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

test('follow-live targets only painted tails on non-live positions', () => {
  const base = {
    roomId: '!room:example.org',
    atLiveBottom: true,
    positionKind: 'unread' as const,
    latestVisibleEventId: '$tail:example.org',
  };
  assert.equal(nativeFollowLiveTarget(base), '$tail:example.org');
  assert.equal(nativeFollowLiveTarget({ ...base, positionKind: 'restored' }), '$tail:example.org');
  assert.equal(nativeFollowLiveTarget({ ...base, positionKind: 'focused' }), '$tail:example.org');
  // Already live: the receipt path owns the tail, never follow.
  assert.equal(nativeFollowLiveTarget({ ...base, positionKind: 'live_bottom' }), undefined);
  // Not at the visual bottom, or no painted tail: no transition.
  assert.equal(nativeFollowLiveTarget({ ...base, atLiveBottom: false }), undefined);
  assert.equal(nativeFollowLiveTarget({ ...base, latestVisibleEventId: undefined }), undefined);
  assert.equal(
    nativeFollowLiveTarget({ ...base, latestVisibleEventId: 'not-an-event' }),
    undefined
  );
});

test('follow-live attempts are keyed per painted tail', () => {
  assert.equal(
    nativeFollowLiveAttemptKey('!room:example.org', '$a:example.org'),
    '!room:example.org:$a:example.org:follow-live'
  );
  assert.notEqual(
    nativeFollowLiveAttemptKey('!room:example.org', '$a:example.org'),
    nativeFollowLiveAttemptKey('!room:example.org', '$b:example.org')
  );
});

test('a folded edit after acknowledgement advances the receipt identity on the same row', () => {
  const row = '$message:example.org';
  const first = nativeVisibleReadFrontier(row, {
    visibleTailEventId: row,
    receiptTailEventId: row,
  });
  const edited = nativeVisibleReadFrontier(row, {
    visibleTailEventId: row,
    receiptTailEventId: '$edit:example.org',
  });
  const reacted = nativeVisibleReadFrontier(row, {
    visibleTailEventId: row,
    receiptTailEventId: '$reaction:example.org',
  });
  assert.equal(first, row);
  assert.equal(edited, '$edit:example.org');
  assert.equal(reacted, '$reaction:example.org');
  assert.notEqual(
    nativeLiveReadAttemptKey('!room:example.org', first!, false),
    nativeLiveReadAttemptKey('!room:example.org', edited!, false)
  );
  assert.notEqual(
    nativeLiveReadAttemptKey('!room:example.org', edited!, false),
    nativeLiveReadAttemptKey('!room:example.org', reacted!, false)
  );
});

test('frontier metadata ahead of displayed rows cannot acknowledge an unseen new message', () => {
  assert.equal(
    nativeVisibleReadFrontier('$old:example.org', {
      visibleTailEventId: '$new:example.org',
      receiptTailEventId: '$new:example.org',
    }),
    undefined
  );
  assert.equal(
    nativeVisibleReadFrontier(undefined, {
      visibleTailEventId: '$new:example.org',
      receiptTailEventId: '$new:example.org',
    }),
    undefined
  );
  assert.equal(nativeVisibleReadFrontier('$old:example.org', undefined), undefined);
});
