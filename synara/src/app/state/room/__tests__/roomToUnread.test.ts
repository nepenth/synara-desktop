import assert from 'node:assert/strict';
import test from 'node:test';
import type { RoomListPresentation } from '../../../features/matrix-dto/generated';
import {
  EMPTY_ROOM_LIST_PRESENTATION,
  orderRoomIdsByPresentation,
  parseRoomListPresentation,
  unreadFromPresentation,
} from '../../room-list/roomListPresentation';
import { roomToUnreadFromPresentation, unreadInfosFromPresentation } from '../roomToUnread';

// Which rooms need attention, and how much, is decided by Core and tested in
// Rust (`room_list::presentation`). These tests cover the renderer readers.
const presentation: RoomListPresentation = {
  ...EMPTY_ROOM_LIST_PRESENTATION,
  unread: [
    { roomId: '!a:example.org', highlight: 1, total: 2 },
    { roomId: '!b:example.org', highlight: 0, total: 1 },
    {
      roomId: '!space:example.org',
      highlight: 1,
      total: 3,
      fromRoomIds: ['!a:example.org', '!b:example.org'],
    },
  ],
};

test('room unread reads Core rows, and a room without a row needs no attention', () => {
  assert.deepEqual(unreadFromPresentation(presentation, '!a:example.org'), {
    highlight: 1,
    total: 2,
    from: null,
  });
  assert.equal(unreadFromPresentation(presentation, '!c:example.org'), undefined);
});

test('space rollup rows keep the rooms they came from', () => {
  assert.deepEqual(unreadFromPresentation(presentation, '!space:example.org'), {
    highlight: 1,
    total: 3,
    from: new Set(['!a:example.org', '!b:example.org']),
  });
  const map = roomToUnreadFromPresentation(presentation);
  assert.equal(map.size, 3);
  assert.equal(map.get('!space:example.org')?.total, 3);
});

test('unread infos exclude space rollups and honour a room filter', () => {
  assert.deepEqual(unreadInfosFromPresentation(presentation), [
    { roomId: '!a:example.org', highlight: 1, total: 2 },
    { roomId: '!b:example.org', highlight: 0, total: 1 },
  ]);
  assert.deepEqual(unreadInfosFromPresentation(presentation, new Set(['!b:example.org'])), [
    { roomId: '!b:example.org', highlight: 0, total: 1 },
  ]);
});

test('presentation parsing defaults an absent field and rejects malformed rows', () => {
  assert.deepEqual(parseRoomListPresentation(undefined), EMPTY_ROOM_LIST_PRESENTATION);
  assert.equal(
    parseRoomListPresentation({
      ...presentation,
      unread: [{ roomId: '!a', total: -1, highlight: 0 }],
    }),
    null
  );
  assert.equal(parseRoomListPresentation({ ...presentation, nameOrder: 'nope' }), null);
  assert.deepEqual(parseRoomListPresentation(presentation), presentation);
});

test('ordering keeps ids Core did not list at the end in their original order', () => {
  const order = { ...EMPTY_ROOM_LIST_PRESENTATION, recentOrder: ['!b', '!a'] };
  assert.deepEqual(orderRoomIdsByPresentation(['!x', '!a', '!y', '!b'], order, 'recent'), [
    '!b',
    '!a',
    '!x',
    '!y',
  ]);
});
