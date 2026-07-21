import assert from 'node:assert/strict';
import test from 'node:test';
import { AccountDataEvent } from '../../../types/matrix/accountData';
import { clearUnreadAnchor, markAsRead } from '../notifications';
import { getThreadRootEventId, roomHaveUnread } from '../room';
import {
  ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  TIMELINE_BOTTOM_TOLERANCE_PX,
  getLatestReceiptEventFromEvents,
  getRoomCurrentState,
  isTimelineViewportAtBottom,
  shouldRestoreRoomTimelineViewport,
} from '../timelineLifecycle';

const createTimelineEvent = (
  id: string,
  {
    sender = '@bob:example.org',
    type = 'm.room.message',
    sending = false,
    ts = 0,
  }: {
    sender?: string;
    type?: string;
    sending?: boolean;
    ts?: number;
  } = {}
) =>
  ({
    getId: () => id,
    getSender: () => sender,
    getType: () => type,
    getTs: () => ts,
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
  let markerArgs: any[] | undefined;

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
    compareEventOrdering: () => null,
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
    setRoomReadMarkers: async (...args: any[]) => {
      markerArgs = args;
    },
  } as any;

  await markAsRead(mx, room.roomId, false);

  assert.equal(latestTimelineCalls, 1);
  assert.deepEqual(markerArgs, [room.roomId, '$latest', latest, undefined]);
});

test('markAsRead can explicitly use the loaded live tail for mounted bottom state', async () => {
  const liveTail = createTimelineEvent('$loaded-live-tail');
  let latestTimelineCalls = 0;
  let markerArgs: any[] | undefined;

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
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    getLatestTimeline: async () => {
      latestTimelineCalls += 1;
      return undefined;
    },
    setRoomReadMarkers: async (...args: any[]) => {
      markerArgs = args;
    },
  } as any;

  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

  assert.equal(latestTimelineCalls, 0);
  assert.deepEqual(markerArgs, [room.roomId, '$loaded-live-tail', liveTail, undefined]);
});

test('markAsRead sends a private receipt in the same exact read-marker request', async () => {
  const latest = createTimelineEvent('$latest');
  let markerArgs: any[] | undefined;
  const room = {
    roomId: '!room:example.org',
    accountData: { get: () => undefined },
    getEventReadUpTo: () => '$older',
    getLiveTimeline: () => ({ getEvents: () => [latest] }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: any[]) => {
      markerArgs = args;
    },
  } as any;

  await markAsRead(mx, room.roomId, true, 'loaded-live-tail');

  assert.deepEqual(markerArgs, [room.roomId, '$latest', undefined, latest]);
});

test('markAsRead repairs a stale fully-read marker even when the receipt is already latest', async () => {
  const latest = createTimelineEvent('$latest');
  let markerArgs: any[] | undefined;
  const room = {
    roomId: '!room:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => [latest] }),
    getAccountData: () => ({ getContent: () => ({ event_id: '$stale' }) }),
    getReadReceiptForUserId: () => ({ eventId: '$latest' }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: any[]) => {
      markerArgs = args;
    },
  } as any;

  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

  assert.deepEqual(markerArgs, [room.roomId, '$latest', latest, undefined]);
});

