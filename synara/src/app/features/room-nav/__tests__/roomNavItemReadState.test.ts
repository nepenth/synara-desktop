import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const source = readFileSync('src/app/features/room-nav/RoomNavItem.tsx', 'utf8');

test('RoomNavItem mark-as-read hits the native owner instead of the JS-sdk no-op', () => {
  assert.match(source, /setRoomReadStateWithNativeOwner/);
  assert.match(source, /await markAsReadFromExplicitUserAction\(mx, room\.roomId\)/);
  assert.match(source, /Couldn't mark this channel as read\./);
  assert.doesNotMatch(source, /markAsReadFromExplicitUserActionInBackground/);
  const markAsRead = source.slice(
    source.indexOf('const handleMarkAsRead'),
    source.indexOf('const handleMarkAsUnread')
  );
  assert.match(markAsRead, /await markAsReadFromExplicitUserAction/);
  const awaitIndex = markAsRead.indexOf('await markAsReadFromExplicitUserAction');
  const closeIndex = markAsRead.indexOf('requestClose()');
  assert.ok(awaitIndex >= 0 && closeIndex > awaitIndex);
  assert.match(source, /unreadFromNativeRoom/);
  assert.match(source, /useNativeRoomListSnapshot/);
  assert.match(source, /LiveCallChip/);
  assert.match(source, /hasActiveCall/);
  assert.match(source, /useNativeUserStatus/);
  assert.match(source, /directUserId/);
  assert.doesNotMatch(source, /experimental-widgets/);
  assert.match(source, /'mark_unread'/);
  assert.doesNotMatch(source, /markAsReadInBackground/);
  assert.doesNotMatch(source, /sendReadReceipt/);
  assert.doesNotMatch(source, /setRoomReadMarkers/);
  assert.doesNotMatch(source, /dual_backend/);
});

test('RoomViewHeader mark-as-read awaits the explicit action and surfaces failure', () => {
  const header = readFileSync('src/app/features/room/RoomViewHeader.tsx', 'utf8');
  const markAsRead = header.slice(
    header.indexOf('const handleMarkAsRead'),
    header.indexOf('const handleMarkAsUnread')
  );
  assert.match(markAsRead, /await markAsReadFromExplicitUserAction/);
  assert.match(header, /Couldn't mark this channel as read\./);
  assert.doesNotMatch(markAsRead, /markAsReadFromExplicitUserActionInBackground/);
  const awaitIndex = markAsRead.indexOf('await markAsReadFromExplicitUserAction');
  const closeIndex = markAsRead.indexOf('requestClose()');
  assert.ok(awaitIndex >= 0 && closeIndex > awaitIndex);
});

test('native room read-state owner invokes the room-level native command', () => {
  const owner = readFileSync('src/app/utils/nativeRoomReadStateOwner.ts', 'utf8');
  assert.match(owner, /matrix_room_set_read_state/);
  assert.match(owner, /mark_read/);
  assert.match(owner, /mark_unread/);
  assert.doesNotMatch(owner, /sendReadReceipt/);
  assert.doesNotMatch(owner, /setRoomReadMarkers/);
});
