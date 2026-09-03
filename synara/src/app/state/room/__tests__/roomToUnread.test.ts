import assert from 'node:assert/strict';
import test from 'node:test';
import type { RoomSummary } from '../../../features/matrix-dto/room';
import { unreadFromNativeRoom, unreadInfosFromNativeRooms } from '../roomToUnread';

const room = (overrides: Partial<RoomSummary> & Pick<RoomSummary, 'roomId'>): RoomSummary => ({
  membership: 'join',
  isDirect: false,
  isSpace: false,
  isCall: false,
  isFavorite: false,
  isEncrypted: false,
  encryptionStatus: 'not_encrypted',
  unreadCount: 0,
  highlightCount: 0,
  markedUnread: false,
  ...overrides,
});

test('native unread projection keeps joined rooms with counts or marked-unread', () => {
  const infos = unreadInfosFromNativeRooms([
    room({ roomId: '!a:example.org', unreadCount: 2, highlightCount: 1 }),
    room({ roomId: '!b:example.org', markedUnread: true }),
    room({ roomId: '!c:example.org' }),
  ]);
  assert.deepEqual(infos, [
    { roomId: '!a:example.org', highlight: 1, total: 2 },
    { roomId: '!b:example.org', highlight: 0, total: 0 },
  ]);
});

test('native unread projection raises total to the highlight count', () => {
  const infos = unreadInfosFromNativeRooms([
    room({ roomId: '!a:example.org', unreadCount: 1, highlightCount: 4 }),
  ]);
  assert.deepEqual(infos, [{ roomId: '!a:example.org', highlight: 4, total: 4 }]);
});

test('native unread projection skips spaces, muted rooms, and non-joined membership', () => {
  const infos = unreadInfosFromNativeRooms([
    room({ roomId: '!space:example.org', isSpace: true, unreadCount: 3 }),
    room({ roomId: '!mute:example.org', notificationMode: 'mute', unreadCount: 5 }),
    room({ roomId: '!leave:example.org', membership: 'leave', unreadCount: 2 }),
  ]);
  assert.deepEqual(infos, []);
});

test('native unread projection drops a room after receipts clear counts and marked-unread', () => {
  const before = unreadInfosFromNativeRooms([
    room({ roomId: '!a:example.org', unreadCount: 2, highlightCount: 1, markedUnread: true }),
  ]);
  assert.deepEqual(before, [{ roomId: '!a:example.org', highlight: 1, total: 2 }]);

  const after = unreadInfosFromNativeRooms([
    room({ roomId: '!a:example.org', unreadCount: 0, highlightCount: 0, markedUnread: false }),
  ]);
  assert.deepEqual(after, []);
  assert.equal(
    unreadFromNativeRoom(
      room({ roomId: '!a:example.org', unreadCount: 0, highlightCount: 0, markedUnread: false })
    ),
    undefined
  );
});

test('nav unread comes from native unreadCount, not a leftover jotai total', () => {
  const unread = unreadFromNativeRoom(
    room({ roomId: '!a:example.org', unreadCount: 4, highlightCount: 1 })
  );
  assert.deepEqual(unread, { highlight: 1, total: 4, from: null });
});
