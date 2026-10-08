import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  createRoomListUpdateSource,
  EVENT_FALLBACK_HIDDEN_POLL_MS,
  EVENT_FALLBACK_VISIBLE_POLL_MS,
  parseNativeRoomListUpdate,
  ROOM_LIST_UPDATED_EVENT,
  startRoomListUpdateDrivenPoll,
  type NativeRoomListUpdate,
} from '../nativeRoomListUpdates';
import { HIDDEN_POLL_INTERVAL_MS, VISIBLE_POLL_INTERVAL_MS } from '../visibilityPoll';

type Handler = (event: { payload: unknown }) => void;

const fakeListen = () => {
  const handlers = new Set<Handler>();
  let resolveListen: (() => void) | undefined;
  let rejectListen: (() => void) | undefined;
  let registrations = 0;
  let unlistens = 0;
  const listenTo = (event: string, handler: Handler) => {
    assert.equal(event, ROOM_LIST_UPDATED_EVENT);
    registrations += 1;
    return new Promise<() => void>((resolve, reject) => {
      resolveListen = () => {
        handlers.add(handler);
        resolve(() => {
          unlistens += 1;
          handlers.delete(handler);
        });
      };
      rejectListen = () => reject(new Error('unavailable'));
    });
  };
  return {
    listenTo,
    live: async () => {
      resolveListen?.();
      await Promise.resolve();
      await Promise.resolve();
    },
    fail: async () => {
      rejectListen?.();
      await Promise.resolve();
      await Promise.resolve();
    },
    emit: (payload: unknown) => handlers.forEach((handler) => handler({ payload })),
    registrations: () => registrations,
    unlistens: () => unlistens,
  };
};

const fakePoll = () => {
  const polls: { visibleMs: number; hiddenMs: number; stopped: boolean }[] = [];
  const startPoll = (_tick: () => void, visibleMs?: number, hiddenMs?: number) => {
    const poll = { visibleMs: visibleMs ?? -1, hiddenMs: hiddenMs ?? -1, stopped: false };
    polls.push(poll);
    return () => {
      poll.stopped = true;
    };
  };
  return { polls, startPoll };
};

test('parser accepts exactly a generation and a revision', () => {
  assert.deepEqual(parseNativeRoomListUpdate({ sessionGeneration: 3, revision: 9 }), {
    sessionGeneration: 3,
    revision: 9,
  });
  assert.equal(parseNativeRoomListUpdate({ sessionGeneration: 3 }), undefined);
  assert.equal(
    parseNativeRoomListUpdate({ sessionGeneration: 3, revision: 1, roomId: '!a:b' }),
    undefined
  );
  assert.equal(parseNativeRoomListUpdate({ sessionGeneration: -1, revision: 1 }), undefined);
  assert.equal(parseNativeRoomListUpdate({ sessionGeneration: 1, revision: 1.5 }), undefined);
  assert.equal(parseNativeRoomListUpdate(null), undefined);
});

test('updates drive refreshes and the safety poll stays slow', async () => {
  const listen = fakeListen();
  const poll = fakePoll();
  const source = createRoomListUpdateSource(listen.listenTo);
  let ticks = 0;
  const stop = startRoomListUpdateDrivenPoll(() => (ticks += 1), {
    source,
    startPoll: poll.startPoll,
    isDesktop: () => true,
  });
  assert.equal(poll.polls.length, 1);
  assert.equal(poll.polls[0].visibleMs, EVENT_FALLBACK_VISIBLE_POLL_MS);
  assert.equal(poll.polls[0].hiddenMs, EVENT_FALLBACK_HIDDEN_POLL_MS);
  assert.equal(ticks, 0, 'callers refresh on mount themselves');

  await listen.live();
  assert.equal(ticks, 1, 'going live runs one catch-up tick');

  listen.emit({ sessionGeneration: 2, revision: 1 });
  listen.emit({ sessionGeneration: 2, revision: 2 });
  assert.equal(ticks, 3);
  listen.emit({ roomId: '!leak:example.org' });
  assert.equal(ticks, 3, 'malformed payloads are ignored');

  stop();
  assert.equal(poll.polls[0].stopped, true);
  assert.equal(listen.unlistens(), 1, 'the last subscriber releases the listener');
  listen.emit({ sessionGeneration: 2, revision: 3 });
  assert.equal(ticks, 3);
});

test('all lists share one Tauri listener', async () => {
  const listen = fakeListen();
  const poll = fakePoll();
  const source = createRoomListUpdateSource(listen.listenTo);
  const ticks: number[] = [0, 0];
  const options = { source, startPoll: poll.startPoll, isDesktop: () => true };
  const stopFirst = startRoomListUpdateDrivenPoll(() => (ticks[0] += 1), options);
  const stopSecond = startRoomListUpdateDrivenPoll(() => (ticks[1] += 1), options);
  assert.equal(listen.registrations(), 1);
  await listen.live();
  listen.emit({ sessionGeneration: 1, revision: 1 } satisfies NativeRoomListUpdate);
  assert.deepEqual(ticks, [2, 2]);
  stopFirst();
  assert.equal(listen.unlistens(), 0);
  listen.emit({ sessionGeneration: 1, revision: 2 });
  assert.deepEqual(ticks, [2, 3]);
  stopSecond();
  assert.equal(listen.unlistens(), 1);
});

test('a stream that cannot listen falls back to the regular cadence', async () => {
  const listen = fakeListen();
  const poll = fakePoll();
  const source = createRoomListUpdateSource(listen.listenTo);
  const stop = startRoomListUpdateDrivenPoll(() => undefined, {
    source,
    startPoll: poll.startPoll,
    isDesktop: () => true,
  });
  await listen.fail();
  assert.equal(poll.polls.length, 2);
  assert.equal(poll.polls[0].stopped, true);
  assert.equal(poll.polls[1].visibleMs, VISIBLE_POLL_INTERVAL_MS);
  assert.equal(poll.polls[1].hiddenMs, HIDDEN_POLL_INTERVAL_MS);
  stop();
  assert.equal(poll.polls[1].stopped, true);
});

test('sync-derived lists use the event stream and typing keeps its own poll', () => {
  for (const file of [
    'src/app/state/room-list/roomList.ts',
    'src/app/state/room-list/inviteList.ts',
    'src/app/state/mDirectList.ts',
    'src/app/state/laterList.ts',
    'src/app/state/roomNotesList.ts',
    'src/app/state/room/roomToParents.ts',
    'src/app/hooks/useSpaceHierarchy.ts',
  ]) {
    const source = readFileSync(file, 'utf8');
    assert.match(source, /startRoomListUpdateDrivenPoll\(\(\) => void refresh\(\)\)/, file);
    assert.doesNotMatch(source, /startVisibilityAwarePoll/, file);
  }
  const typing = readFileSync('src/app/state/typingMembers.ts', 'utf8');
  assert.match(typing, /startVisibilityAwarePoll/);
});
