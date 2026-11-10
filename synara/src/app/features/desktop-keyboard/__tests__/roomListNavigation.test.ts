import assert from 'node:assert/strict';
import test from 'node:test';
import {
  adjacentRoomTarget,
  adjacentUnreadTarget,
  currentRoomListNavSource,
  registerRoomListNavSource,
  roomListNavTargets,
  type RoomListNavSource,
} from '../roomListNavigation';
import { registerAppShortcutHandler, runAppShortcutHandler } from '../appShortcutHandlers';

const rooms = (count: number, prefix = 'room') =>
  Array.from({ length: count }, (_, index) => `!${prefix}-${index}:example.test`);

/** Home's shape: favorites, then two virtual sections. */
const home = (selectedRoomId?: string): RoomListNavSource => ({
  selectedRoomId,
  sections: [
    { rows: ['!fav:example.test'], pathFor: (id) => `/home/${id}/` },
    { rows: rooms(500), pathFor: (id) => `/home/${id}/`, reveal: () => undefined },
    { rows: rooms(3, 'dm'), pathFor: (id) => `/direct/${id}/`, reveal: () => undefined },
  ],
});

test('targets cover every row of every section, not only the rendered ones', () => {
  const targets = roomListNavTargets(home());
  assert.equal(targets.length, 1 + 500 + 3);
  assert.equal(targets[0].roomId, '!fav:example.test');
  assert.equal(targets[400].roomId, '!room-399:example.test');
  assert.equal(targets[400].index, 399);
  assert.equal(targets[400].path, '/home/!room-399:example.test/');
  assert.equal(targets.at(-1)?.path, '/direct/!dm-2:example.test/');
});

test('previous and next room step through the data across sections, without wrapping', () => {
  const at = (selected: string | undefined, direction: 'previous' | 'next') =>
    adjacentRoomTarget(roomListNavTargets(home(selected)), selected, direction)?.roomId;
  assert.equal(at('!room-250:example.test', 'next'), '!room-251:example.test');
  assert.equal(at('!room-250:example.test', 'previous'), '!room-249:example.test');
  // Section boundaries.
  assert.equal(at('!fav:example.test', 'next'), '!room-0:example.test');
  assert.equal(at('!room-0:example.test', 'previous'), '!fav:example.test');
  assert.equal(at('!room-499:example.test', 'next'), '!dm-0:example.test');
  // The ends do not wrap.
  assert.equal(at('!fav:example.test', 'previous'), undefined);
  assert.equal(at('!dm-2:example.test', 'next'), undefined);
  // No open room in this list: the first or the last room, far outside any window.
  assert.equal(at(undefined, 'next'), '!fav:example.test');
  assert.equal(at(undefined, 'previous'), '!dm-2:example.test');
  assert.equal(at('!elsewhere:example.test', 'previous'), '!dm-2:example.test');
  assert.equal(adjacentRoomTarget([], undefined, 'next'), undefined);
});

test('header rows of a space list are skipped but keep their virtual index', () => {
  const source: RoomListNavSource = {
    selectedRoomId: '!a:example.test',
    sections: [
      {
        rows: [undefined, '!a:example.test', undefined, '!b:example.test'],
        pathFor: (id) => `/s/${id}/`,
      },
    ],
  };
  const target = adjacentRoomTarget(roomListNavTargets(source), '!a:example.test', 'next');
  assert.equal(target?.roomId, '!b:example.test');
  assert.equal(target?.index, 3);
});

test('a collapsed section contributes only the rows it shows', () => {
  // Home reduces a closed section to its unread and selected rows before publishing.
  const source: RoomListNavSource = {
    selectedRoomId: '!a:example.test',
    sections: [
      { rows: ['!a:example.test', '!unread:example.test'], pathFor: (id) => id },
      { rows: ['!c:example.test'], pathFor: (id) => id },
    ],
  };
  const targets = roomListNavTargets(source);
  assert.equal(
    adjacentRoomTarget(targets, '!a:example.test', 'next')?.roomId,
    '!unread:example.test'
  );
  assert.equal(
    adjacentRoomTarget(targets, '!unread:example.test', 'next')?.roomId,
    '!c:example.test'
  );
});

test('next and previous unread wrap once and never pick the open room', () => {
  const unread = new Set(['!room-3:example.test', '!room-420:example.test', '!dm-1:example.test']);
  const isUnread = (id: string) => unread.has(id);
  const at = (selected: string | undefined, direction: 'previous' | 'next') =>
    adjacentUnreadTarget(roomListNavTargets(home(selected)), selected, direction, isUnread)?.roomId;
  assert.equal(at('!room-3:example.test', 'next'), '!room-420:example.test');
  assert.equal(at('!room-420:example.test', 'next'), '!dm-1:example.test');
  // Wraps past the end, and past the start going up.
  assert.equal(at('!dm-1:example.test', 'next'), '!room-3:example.test');
  assert.equal(at('!room-3:example.test', 'previous'), '!dm-1:example.test');
  assert.equal(at('!room-100:example.test', 'previous'), '!room-3:example.test');
  // Without an open room: from the top going down, from the bottom going up.
  assert.equal(at(undefined, 'next'), '!room-3:example.test');
  assert.equal(at(undefined, 'previous'), '!dm-1:example.test');
  // The open room is unread but is the only one: nothing to jump to.
  const only = (id: string) => id === '!room-3:example.test';
  assert.equal(
    adjacentUnreadTarget(
      roomListNavTargets(home('!room-3:example.test')),
      '!room-3:example.test',
      'next',
      only
    ),
    undefined
  );
  assert.equal(
    adjacentUnreadTarget([], undefined, 'next', () => true),
    undefined
  );
});

test('the mounted list is the latest registration, and unmounting restores the previous', () => {
  assert.equal(currentRoomListNavSource(), undefined);
  const first = home();
  const second = home('!room-1:example.test');
  const releaseFirst = registerRoomListNavSource(first);
  const releaseSecond = registerRoomListNavSource(second);
  assert.equal(currentRoomListNavSource(), second);
  // A route change can mount the next list before the old one unmounts.
  releaseFirst();
  assert.equal(currentRoomListNavSource(), second);
  releaseSecond();
  assert.equal(currentRoomListNavSource(), undefined);
});

test('component handlers: the latest mounted one runs; none means the key is not used', () => {
  assert.equal(runAppShortcutHandler('close-thread'), false);
  const calls: string[] = [];
  const releaseA = registerAppShortcutHandler('close-thread', () => {
    calls.push('a');
    return true;
  });
  const releaseB = registerAppShortcutHandler('close-thread', () => {
    calls.push('b');
    return false;
  });
  assert.equal(runAppShortcutHandler('close-thread'), false);
  releaseB();
  assert.equal(runAppShortcutHandler('close-thread'), true);
  releaseA();
  assert.equal(runAppShortcutHandler('close-thread'), false);
  assert.deepEqual(calls, ['b', 'a']);
});
