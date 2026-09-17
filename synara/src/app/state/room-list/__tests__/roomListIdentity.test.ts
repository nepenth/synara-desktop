import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import type { RoomSummary } from '../../../features/matrix-dto/room';
import { sameNativeRoomListSnapshot, sameStringList } from '../roomList';

const room = (overrides: Partial<RoomSummary> & Pick<RoomSummary, 'roomId'>): RoomSummary =>
  ({
    membership: 'join',
    isDirect: false,
    isSpace: false,
    isCall: false,
    hasActiveCall: false,
    activeCallParticipantCount: 0,
    isFavorite: false,
    isEncrypted: false,
    encryptionStatus: 'not_encrypted',
    unreadCount: 0,
    highlightCount: 0,
    markedUnread: false,
    lastMessageIsAgentApproval: false,
    ...overrides,
  } as RoomSummary);

test('sameStringList compares order and identity, not array reference', () => {
  const first = ['!a', '!b'];
  assert.equal(sameStringList(first, ['!a', '!b']), true);
  assert.equal(sameStringList(first, first), true);
  assert.equal(sameStringList(first, ['!b', '!a']), false);
  assert.equal(sameStringList(first, ['!a']), false);
});

test('sameNativeRoomListSnapshot ignores a new object with the same unread-bearing fields', () => {
  const snapshot = {
    sessionGeneration: 3,
    orderedRoomIds: ['!a'],
    rooms: [room({ roomId: '!a', unreadCount: 2, name: 'Room' })],
  };
  assert.equal(
    sameNativeRoomListSnapshot(snapshot, {
      sessionGeneration: 3,
      orderedRoomIds: ['!a'],
      rooms: [room({ roomId: '!a', unreadCount: 2, name: 'Room' })],
    }),
    true
  );
  assert.equal(
    sameNativeRoomListSnapshot(snapshot, {
      sessionGeneration: 3,
      orderedRoomIds: ['!a'],
      rooms: [room({ roomId: '!a', unreadCount: 3, name: 'Room' })],
    }),
    false
  );
});

test('room list poll skips atom writes when the snapshot is unchanged', () => {
  const source = readFileSync('src/app/state/room-list/roomList.ts', 'utf8');
  assert.match(source, /if \(sameStringList\(current, action\.rooms\)\) return/);
  assert.match(
    source,
    /if \(sameNativeRoomListSnapshot\(latestNativeRoomListSnapshot, snapshot\)\) return/
  );
});
