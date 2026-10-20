import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (path: string): string => readFileSync(`src/app/${path}`, 'utf8');

/** User actions whose failures were swallowed (review D89-D98). */
const ACTION_FILES = [
  'features/lobby/Lobby.tsx',
  'features/room/RoomViewHeader.tsx',
  'features/room-nav/RoomNavItem.tsx',
  'features/room/room-notes/RoomNotesPanel.tsx',
  'pages/client/inbox/Later.tsx',
  'pages/client/sidebar/SpaceTabs.tsx',
  'components/user-profile/UserChips.tsx',
];

test('user actions no longer swallow failures', () => {
  for (const path of ACTION_FILES) {
    assert.doesNotMatch(read(path), /\.catch\(\(\) => undefined\)/, path);
  }
});

test('Add existing rooms reports partial failures and keeps the failed rooms selected', () => {
  const source = read('features/add-existing/AddExisting.tsx');
  assert.match(source, /const failures = await rateLimitedActions\(roomsWithVia/);
  assert.match(source, /throw new BulkActionError\(/);
  assert.match(
    source,
    /setSelected\(\(current\) => current\.filter\(\(rId\) => failedIds\.has\(rId\)\)\)/
  );
  assert.match(source, /role="alert"/);
});

test('bulk invite actions continue past failures and say which failed', () => {
  const source = read('pages/client/inbox/Invites.tsx');
  assert.doesNotMatch(source, /for \(const invite of/);
  // Public Decline All, and spam Decline, Report and Block All.
  assert.equal(source.match(/runInviteBulkAction\(/g)?.length, 4);
  assert.match(source, /<BulkInviteError error=\{declineAllStatus\.error\} \/>/);
  assert.match(source, /<BulkInviteError error=\{reportAllStatus\.error\} \/>/);
  assert.match(source, /<BulkInviteError error=\{blockAllStatus\.error\} \/>/);
  // A retried Report All skips invites that were already reported.
  assert.match(source, /reportedRoomIds\.current\.has\(invite\.roomId\)/);
});

test('block and unblock failures are shown, and a failed list load is not an empty list', () => {
  const list = read('features/settings/account/IgnoredUserList.tsx');
  assert.match(list, /Couldn't block \$\{uId\}/);
  assert.match(list, /Couldn't unblock \$\{userId\}/);
  assert.match(list, /Couldn't load your blocked users/);
  const chips = read('components/user-profile/UserChips.tsx');
  assert.doesNotMatch(chips, /setNativeIgnoredIds\(\[\]\)/);
  assert.match(chips, /disabled=\{ignoring \|\| !ignoreKnown\}/);
  assert.match(chips, /toggleIgnore\(\)\.then\(close, \(\) => \{\}\)/);
});

test('sidebar layout writes revert at once and show the failure', () => {
  const source = read('pages/client/sidebar/SpaceTabs.tsx');
  assert.match(source, /localEchoSidebarItem\(previous\);\s*setSaveError\(failure\);/);
  assert.doesNotMatch(source, /void saveSynaraSpacesContent/);
});

test('mark read and unread report failures in the menu that started them', () => {
  for (const path of [
    'pages/client/sidebar/HomeTab.tsx',
    'pages/client/sidebar/DirectTab.tsx',
    'pages/client/sidebar/SpaceTabs.tsx',
    'pages/client/home/Home.tsx',
    'pages/client/direct/Direct.tsx',
    'pages/client/space/Space.tsx',
  ]) {
    const source = read(path);
    assert.doesNotMatch(source, /markAsReadFromExplicitUserActionInBackground/, path);
    assert.match(source, /useMarkRoomsAsRead\(\)/, path);
    assert.match(source, /\{markReadError\}/, path);
  }
  for (const path of ['features/room/RoomViewHeader.tsx', 'features/room-nav/RoomNavItem.tsx']) {
    assert.match(read(path), /Couldn't mark this channel as unread\./, path);
  }
  assert.doesNotMatch(
    read('pages/client/inbox/Notifications.tsx'),
    /markAsReadFromExplicitUserActionInBackground/
  );
});
