import assert from 'node:assert/strict';
import test from 'node:test';
import { AccountDataEvent } from '../../../types/matrix/accountData';
import { clearUnreadAnchor, markAsRead } from '../notifications';
import { getThreadRootEventId, roomHaveUnread } from '../room';
import {
  ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  getLatestReceiptEventFromEvents,
  getRoomCurrentState,
  shouldRestoreRoomTimelineViewport,
} from '../timelineLifecycle';

const createTimelineEvent = (
  id: string,
  {
    sender = '@bob:example.org',
    type = 'm.room.message',
    sending = false,
  }: {
    sender?: string;
    type?: string;
    sending?: boolean;
  } = {}
) =>
  ({
    getId: () => id,
    getSender: () => sender,
    getType: () => type,
    isSending: () => sending,
    isRedacted: () => false,
    getRelation: () => undefined,
  } as any);

const createUnreadRoom = (events: any[], readUpToId?: string) =>
  ({
    getEventReadUpTo: () => readUpToId,
    getLiveTimeline: () => ({
      getEvents: () => events,
    }),
  } as any);

test('clearUnreadAnchor skips account-data writes when the room has no anchor', async () => {
  let writes = 0;
  const mx = {
    getAccountData: () => ({
      getContent: () => ({
        version: 1,
        anchors: {
          '!other:example.org': {
            eventId: '$other',
            ts: 1,
          },
        },
      }),
    }),
    setAccountData: async () => {
      writes += 1;
    },
  } as any;

  await clearUnreadAnchor(mx, '!room:example.org');

  assert.equal(writes, 0);
});

test('clearUnreadAnchor removes existing anchors with one account-data write', async () => {
  let writtenContent: unknown;
  const mx = {
    getAccountData: () => ({
      getContent: () => ({
        version: 1,
        anchors: {
          '!room:example.org': {
            eventId: '$event',
            ts: 1,
          },
          '!other:example.org': {
            eventId: '$other',
            ts: 2,
          },
        },
      }),
    }),
    setAccountData: async (eventType: string, content: unknown) => {
      assert.equal(eventType, AccountDataEvent.SynaraUnreadAnchor);
      writtenContent = content;
    },
  } as any;

  await clearUnreadAnchor(mx, '!room:example.org');

  assert.deepEqual(writtenContent, {
    version: 1,
    anchors: {
      '!other:example.org': {
        eventId: '$other',
        ts: 2,
      },
    },
  });
});

test('latest receipt event helper skips local echoes and already-read tails', () => {
  const older = createTimelineEvent('$older');
  const sending = createTimelineEvent('$sending', { sending: true });
  const latest = createTimelineEvent('$latest');

  assert.equal(getLatestReceiptEventFromEvents([older, sending, latest], '$older'), latest);
  assert.equal(getLatestReceiptEventFromEvents([older, sending], '$older'), undefined);
  assert.equal(getLatestReceiptEventFromEvents([older], '$older'), undefined);
});

test('markAsRead resolves the latest SDK timeline by default', async () => {
  const liveTail = createTimelineEvent('$loaded-live-tail');
  const latest = createTimelineEvent('$latest');
  let latestTimelineCalls = 0;
  let receiptEvent: any;

  const room = {
    roomId: '!room:example.org',
    accountData: {
      get: () => undefined,
    },
    getEventReadUpTo: () => '$older',
    getLiveTimeline: () => ({
      getEvents: () => [liveTail],
    }),
    getUnfilteredTimelineSet: () => ({}),
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    getLatestTimeline: async () => {
      latestTimelineCalls += 1;
      return {
        getEvents: () => [latest],
      };
    },
    sendReadReceipt: async (event: any) => {
      receiptEvent = event;
    },
  } as any;

  await markAsRead(mx, room.roomId, false);

  assert.equal(latestTimelineCalls, 1);
  assert.equal(receiptEvent, latest);
});

test('markAsRead can explicitly use the loaded live tail for mounted bottom state', async () => {
  const liveTail = createTimelineEvent('$loaded-live-tail');
  let latestTimelineCalls = 0;
  let receiptEvent: any;

  const room = {
    roomId: '!room:example.org',
    accountData: {
      get: () => undefined,
    },
    getEventReadUpTo: () => '$older',
    getLiveTimeline: () => ({
      getEvents: () => [liveTail],
    }),
    getUnfilteredTimelineSet: () => ({}),
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    getLatestTimeline: async () => {
      latestTimelineCalls += 1;
      return undefined;
    },
    sendReadReceipt: async (event: any) => {
      receiptEvent = event;
    },
  } as any;

  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

  assert.equal(latestTimelineCalls, 0);
  assert.equal(receiptEvent, liveTail);
});

test('roomHaveUnread only infers unread from a loaded slice containing the read marker', () => {
  const readMarker = createTimelineEvent('$read');
  const unread = createTimelineEvent('$unread');
  const mx = {
    getUserId: () => '@alice:example.org',
  } as any;

  assert.equal(roomHaveUnread(mx, createUnreadRoom([readMarker, unread], '$read')), true);
  assert.equal(roomHaveUnread(mx, createUnreadRoom([unread], '$missing-read-marker')), false);
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
