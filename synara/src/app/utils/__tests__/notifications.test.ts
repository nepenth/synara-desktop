import assert from 'node:assert/strict';
import test from 'node:test';
// 'm.read' literal is the probed js-sdk value 'm.read'.
import { AccountDataEvent } from '../../../types/matrix/accountData';
import {
  clearUnreadAnchor,
  markAsRead,
  markAsReadAtEvent,
  markAsReadFromExplicitUserAction,
  markAsReadFromExplicitUserActionInBackground,
  markAsReadInBackground,
} from '../notifications';
import { getThreadRootEventId } from '../room';
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

test('explicit user Mark Read enters the private receipt channel', async () => {
  const latest = createTimelineEvent('$latest');
  let markerArgs: any[] | undefined;
  const room = {
    roomId: '!room:example.org',
    accountData: { get: () => undefined },
    getEventReadUpTo: () => '$older',
    getLiveTimeline: () => ({ getEvents: () => [latest] }),
    getUnfilteredTimelineSet: () => ({}),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    getLatestTimeline: async () => ({ getEvents: () => [latest] }),
    setRoomReadMarkers: async (...args: any[]) => {
      markerArgs = args;
    },
  } as any;

  await markAsReadFromExplicitUserAction(mx, room.roomId);

  assert.deepEqual(markerArgs, [room.roomId, '$latest', undefined, latest]);
});

