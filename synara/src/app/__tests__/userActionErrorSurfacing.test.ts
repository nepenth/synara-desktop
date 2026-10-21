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

test('D86: create and address errors read the Core diagnostic, not js-sdk fields', () => {
  for (const path of [
    'features/create-room/CreateRoom.tsx',
    'features/create-space/CreateSpace.tsx',
    'features/create-chat/CreateChat.tsx',
    'features/common-settings/general/RoomAddress.tsx',
  ]) {
    const source = read(path);
    assert.doesNotMatch(source, /M_LIMIT_EXCEEDED|retry_after_ms|httpStatus/, path);
    assert.match(source, /composerErrorMessage\(/, path);
  }
  assert.match(
    read('features/common-settings/general/RoomAddress.tsx'),
    /coreDiagnosticId\(addState\.error\) === 'v-rooms-alias-taken'/
  );
});

test('D24: action failures no longer show raw error.message', () => {
  for (const path of [
    'components/AccountDataEditor.tsx',
    'components/SecretStorage.tsx',
    'components/invite-user-prompt/InviteUserPrompt.tsx',
    'components/leave-room-prompt/LeaveRoomPrompt.tsx',
    'components/leave-space-prompt/LeaveSpacePrompt.tsx',
    'components/upload-card/CompactUploadCardRenderer.tsx',
    'components/upload-card/UploadCardRenderer.tsx',
    'components/user-profile/PowerChip.tsx',
    'components/user-profile/UserModeration.tsx',
    'features/common-settings/developer-tools/SendRoomEvent.tsx',
    'features/common-settings/developer-tools/StateEventEditor.tsx',
    'features/common-settings/emojis-stickers/RoomPacks.tsx',
    'features/common-settings/general/RoomPublish.tsx',
    'features/lobby/RoomItem.tsx',
    'features/room/RoomTombstone.tsx',
    'features/room/gif/GifPicker.tsx',
    'features/room/jump-to-time/JumpToTime.tsx',
    'features/settings/devices/DeviceTile.tsx',
    'features/settings/devices/LocalBackup.tsx',
    'features/settings/devices/Verification.tsx',
    'features/settings/notifications/AgentNotifications.tsx',
    'pages/client/ClientRoot.tsx',
    'pages/client/inbox/Invites.tsx',
    'pages/client/space/Space.tsx',
  ]) {
    const source = read(path);
    assert.doesNotMatch(source, /[Ss]tate\.error\.message|\.error as any\)\?\.message/, path);
    assert.doesNotMatch(source, /instanceof Error \? \w+\.message/, path);
  }
  assert.doesNotMatch(read('features/room/RoomInput.tsx'), /err instanceof Error \? err\.message/);
});
