import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const panel = readFileSync('src/app/features/room/RoomThreadListPanel.tsx', 'utf8');
const owner = readFileSync('src/app/features/room/nativeThreadList.ts', 'utf8');
const header = readFileSync('src/app/features/room/RoomViewHeader.tsx', 'utf8');
const sidePanel = readFileSync('src/app/features/room/RoomSidePanel.tsx', 'utf8');
const router = readFileSync('src/app/pages/Router.tsx', 'utf8');
const paths = readFileSync('src/app/pages/paths.ts', 'utf8');
const roomView = readFileSync('src/app/features/room/RoomView.tsx', 'utf8');
const navigate = readFileSync('src/app/hooks/useRoomNavigate.ts', 'utf8');

test('native thread list command is ids-and-counts only', () => {
  assert.match(owner, /NATIVE_THREAD_LIST_SCHEMA_VERSION = 1/);
  assert.match(owner, /invokeDesktopWithAvailability\('matrix_thread_list'/);
  assert.match(owner, /action: NativeThreadListAction/);
  assert.match(owner, /'open' \| 'paginate' \| 'close'/);
  assert.doesNotMatch(owner, /prev_batch|next_batch|prevBatch|nextBatch/);
});

test('thread list panel paginates without claiming completeness', () => {
  assert.match(panel, /nativeThreadList\(room.roomId, action\)/);
  assert.match(panel, /nativeThreadList\(room.roomId, 'close'\)/);
  assert.match(panel, /Load more/);
  assert.match(panel, /Showing the 256 most recently active threads/);
  assert.match(panel, /navigateThread\(room.roomId, thread.rootEventId\)/);
  assert.match(panel, /thread.unreadCount \? ` · \$\{thread.unreadCount\} unread` : ''/);
  assert.doesNotMatch(panel, /all threads/);
});

test('room header and side panel expose a threads surface', () => {
  assert.match(header, /aria-label="Threads"/);
  assert.match(header, /Icons\.Thread/);
  assert.match(header, /onToggleSidePanel\('threads'\)/);
  assert.match(sidePanel, /'notes' \| 'pins' \| 'search' \| 'threads'/);
  assert.match(sidePanel, /<RoomThreadListPanel requestClose=\{requestClose\} \/>/);
});

test('react router honors /thread/ before the optional event permalink', () => {
  assert.match(paths, /:roomIdOrAlias\/thread\/:threadRootId\//);
  const homeThread = router.indexOf('path={_ROOM_THREAD_PATH}');
  const homeRoom = router.indexOf('path={_ROOM_PATH}');
  assert.ok(homeThread >= 0 && homeRoom > homeThread);
  assert.match(navigate, /getHomeRoomThreadPath/);
  assert.match(navigate, /getDirectRoomThreadPath/);
  assert.match(navigate, /getSpaceRoomThreadPath/);
  assert.match(roomView, /threadRootEventId=\{threadRootEventId\}/);
  assert.match(roomView, /onOpenThreadRoute=\{\(rootEventId\) => navigateThread\(roomId, rootEventId\)\}/);
});