test('explicit user Mark Read background wrapper remains observable as a void launch', async () => {
  const mx = { getRoom: () => undefined } as any;
  const result = markAsReadFromExplicitUserActionInBackground(mx, '!room:example.org');
  await Promise.resolve();
  assert.equal(result, undefined);
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

test('markAsReadAtEvent commits an authoritative event from a detached latest timeline', async () => {
  const loadedLiveTail = createTimelineEvent('$loaded-live-tail', { ts: 1 });
  const detachedLatest = createTimelineEvent('$detached-latest', { ts: 2 });
  let latestTimelineCalls = 0;
  let markerArgs: any[] | undefined;
  const room = {
    roomId: '!detached-latest:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => [loadedLiveTail] }),
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

  await markAsReadAtEvent(mx, room.roomId, true, detachedLatest);

  assert.equal(latestTimelineCalls, 0);
  assert.deepEqual(markerArgs, [room.roomId, '$detached-latest', undefined, detachedLatest]);
});

test('markAsReadAtEvent preserves custom unread state when the explicit marker fails', async () => {
  const detachedLatest = createTimelineEvent('$detached-latest', { ts: 2 });
  let markedUnreadWrites = 0;
  let unreadAnchorWrites = 0;
  const room = {
    roomId: '!detached-failure:example.org',
    accountData: {
      get: () => ({ getContent: () => ({ unread: true }) }),
    },
    getLiveTimeline: () => ({ getEvents: () => [] }),
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
      throw new Error('detached marker failed');
    },
    setRoomAccountData: async () => {
      markedUnreadWrites += 1;
    },
    setAccountData: async () => {
      unreadAnchorWrites += 1;
    },
  } as any;

  await assert.rejects(
    markAsReadAtEvent(mx, room.roomId, false, detachedLatest),
    /detached marker failed/
  );

  assert.equal(markedUnreadWrites, 0);
  assert.equal(unreadAnchorWrites, 0);
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

test('exact read-marker emergency disable uses the SDK receipt fallback', async () => {
  const originalWindow = globalThis.window;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage: {
        getItem: (key: string) => (key === 'synara.feature.exactReadMarkers' ? 'false' : null),
      },
    },
  });
  try {
    const latest = createTimelineEvent('$legacy-latest');
    let markerWrites = 0;
    let receiptArgs: any[] | undefined;
    const room = {
      roomId: '!legacy-read:example.org',
      accountData: { get: () => undefined },
      getEventReadUpTo: () => '$older',
      getLiveTimeline: () => ({ getEvents: () => [latest] }),
      getAccountData: () => undefined,
      getReadReceiptForUserId: () => undefined,
    } as any;
    const mx = {
      getRoom: () => room,
      getUserId: () => '@alice:example.org',
      getAccountData: () => undefined,
      sendReadReceipt: async (...args: any[]) => {
        receiptArgs = args;
      },
      setRoomReadMarkers: async () => {
        markerWrites += 1;
      },
    } as any;

    await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

    assert.deepEqual(receiptArgs, [latest, 'm.read']);
    assert.equal(markerWrites, 0);
  } finally {
    if (originalWindow === undefined) {
      Reflect.deleteProperty(globalThis, 'window');
    } else {
      Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    }
  }
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

test('equal-timestamp detached targets keep distinct writes and waiters', async () => {
  const first = createTimelineEvent('$equal-first', { ts: 1 });
  const second = createTimelineEvent('$equal-second', { ts: 10 });
  const third = createTimelineEvent('$equal-third', { ts: 10 });
  let notifyFirstStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyFirstStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  let notifySecondStarted: (() => void) | undefined;
  const secondWriteStarted = new Promise<void>((resolve) => {
    notifySecondStarted = resolve;
  });
  let releaseSecond: (() => void) | undefined;
  const secondWriteBlocked = new Promise<void>((resolve) => {
    releaseSecond = resolve;
  });
  let notifyThirdStarted: (() => void) | undefined;
  const thirdWriteStarted = new Promise<void>((resolve) => {
    notifyThirdStarted = resolve;
  });
  let releaseThird: (() => void) | undefined;
  const thirdWriteBlocked = new Promise<void>((resolve) => {
    releaseThird = resolve;
  });
  const writes: Array<[string, string, any, any]> = [];
  const room = {
    roomId: '!equal-detached:example.org',
    accountData: { get: () => undefined },
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: [string, string, any, any]) => {
      writes.push(args);
      if (writes.length === 1) {
        notifyFirstStarted?.();
        await firstWriteBlocked;
      } else if (writes.length === 2) {
        notifySecondStarted?.();
        await secondWriteBlocked;
      } else if (writes.length === 3) {
        notifyThirdStarted?.();
        await thirdWriteBlocked;
      }
    },
  } as any;

  const firstRequest = markAsReadAtEvent(mx, room.roomId, false, first);
  await firstWriteStarted;
  let secondResolved = false;
  const secondRequest = markAsReadAtEvent(mx, room.roomId, false, second).then(() => {
    secondResolved = true;
  });
  let thirdResolved = false;
  const thirdRequest = markAsReadAtEvent(mx, room.roomId, false, third).then(() => {
    thirdResolved = true;
  });

  releaseFirst?.();
  await secondWriteStarted;
  assert.equal(secondResolved, false);
  assert.equal(thirdResolved, false);

  releaseSecond?.();
  await secondRequest;
  await thirdWriteStarted;
  assert.equal(secondResolved, true);
  assert.equal(thirdResolved, false);

  releaseThird?.();
  await Promise.all([firstRequest, secondRequest, thirdRequest]);
  assert.deepEqual(
    writes.map(([, fullyReadId, publicEvent]) => [fullyReadId, publicEvent?.getId()]),
    [
      ['$equal-first', '$equal-first'],
      ['$equal-second', '$equal-second'],
      ['$equal-second', '$equal-third'],
    ]
  );
});

test('invalid-timestamp detached targets fail and resolve only their own waiters', async () => {
  const first = createTimelineEvent('$invalid-first', { ts: 1 });
  const invalidFailure = createTimelineEvent('$invalid-failure', { ts: Number.NaN });
  const invalidSuccess = createTimelineEvent('$invalid-success', { ts: Number.POSITIVE_INFINITY });
  let notifyFirstStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyFirstStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  const writes: Array<[string, string, any, any]> = [];
  const room = {
    roomId: '!invalid-detached:example.org',
    accountData: { get: () => undefined },
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: [string, string, any, any]) => {
      writes.push(args);
      if (writes.length === 1) {
        notifyFirstStarted?.();
        await firstWriteBlocked;
      } else if (args[2]?.getId() === '$invalid-failure') {
        throw new Error('invalid target failed');
      }
    },
  } as any;

  const firstRequest = markAsReadAtEvent(mx, room.roomId, false, first);
  await firstWriteStarted;
  const failingRequest = markAsReadAtEvent(mx, room.roomId, false, invalidFailure);
  const successfulRequest = markAsReadAtEvent(mx, room.roomId, false, invalidSuccess);
  const failingOutcome = assert.rejects(failingRequest, /invalid target failed/);
  releaseFirst?.();

  await Promise.all([firstRequest, failingOutcome, successfulRequest]);
  assert.deepEqual(
    writes.map(([, fullyReadId, publicEvent]) => [fullyReadId, publicEvent?.getId()]),
    [
      ['$invalid-first', '$invalid-first'],
      ['$invalid-first', '$invalid-failure'],
      ['$invalid-first', '$invalid-success'],
    ]
  );
});

test('an active public receipt never satisfies an older private receipt request', async () => {
  const older = createTimelineEvent('$older-private', { ts: 10 });
  const newer = createTimelineEvent('$newer-public', { ts: 20 });
  let liveEvents = [newer];
  let notifyStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  const writes: Array<[string, string, any, any]> = [];
  const room = {
    roomId: '!cross-channel-active:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: [string, string, any, any]) => {
      writes.push(args);
      if (writes.length === 1) {
        notifyStarted?.();
        await firstWriteBlocked;
      }
    },
  } as any;

  const publicRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  await firstWriteStarted;
  liveEvents = [older];
  let privateResolved = false;
  const privateRequest = markAsRead(mx, room.roomId, true, 'loaded-live-tail').then(() => {
    privateResolved = true;
  });
  await Promise.resolve();

  assert.equal(privateResolved, false);
  releaseFirst?.();
  await Promise.all([publicRequest, privateRequest]);
  assert.deepEqual(
    writes.map(([, fullyReadId, publicEvent, privateEvent]) => [
      fullyReadId,
      publicEvent?.getId(),
      privateEvent?.getId(),
    ]),
    [
      ['$newer-public', '$newer-public', undefined],
      ['$newer-public', undefined, '$older-private'],
    ]
  );
});

