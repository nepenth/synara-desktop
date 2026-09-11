import { Box, Button, color, config, Icon, Icons, Spinner, Text } from 'folds';
import React, { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSetAtom } from 'jotai';
import { UserHero, UserHeroName } from './UserHero';
import { getMxIdLocalPart, getMxIdServer } from '../../utils/matrix';
import { getMemberAvatarMxc, getMemberDisplayName } from '../../utils/room';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { usePowerLevels } from '../../hooks/usePowerLevels';
import { useRoom } from '../../hooks/useRoom';
import { useNativeUserPresence } from '../../features/matrix-presence/nativePresence';
import { IgnoredUserAlert, MutualRoomsChip, OptionsChip, ServerChip, ShareChip } from './UserChips';
import { useCloseUserRoomProfile } from '../../state/hooks/userRoomProfile';
import { PowerChip } from './PowerChip';
import { UserInviteAlert, UserBanAlert, UserModeration, UserKickAlert } from './UserModeration';
import { useIgnoredUsers } from '../../hooks/useIgnoredUsers';
import { useMembership } from '../../hooks/useMembership';
import { Membership } from '../../../types/matrix/room';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { useMemberPowerCompare } from '../../hooks/useMemberPowerCompare';
import { CreatorChip } from './CreatorChip';
import { getDirectCreatePath, withSearchParam } from '../../pages/pathUtils';
import { DirectCreateSearchParams } from '../../pages/paths';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { isNativeMatrixSession } from '../../features/verification/nativeVerification';
import { useRoomMembers } from '../../hooks/useRoomMembers';
import { memberActionVisibility, resolveNativeRoomMembership } from './memberActions';
import { composerMentionInsertAtom } from '../../state/composerMentionInsert';
import { nativeIgnoredUsersSnapshot } from '../../features/settings/account/nativeIgnoredUsers';

