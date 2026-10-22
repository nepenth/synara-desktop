import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { createStore } from 'jotai';
import type { RoomSummary } from '../../../features/matrix-dto/room';
import {
  nativeRoomListSnapshotAtom,
  nativeRoomSummaryAtom,
  nativeRoomUnreadAtom,
  type NativeRoomListSnapshot,
} from '../roomList';
import { EMPTY_ROOM_LIST_PRESENTATION } from '../roomListPresentation';

const room = (roomId: string, overrides: Partial<RoomSummary> = {}): RoomSummary => ({
  roomId,
  name: roomId,
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
});

const snapshot = (
  rooms: RoomSummary[],
  unread: { roomId: string; highlight: number; total: number }[] = []
): NativeRoomListSnapshot => ({
  sessionGeneration: 1,
  orderedRoomIds: rooms.map((summary) => summary.roomId),
  rooms,
  presentation: { ...EMPTY_ROOM_LIST_PRESENTATION, unread },
});

test('D10: a room subscription ignores other rooms and fires on its own change', () => {
  const store = createStore();
  store.set(nativeRoomListSnapshotAtom, snapshot([room('!a'), room('!b')]));
  let summaryWakes = 0;
  let unreadWakes = 0;
  const stopSummary = store.sub(nativeRoomSummaryAtom('!a'), () => {
    summaryWakes += 1;
  });
  const stopUnread = store.sub(nativeRoomUnreadAtom('!a'), () => {
    unreadWakes += 1;
  });
  const first = store.get(nativeRoomSummaryAtom('!a'));

  // Activity in another room publishes a new snapshot with fresh objects.
  store.set(
    nativeRoomListSnapshotAtom,
    snapshot(
      [room('!a'), room('!b', { unreadCount: 3 })],
      [{ roomId: '!b', highlight: 0, total: 3 }]
    )
  );
  assert.equal(summaryWakes, 0);
  assert.equal(unreadWakes, 0);
  assert.equal(store.get(nativeRoomSummaryAtom('!a')), first, 'identity kept for an equal summary');

  store.set(
    nativeRoomListSnapshotAtom,
    snapshot(
      [room('!a', { encryptionStatus: 'encrypted' }), room('!b', { unreadCount: 3 })],
      [
        { roomId: '!a', highlight: 1, total: 1 },
        { roomId: '!b', highlight: 0, total: 3 },
      ]
    )
  );
  assert.equal(summaryWakes, 1);
  assert.equal(unreadWakes, 1);
  assert.equal(store.get(nativeRoomSummaryAtom('!a'))?.encryptionStatus, 'encrypted');
  assert.deepEqual(store.get(nativeRoomUnreadAtom('!a')), { highlight: 1, total: 1, from: null });
  assert.equal(nativeRoomSummaryAtom('!a'), nativeRoomSummaryAtom('!a'), 'one atom per room');

  stopSummary();
  stopUnread();
});

test('D10: single-room consumers subscribe to their room, not the whole list', () => {
  const read = (path: string) => readFileSync(`src/app/${path}`, 'utf8');
  for (const path of [
    'features/room/NativeTimelinePresenter.tsx',
    'features/room/RoomInput.tsx',
    'features/room/VoiceRoom.tsx',
    'features/room/RoomViewHeader.tsx',
    'features/common-settings/general/RoomEncryption.tsx',
  ]) {
    assert.doesNotMatch(read(path), /rooms\.find\(/, path);
  }
  const navItem = read('features/room-nav/RoomNavItem.tsx');
  assert.doesNotMatch(navItem, /useNativeRoomListSnapshot/);
  assert.match(navItem, /useNativeRoomSummary\(room\.roomId\)/);
  assert.match(
    read('features/room/NativeTimelinePresenter.tsx'),
    /const NativeTimelineRow = React\.memo\(NativeTimelineRowImpl\)/
  );
  assert.match(read('hooks/useRoomTypingMembers.ts'), /useMemo\(\s*\(\) => selectAtom/);
});