test('a completed public receipt never satisfies the private receipt channel', async () => {
  const older = createTimelineEvent('$completed-older-private', { ts: 10 });
  const newer = createTimelineEvent('$completed-newer-public', { ts: 20 });
  let liveEvents = [newer];
  const writes: Array<[string, string, any, any]> = [];
  const room = {
    roomId: '!cross-channel-completed:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: [string, string, any, any]) => {
      writes.push(args);
    },
  } as any;

  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  liveEvents = [older];
  await markAsRead(mx, room.roomId, true, 'loaded-live-tail');

  assert.deepEqual(
    writes.map(([, fullyReadId, publicEvent, privateEvent]) => [
      fullyReadId,
      publicEvent?.getId(),
      privateEvent?.getId(),
    ]),
    [
      ['$completed-newer-public', '$completed-newer-public', undefined],
      ['$completed-newer-public', undefined, '$completed-older-private'],
    ]
  );
});

test('interleaved detached read targets preserve both receipt channels at the furthest event', async () => {
  const first = createTimelineEvent('$detached-first-public', { ts: 10 });
  const second = createTimelineEvent('$detached-second-public', { ts: 20 });
  const third = createTimelineEvent('$detached-third-private', { ts: 30 });
  let notifyStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  const writes: Array<[string, string, any, any]> = [];
  const room = {
    roomId: '!detached-interleaving:example.org',
    accountData: { get: () => undefined },
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: [string, string, any, any]) => {
      writes.push(args);
      if (writes.length === 1) {
        notifyStarted?.();
        await firstWriteBlocked;
      }
    },
  } as any;

  const firstRequest = markAsReadAtEvent(mx, room.roomId, false, first);
  await firstWriteStarted;
  const privateRequest = markAsReadAtEvent(mx, room.roomId, true, third);
  const secondPublicRequest = markAsReadAtEvent(mx, room.roomId, false, second);
  releaseFirst?.();

  await Promise.all([firstRequest, privateRequest, secondPublicRequest]);
  assert.deepEqual(
    writes.map(([, fullyReadId, publicEvent, privateEvent]) => [
      fullyReadId,
      publicEvent?.getId(),
      privateEvent?.getId(),
    ]),
    [
      ['$detached-first-public', '$detached-first-public', undefined],
      ['$detached-third-private', '$detached-second-public', '$detached-third-private'],
    ]
  );
});