type UserRoomProfileProps = {
  userId: string;
};
export function UserRoomProfile({ userId }: UserRoomProfileProps) {
  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();
  const navigate = useNavigate();
  const closeUserRoomProfile = useCloseUserRoomProfile();
  const setComposerMention = useSetAtom(composerMentionInsertAtom);
  const ignoredUsers = useIgnoredUsers();
  const [nativeIgnoredIds, setNativeIgnoredIds] = useState<string[] | null>(null);

  const room = useRoom();
  const powerLevels = usePowerLevels(room);
  const creators = useRoomCreators(room);
  const nativeSession = isNativeMatrixSession();
  const nativeMembers = useRoomMembers(mx, room.roomId, nativeSession);
  const jsMembership = useMembership(room, userId);
  const membership = nativeSession
    ? resolveNativeRoomMembership(nativeMembers ?? null, userId)
    : jsMembership;

  useEffect(() => {
    if (!nativeSession) return undefined;
    let disposed = false;
    void nativeIgnoredUsersSnapshot()
      .then((ids) => {
        if (!disposed) setNativeIgnoredIds(ids);
      })
      .catch(() => {
        if (!disposed) setNativeIgnoredIds([]);
      });
    return () => {
      disposed = true;
    };
  }, [nativeSession, userId]);
  const ignored = nativeSession
    ? (nativeIgnoredIds ?? []).includes(userId)
    : ignoredUsers.includes(userId);

  const permissions = useRoomPermissions(creators, powerLevels);
  const { hasMorePower } = useMemberPowerCompare(creators, powerLevels);

  const myUserId = mx.getSafeUserId();
  const creator = creators.has(userId);
  const nativeMembersFailed = Boolean(nativeSession && nativeMembers === undefined);
  const permissionsReady =
    !powerLevels.nativeUnavailable && (!nativeSession || Array.isArray(nativeMembers));
  const canKickUser = permissions.action('kick', myUserId) && hasMorePower(myUserId, userId);
  const canBanUser = permissions.action('ban', myUserId) && hasMorePower(myUserId, userId);
  const canInvite = permissions.action('invite', myUserId);
  const visibility = memberActionVisibility({
    isSelf: userId === myUserId,
    membership,
    permissionsReady,
    permissions: {
      canKick: canKickUser,
      canBan: canBanUser,
      canInvite,
    },
  });

  const nativeMember = useMemo(
    () =>
      nativeSession && Array.isArray(nativeMembers)
        ? nativeMembers.find((item) => item.userId === userId)
        : undefined,
    [nativeSession, nativeMembers, userId]
  );

  const member = room.getMember(userId);
  const server = getMxIdServer(userId);
  const displayName =
    nativeMember && 'displayName' in nativeMember && typeof nativeMember.displayName === 'string'
      ? nativeMember.displayName
      : getMemberDisplayName(room, userId);
  const avatarMxc =
    nativeMember && 'avatarUrl' in nativeMember && typeof nativeMember.avatarUrl === 'string'
      ? nativeMember.avatarUrl
      : getMemberAvatarMxc(room, userId);
  const avatarUrl = avatarMxc
    ? resolveMatrixThumbnailUrl(mx, avatarMxc, 96, { useAuthentication })
    : undefined;

  const presence = useNativeUserPresence(userId);

  const handleMessage = () => {
    closeUserRoomProfile();
    const directSearchParam: DirectCreateSearchParams = {
      userId,
    };
    navigate(withSearchParam(getDirectCreatePath(), directSearchParam));
  };

  const handleMention = () => {
    const name = displayName ?? getMxIdLocalPart(userId) ?? userId;
    setComposerMention({
      roomId: room.roomId,
      userId,
      name: name.startsWith('@') ? name : `@${name}`,
    });
    closeUserRoomProfile();
  };

  return (
    <Box direction="Column">
      <UserHero
        userId={userId}
        avatarUrl={avatarUrl}
        presence={presence && presence.lastActiveTs !== 0 ? presence : undefined}
      />
      <Box direction="Column" gap="500" style={{ padding: config.space.S400 }}>
        <Box direction="Column" gap="400">
          <Box gap="400" alignItems="Start">
            <UserHeroName displayName={displayName} userId={userId} />
            {visibility.sendMessage && (
              <Box shrink="No">
                <Button
                  size="300"
                  variant="Primary"
                  fill="Solid"
                  radii="300"
                  before={<Icon size="50" src={Icons.Message} filled />}
                  onClick={handleMessage}
                >
                  <Text size="B300">Message</Text>
                </Button>
              </Box>
            )}
          </Box>
          <Box alignItems="Center" gap="200" wrap="Wrap">
            {server && <ServerChip server={server} />}
            <ShareChip userId={userId} />
            {creator ? <CreatorChip /> : <PowerChip userId={userId} />}
            {userId !== myUserId && <MutualRoomsChip userId={userId} />}
            {userId !== myUserId && <OptionsChip userId={userId} />}
          </Box>
        </Box>
        {ignored && <IgnoredUserAlert />}
        {membership === Membership.Ban && (
          <UserBanAlert
            userId={userId}
            reason={member?.events.member?.getContent().reason}
            canUnban={visibility.unban}
            bannedBy={member?.events.member?.getSender()}
            ts={member?.events.member?.getTs()}
          />
        )}
        {member &&
          membership === Membership.Leave &&
          member.events.member &&
          member.events.member.getSender() !== userId && (
            <UserKickAlert
              reason={member.events.member?.getContent().reason}
              kickedBy={member.events.member?.getSender()}
              ts={member.events.member?.getTs()}
            />
          )}
        {membership === Membership.Invite && (
          <UserInviteAlert
            userId={userId}
            reason={member?.events.member?.getContent().reason}
            canKick={visibility.cancelInvite}
            invitedBy={member?.events.member?.getSender()}
            ts={member?.events.member?.getTs()}
          />
        )}
        {nativeMembersFailed && (
          <Text size="T200" style={{ color: color.Critical.Main }}>
            Could not load room membership for this user.
          </Text>
        )}
        {visibility.showPermissionsLoading && !nativeMembersFailed && (
          <Box alignItems="Center" gap="200">
            <Spinner size="100" variant="Secondary" />
            <Text size="T200">Loading room permissions…</Text>
          </Box>
        )}
        {visibility.mention && (
          <Box direction="Column" gap="200">
            <Text size="L400">Member options</Text>
            <Button
              size="300"
              variant="Secondary"
              fill="Soft"
              radii="300"
              before={<Icon size="50" src={Icons.Mention} />}
              onClick={handleMention}
              data-testid="member-option-mention"
            >
              <Text size="B300">Mention</Text>
            </Button>
          </Box>
        )}
        {/* Cancel Invite is owned by UserInviteAlert for invitees. */}
        <UserModeration
          userId={userId}
          canInvite={visibility.invite}
          canKick={visibility.removeFromRoom}
          canBan={visibility.ban}
          canAcceptKnock={visibility.acceptKnock}
          canDenyKnock={visibility.denyKnock}
        />
      </Box>
    </Box>
  );
}
