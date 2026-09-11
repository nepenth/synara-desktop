import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { Membership } from '../../../../types/matrix/room';
import { memberActionVisibility, resolveNativeRoomMembership } from '../memberActions';

const moderator = {
  canKick: true,
  canBan: true,
  canInvite: true,
};

test('joined members expose Remove from room instead of Invite', () => {
  const visibility = memberActionVisibility({
    isSelf: false,
    membership: Membership.Join,
    permissionsReady: true,
    permissions: moderator,
  });

  assert.equal(visibility.removeFromRoom, true);
  assert.equal(visibility.ban, true);
  assert.equal(visibility.invite, false);
  assert.equal(visibility.cancelInvite, false);
  assert.equal(visibility.unban, false);
  assert.equal(visibility.mention, true);
  assert.equal(visibility.ignore, true);
});

test('native rooms must not treat a missing JS member as leave', () => {
  assert.equal(resolveNativeRoomMembership(null, '@bob:example.org'), null);
  assert.equal(resolveNativeRoomMembership(undefined, '@bob:example.org'), null);
  assert.equal(
    resolveNativeRoomMembership(
      [{ userId: '@bob:example.org', membership: Membership.Join }],
      '@bob:example.org'
    ),
    Membership.Join
  );
  assert.equal(
    resolveNativeRoomMembership(
      [{ userId: '@alice:example.org', membership: Membership.Join }],
      '@bob:example.org'
    ),
    Membership.Leave
  );

  const unknown = memberActionVisibility({
    isSelf: false,
    membership: null,
    permissionsReady: true,
    permissions: moderator,
  });
  assert.equal(unknown.removeFromRoom, false);
  assert.equal(unknown.invite, false);
  assert.equal(unknown.ban, false);
});

test('left members can be invited and banned, not kicked', () => {
  const visibility = memberActionVisibility({
    isSelf: false,
    membership: Membership.Leave,
    permissionsReady: true,
    permissions: moderator,
  });

  assert.equal(visibility.invite, true);
  assert.equal(visibility.removeFromRoom, false);
  assert.equal(visibility.ban, true);
  assert.equal(visibility.cancelInvite, false);
});

test('knocking members can be accepted or denied', () => {
  const knock = memberActionVisibility({
    isSelf: false,
    membership: Membership.Knock,
    permissionsReady: true,
    permissions: moderator,
  });
  assert.equal(knock.acceptKnock, true);
  assert.equal(knock.denyKnock, true);
  assert.equal(knock.removeFromRoom, false);
  assert.equal(knock.invite, false);
  assert.equal(knock.cancelInvite, false);
});

test('invited members expose cancel invite, banned members expose unban', () => {
  const invited = memberActionVisibility({
    isSelf: false,
    membership: Membership.Invite,
    permissionsReady: true,
    permissions: moderator,
  });
  assert.equal(invited.cancelInvite, true);
  assert.equal(invited.removeFromRoom, false);
  assert.equal(invited.invite, false);

  const banned = memberActionVisibility({
    isSelf: false,
    membership: Membership.Ban,
    permissionsReady: true,
    permissions: moderator,
  });
  assert.equal(banned.unban, true);
  assert.equal(banned.ban, false);
  assert.equal(banned.removeFromRoom, false);
});

test('self and unread power levels hide moderation writes', () => {
  const selfView = memberActionVisibility({
    isSelf: true,
    membership: Membership.Join,
    permissionsReady: true,
    permissions: moderator,
  });
  assert.equal(selfView.removeFromRoom, false);
  assert.equal(selfView.ban, false);
  assert.equal(selfView.mention, false);
  assert.equal(selfView.ignore, false);
  assert.equal(selfView.sendMessage, false);

  const loading = memberActionVisibility({
    isSelf: false,
    membership: Membership.Join,
    permissionsReady: false,
    permissions: { canKick: false, canBan: false, canInvite: false },
  });
  assert.equal(loading.showPermissionsLoading, true);
  assert.equal(loading.removeFromRoom, false);
  assert.equal(loading.ban, false);
});

test('regular members without kick power still get mention, share, and ignore', () => {
  const visibility = memberActionVisibility({
    isSelf: false,
    membership: Membership.Join,
    permissionsReady: true,
    permissions: { canKick: false, canBan: false, canInvite: false },
  });
  assert.equal(visibility.removeFromRoom, false);
  assert.equal(visibility.ban, false);
  assert.equal(visibility.mention, true);
  assert.equal(visibility.ignore, true);
  assert.equal(visibility.share, true);
});

test('profile ignore uses the native ignored-user owner', () => {
  const chips = readFileSync('src/app/components/user-profile/UserChips.tsx', 'utf8');
  assert.match(chips, /nativeIgnoredUsersIgnore/);
  assert.match(chips, /nativeIgnoredUsersUnignore/);
  assert.match(chips, /isNativeMatrixSession/);
});

test('desktop profile reads native membership instead of room.getMember()', () => {
  const profile = readFileSync('src/app/components/user-profile/UserRoomProfile.tsx', 'utf8');
  const moderation = readFileSync('src/app/components/user-profile/UserModeration.tsx', 'utf8');
  assert.match(profile, /useRoomMembers\(mx, room\.roomId, nativeSession\)/);
  assert.match(profile, /resolveNativeRoomMembership/);
  assert.match(profile, /memberActionVisibility/);
  assert.match(profile, /nativeIgnoredUsersSnapshot/);
  assert.match(profile, /Array\.isArray\(nativeMembers\)/);
  assert.match(profile, /visibility\.unban/);
  assert.match(profile, /visibility\.acceptKnock/);
  assert.match(profile, /\{membership === Membership\.Ban && \(/);
  assert.match(profile, /\{membership === Membership\.Invite && \(/);
  assert.match(moderation, /Remove from room/);
  assert.match(moderation, /Accept knock/);
  assert.doesNotMatch(profile, /canKick=\{canKickUser && membership === Membership\.Join\}/);
});
