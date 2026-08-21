import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const source = readFileSync('src/app/features/room-nav/RoomNavItem.tsx', 'utf8');

test('RoomNavItem mark-as-read hits the native owner instead of the JS-sdk no-op', () => {
  assert.match(source, /setRoomReadStateWithNativeOwner/);
  assert.match(source, /unreadFromNativeRoom/);
  assert.match(source, /useNativeRoomListSnapshot/);
  assert.match(source, /'mark_read'/);
  assert.match(source, /'mark_unread'/);
  assert.doesNotMatch(source, /markAsReadInBackground/);
  assert.doesNotMatch(source, /sendReadReceipt/);
  assert.doesNotMatch(source, /setRoomReadMarkers/);
  assert.doesNotMatch(source, /dual_backend/);
});

test('native room read-state owner invokes the room-level native command', () => {
  const owner = readFileSync('src/app/utils/nativeRoomReadStateOwner.ts', 'utf8');
  assert.match(owner, /matrix_room_set_read_state/);
  assert.match(owner, /mark_read/);
  assert.match(owner, /mark_unread/);
  assert.doesNotMatch(owner, /sendReadReceipt/);
  assert.doesNotMatch(owner, /setRoomReadMarkers/);
});