test('markAsRead clears custom unread state without resending an already-current marker', async () => {
  const latest = createTimelineEvent('$latest');
  let markerWrites = 0;
  let markedUnreadWrites = 0;
  let unreadAnchorWrites = 0;
  const room = {
    roomId: '!room:example.org',
    accountData: {
      get: () => ({ getContent: () => ({ unread: true }) }),
    },
    getLiveTimeline: () => ({ getEvents: () => [latest] }),
    getAccountData: () => ({ getContent: () => ({ event_id: '$latest' }) }),
    getReadReceiptForUserId: () => ({ eventId: '$latest' }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => ({
      getContent: () => ({
        version: 1,
        anchors: { [room.roomId]: { eventId: '$anchor', ts: 1 } },
      }),
    }),
    setRoomReadMarkers: async () => {
      markerWrites += 1;
    },
    setRoomAccountData: async () => {
      markedUnreadWrites += 1;
    },
    setAccountData: async () => {
      unreadAnchorWrites += 1;
    },
  } as any;

  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

  assert.equal(markerWrites, 0);
  assert.equal(markedUnreadWrites, 1);
  assert.equal(unreadAnchorWrites, 1);
});

test('markAsRead preserves custom unread state when the server marker fails', async () => {
  const latest = createTimelineEvent('$latest');
  let markedUnreadWrites = 0;
  let unreadAnchorWrites = 0;
  const room = {
    roomId: '!room:example.org',
    accountData: {
      get: () => ({ getContent: () => ({ unread: true }) }),
    },
    getEventReadUpTo: () => '$older',
    getLiveTimeline: () => ({ getEvents: () => [latest] }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => ({
      getContent: () => ({
        version: 1,
        anchors: { [room.roomId]: { eventId: '$anchor', ts: 1 } },
      }),
    }),
    setRoomReadMarkers: async () => {
      throw new Error('network unavailable');
    },
    setRoomAccountData: async () => {
      markedUnreadWrites += 1;
    },
    setAccountData: async () => {
      unreadAnchorWrites += 1;
    },
  } as any;

  await assert.rejects(
    markAsRead(mx, room.roomId, false, 'loaded-live-tail'),
    /network unavailable/
  );

  assert.equal(markedUnreadWrites, 0);
  assert.equal(unreadAnchorWrites, 0);
});

test('markAsRead serializes writes and coalesces pending markers to the newest event', async () => {
  const first = createTimelineEvent('$first', { ts: 1 });
  const second = createTimelineEvent('$second', { ts: 2 });
  const third = createTimelineEvent('$third', { ts: 3 });
  let liveEvents = [first];
  let releaseFirst: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  let unblockFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    unblockFirst = resolve;
  });
  const writes: string[] = [];
  const room = {
    roomId: '!room:example.org',
    accountData: { get: () => undefined },
    getEventReadUpTo: () => '$older',
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: (leftId: string, rightId: string) =>
      liveEvents.findIndex((event) => event.getId() === leftId) -
      liveEvents.findIndex((event) => event.getId() === rightId),
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (_roomId: string, eventId: string) => {
      writes.push(eventId);
      if (writes.length === 1) {
        releaseFirst?.();
        await firstWriteBlocked;
      }
    },
  } as any;

  const firstRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  await firstWriteStarted;
  const duplicateFirstRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  liveEvents = [first, second];
  const secondRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  liveEvents = [first, second, third];
  const thirdRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  unblockFirst?.();

  await Promise.all([firstRequest, duplicateFirstRequest, secondRequest, thirdRequest]);
  assert.deepEqual(writes, ['$first', '$third']);
});

test('read-marker waiters preserve a successful request when the following write fails', async () => {
  const first = createTimelineEvent('$first', { ts: 1 });
  const second = createTimelineEvent('$second', { ts: 2 });
  let liveEvents = [first];
  let notifyStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  const writes: string[] = [];
  const room = {
    roomId: '!success-then-fail:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: (leftId: string, rightId: string) =>
      liveEvents.findIndex((event) => event.getId() === leftId) -
      liveEvents.findIndex((event) => event.getId() === rightId),
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (_roomId: string, eventId: string) => {
      writes.push(eventId);
      if (eventId === '$first') {
        notifyStarted?.();
        await firstWriteBlocked;
        return;
      }
      throw new Error('second write failed');
    },
  } as any;

  const firstRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  await firstWriteStarted;
  liveEvents = [first, second];
  const secondRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  const secondOutcome = assert.rejects(secondRequest, /second write failed/);
  releaseFirst?.();

  await firstRequest;
  await secondOutcome;
  assert.deepEqual(writes, ['$first', '$second']);
});

test('read-marker waiters reject a failed request without poisoning a newer success', async () => {
  const first = createTimelineEvent('$first', { ts: 1 });
  const second = createTimelineEvent('$second', { ts: 2 });
  let liveEvents = [first];
  let notifyStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  const writes: string[] = [];
  const room = {
    roomId: '!fail-then-success:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: (leftId: string, rightId: string) =>
      liveEvents.findIndex((event) => event.getId() === leftId) -
      liveEvents.findIndex((event) => event.getId() === rightId),
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (_roomId: string, eventId: string) => {
      writes.push(eventId);
      if (eventId === '$first') {
        notifyStarted?.();
        await firstWriteBlocked;
        throw new Error('first write failed');
      }
    },
  } as any;

  const firstRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  await firstWriteStarted;
  liveEvents = [first, second];
  const secondRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  const firstOutcome = assert.rejects(firstRequest, /first write failed/);
  releaseFirst?.();

  await firstOutcome;
  await secondRequest;
  assert.deepEqual(writes, ['$first', '$second']);
});

test('read-marker queue accepts a new request after the previous drain settles', async () => {
  const first = createTimelineEvent('$first', { ts: 1 });
  const second = createTimelineEvent('$second', { ts: 2 });
  let liveEvents = [first];
  const writes: string[] = [];
  const room = {
    roomId: '!drain-restart:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: (leftId: string, rightId: string) =>
      liveEvents.findIndex((event) => event.getId() === leftId) -
      liveEvents.findIndex((event) => event.getId() === rightId),
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (_roomId: string, eventId: string) => {
      writes.push(eventId);
    },
  } as any;

  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  liveEvents = [first, second];
  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

  assert.deepEqual(writes, ['$first', '$second']);
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