test('a failed public write neither poisons nor satisfies the private receipt channel', async () => {
  const oldest = createTimelineEvent('$failure-oldest-public', { ts: 5 });
  const first = createTimelineEvent('$failure-first-public', { ts: 10 });
  const second = createTimelineEvent('$failure-second-private', { ts: 20 });
  let liveEvents = [first];
  let notifyStarted: (() => void) | undefined;
  const firstWriteStarted = new Promise<void>((resolve) => {
    notifyStarted = resolve;
  });
  let releaseFirst: (() => void) | undefined;
  const firstWriteBlocked = new Promise<void>((resolve) => {
    releaseFirst = resolve;
  });
  const writes: Array<[string, string, any, any]> = [];
  const room = {
    roomId: '!cross-channel-failure:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => liveEvents }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: async (...args: [string, string, any, any]) => {
      writes.push(args);
      if (writes.length === 1) {
        notifyStarted?.();
        await firstWriteBlocked;
        throw new Error('public receipt failed');
      }
    },
  } as any;

  const publicRequest = markAsRead(mx, room.roomId, false, 'loaded-live-tail');
  await firstWriteStarted;
  liveEvents = [second];
  const privateRequest = markAsRead(mx, room.roomId, true, 'loaded-live-tail');
  const publicOutcome = assert.rejects(publicRequest, /public receipt failed/);
  releaseFirst?.();

  await publicOutcome;
  await privateRequest;
  liveEvents = [oldest];
  await markAsRead(mx, room.roomId, false, 'loaded-live-tail');

  assert.deepEqual(
    writes.map(([, fullyReadId, publicEvent, privateEvent]) => [
      fullyReadId,
      publicEvent?.getId(),
      privateEvent?.getId(),
    ]),
    [
      ['$failure-first-public', '$failure-first-public', undefined],
      ['$failure-second-private', undefined, '$failure-second-private'],
      ['$failure-second-private', '$failure-oldest-public', undefined],
    ]
  );
});

test('markAsReadInBackground consumes UI-only failures while markAsRead stays awaitable', async () => {
  const latest = createTimelineEvent('$background-failure', { ts: 1 });
  let rejectWrite: ((error: Error) => void) | undefined;
  const writeAttempt = new Promise<void>((_resolve, reject) => {
    rejectWrite = reject;
  });
  const room = {
    roomId: '!background-failure:example.org',
    accountData: { get: () => undefined },
    getLiveTimeline: () => ({ getEvents: () => [latest] }),
    compareEventOrdering: () => null,
  } as any;
  const mx = {
    getRoom: () => room,
    getUserId: () => '@alice:example.org',
    getAccountData: () => undefined,
    setRoomReadMarkers: () => writeAttempt,
  } as any;

  const backgroundResult = markAsReadInBackground(mx, room.roomId, false, 'loaded-live-tail');
  assert.equal(backgroundResult, undefined);
  rejectWrite?.(new Error('background write failed'));
  await new Promise<void>((resolve) => {
    setImmediate(resolve);
  });

  await assert.rejects(
    markAsRead(mx, room.roomId, false, 'loaded-live-tail'),
    /background write failed/
  );
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
