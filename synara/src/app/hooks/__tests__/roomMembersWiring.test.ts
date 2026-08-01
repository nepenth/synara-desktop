import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const readSource = (path: string): string => readFileSync(path, 'utf8');

test('member surfaces select the native room-member owner on desktop sessions', () => {
  const drawer = readSource('src/app/features/room/MembersDrawer.tsx');
  const autocomplete = readSource(
    'src/app/components/editor/autocomplete/UserMentionAutocomplete.tsx'
  );
  const room = readSource('src/app/features/room/Room.tsx');
  const lobby = readSource('src/app/features/lobby/Lobby.tsx');

  assert.match(drawer, /useRoomMembers\(mx, room\.roomId, nativeSession\)/);
  assert.match(autocomplete, /useRoomMembers\(mx, roomId, nativeSession\)/);
  assert.doesNotMatch(room, /useRoomMembers/);
  assert.doesNotMatch(lobby, /useRoomMembers/);
});

test('native room-member hook branch is fail-closed before the JS room read', () => {
  const hook = readSource('src/app/hooks/useRoomMembers.ts');
  const nativeBranch = hook.match(/if \(nativeSession\) \{([\s\S]*?)\n\s{4}\}\n\n\s{4}const room/);

  assert.ok(nativeBranch, 'expected a separate nativeSession hook branch');
  assert.match(nativeBranch[1], /readRoomMembersWithNativeOwner\(roomId, true\)/);
  assert.doesNotMatch(nativeBranch[1], /mx\.getRoom\(roomId\)/);
});
