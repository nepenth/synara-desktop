import assert from 'node:assert/strict';
import test from 'node:test';
import {
  markAsRead,
  markAsReadFromExplicitUserAction,
  markAsReadFromExplicitUserActionInBackground,
  markAsReadInBackground,
} from '../notifications';
import { getThreadRootEventId } from '../room';
import {
  ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  TIMELINE_BOTTOM_TOLERANCE_PX,
  getRoomCurrentState,
  isTimelineViewportAtBottom,
  shouldRestoreRoomTimelineViewport,
} from '../timelineLifecycle';

test('markAsRead fails closed outside the desktop shell instead of writing receipts', async () => {
  await assert.rejects(markAsRead('!r:example.org'), /requires the Synara desktop app/);
  await assert.rejects(
    markAsReadFromExplicitUserAction('!r:example.org'),
    /requires the Synara desktop app/
  );
});

test('background Mark Read wrappers consume failures and return void', async () => {
  assert.equal(markAsReadInBackground('!r:example.org'), undefined);
  assert.equal(markAsReadFromExplicitUserActionInBackground('!r:example.org'), undefined);
  await new Promise<void>((resolve) => {
    setImmediate(resolve);
  });
});

test('getRoomCurrentState prefers the SDK room current state over timeline state', () => {
  const currentState = { marker: 'current' };
  const timelineState = { marker: 'timeline' };
  const room = {
    currentState,
    getLiveTimeline: () => ({
      getState: () => timelineState,
    }),
  } as any;

  assert.equal(getRoomCurrentState(room), currentState);
});

test('timeline viewport restore policy lets unread state win over saved history', () => {
  const nowMs = 10_000;
  const viewport = {
    atBottom: false,
    updatedAtMs: nowMs,
  };

  assert.equal(
    shouldRestoreRoomTimelineViewport(viewport, {
      hasUnread: true,
      nowMs,
    }),
    false
  );
});

test('timeline viewport restore policy lets unread state win over saved bottom snapshots', () => {
  assert.equal(
    shouldRestoreRoomTimelineViewport(
      {
        atBottom: true,
        liveTailEventId: '$old-tail',
      },
      {
        hasUnread: true,
        currentLiveTailEventId: '$new-tail',
        nowMs: 10_000,
      }
    ),
    false
  );
});

test('timeline viewport restore policy keeps explicit bottom when unread state is stale', () => {
  assert.equal(
    shouldRestoreRoomTimelineViewport(
      {
        atBottom: true,
        liveTailEventId: '$tail',
        updatedAtMs: 10_000,
      },
      {
        hasUnread: true,
        currentLiveTailEventId: '$tail',
        nowMs: 10_000,
      }
    ),
    true
  );
});

test('timeline viewport restore policy expires stale historical anchors', () => {
  const nowMs = 10_000 + ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS;
  const freshViewport = {
    atBottom: false,
    updatedAtMs: 10_000,
  };
  const staleViewport = {
    atBottom: false,
    updatedAtMs: 9_999,
  };

  assert.equal(
    shouldRestoreRoomTimelineViewport(freshViewport, {
      hasUnread: false,
      nowMs,
    }),
    true
  );
  assert.equal(
    shouldRestoreRoomTimelineViewport(staleViewport, {
      hasUnread: false,
      nowMs,
    }),
    false
  );
});

test('timeline viewport restore policy always allows bottom snapshots without unread', () => {
  assert.equal(
    shouldRestoreRoomTimelineViewport(
      {
        atBottom: true,
      },
      {
        hasUnread: false,
        nowMs: 10_000,
      }
    ),
    true
  );
});

test('timeline live follow requires the viewport to be at the exact bottom', () => {
  assert.equal(isTimelineViewportAtBottom(1_000, 400, 600), true);
  assert.equal(isTimelineViewportAtBottom(1_000, 399, 600), true);
  assert.equal(TIMELINE_BOTTOM_TOLERANCE_PX, 1);
  assert.equal(isTimelineViewportAtBottom(1_000, 398, 600), false);
  assert.equal(isTimelineViewportAtBottom(1_000, 380, 600), false);
});

test('getThreadRootEventId returns thread root ids when available', () => {
  const threadRootEvent = getThreadRootEventId({
    getRelation: () => ({
      rel_type: 'm.thread',
      event_id: '$thread-root',
    }),
  } as any);
  assert.equal(threadRootEvent, '$thread-root');
});

test('getThreadRootEventId ignores non-thread relations', () => {
  const threadRootEvent = getThreadRootEventId({
    getRelation: () => ({
      rel_type: 'm.annotation',
      event_id: '$thread-root',
    }),
  } as any);
  assert.equal(threadRootEvent, undefined);
});
