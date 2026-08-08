/**
 * Narrow structural projection of a MatrixRTC call membership (js-sdk
 * CallMembership). Only the fields this card renders are declared; the live
 * MatrixRTC class satisfies it at runtime.
 */
type CallMembershipReading = {
  sender: string;
  callIntent?: string;
  membershipID: string;
};

import React, { useState } from 'react';
import { Avatar, Box, Icon, Icons, Text } from 'folds';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useOpenUserRoomProfile } from '../../state/hooks/userRoomProfile';
import { SequenceCard } from '../../components/sequence-card';
import { getMemberAvatarMxc, getMemberDisplayName } from '../../utils/room';
import { useRoom } from '../../hooks/useRoom';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import { getMxIdLocalPart } from '../../utils/matrix';
import { UserAvatar } from '../../components/user-avatar';
import { getMouseEventCords } from '../../utils/dom';
import * as css from './styles.css';

type CallMemberCardProps = {
  member: CallMembershipReading;
};
export function CallMemberCard({ member }: CallMemberCardProps) {
  const mx = useMatrixClient();
  const useAuthentication = useMediaAuthentication();
  const room = useRoom();

  const openUserProfile = useOpenUserRoomProfile();

  const userId = member.sender;
  if (!userId) return null;

  const name = getMemberDisplayName(room, userId) ?? getMxIdLocalPart(userId) ?? userId;
  const avatarMxc = getMemberAvatarMxc(room, userId);
  const avatarUrl = avatarMxc
    ? resolveMatrixThumbnailUrl(mx, avatarMxc, 96, { useAuthentication })
    : undefined;

  const audioOnly = member.callIntent === 'audio';

  return (
    <SequenceCard
      as="button"
      key={member.membershipID}
      className={css.CallMemberCard}
      variant="SurfaceVariant"
      radii="500"
      onClick={(evt: any) =>
        openUserProfile(
          room.roomId,
          undefined,
          userId,
          getMouseEventCords(evt.nativeEvent),
          'Right'
        )
      }
    >
      <Box grow="Yes" gap="300" alignItems="Center">
        <Avatar size="200" radii="400">
          <UserAvatar
            userId={userId}
            src={avatarUrl}
            alt={name}
            renderFallback={() => <Icon size="50" src={Icons.User} filled />}
          />
        </Avatar>
        <Box grow="Yes">
          <Text size="L400" truncate>
            {name}
          </Text>
        </Box>
        {audioOnly && <Icon src={Icons.VideoCameraMute} size="100" />}
      </Box>
    </SequenceCard>
  );
}

export function CallMemberRenderer({
  members,
  max = 4,
}: {
  members: CallMembershipReading[];
  max?: number;
}) {
  const [viewMore, setViewMore] = useState(false);

  const truncatedMembers = viewMore ? members : members.slice(0, 4);
  const remaining = members.length - truncatedMembers.length;

  return (
    <>
      {truncatedMembers.map((member) => (
        <CallMemberCard key={member.membershipID} member={member} />
      ))}
      {members.length > max && (
        <SequenceCard
          as="button"
          className={css.CallMemberCard}
          variant="SurfaceVariant"
          radii="500"
          onClick={() => setViewMore(!viewMore)}
        >
          <Box grow="Yes" gap="300" alignItems="Center">
            {viewMore ? (
              <Text size="L400" truncate>
                Collapse
              </Text>
            ) : (
              <Text size="L400" truncate>
                {remaining === 0 ? `+${remaining} Other` : `+${remaining} Others`}
              </Text>
            )}
          </Box>
          <Icon src={viewMore ? Icons.ChevronTop : Icons.ChevronBottom} size="100" />
        </SequenceCard>
      )}
    </>
  );
}
