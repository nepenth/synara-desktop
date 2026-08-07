import React, { ReactNode } from 'react';
import { as, Avatar, Box, Icon, Icons, Text } from 'folds';
import type { MatrixClientReading } from '../../utils/room';
import type { EventedRoomReading, JsRoomMemberReading } from '../../utils/roomEvents';
import { getMemberDisplayName } from '../../utils/room';
import { getMxIdLocalPart } from '../../utils/matrix';
import { UserAvatar } from '../user-avatar';
import * as css from './style.css';
import { resolveMatrixThumbnailUrl } from '../../matrix/media';
import type { RoomMember as NativeRoomMember } from '../../features/matrix-dto/member';

type RoomMemberListItem = JsRoomMemberReading | NativeRoomMember;

const getName = (room: EventedRoomReading, member: RoomMemberListItem) =>
  (!('getMxcAvatarUrl' in member)
    ? member.displayName
    : getMemberDisplayName(room, member.userId)) ??
  getMxIdLocalPart(member.userId) ??
  member.userId;

type MemberTileProps = {
  mx: MatrixClientReading;
  room: EventedRoomReading;
  member: RoomMemberListItem;
  useAuthentication: boolean;
  after?: ReactNode;
};
export const MemberTile = as<'button', MemberTileProps>(
  ({ as: AsMemberTile = 'button', mx, room, member, useAuthentication, after, ...props }, ref) => {
    const name = getName(room, member);
    const username = getMxIdLocalPart(member.userId);

    const avatarMxcUrl = !('getMxcAvatarUrl' in member)
      ? member.avatarUrl
      : member.getMxcAvatarUrl();
    const avatarUrl = avatarMxcUrl
      ? resolveMatrixThumbnailUrl(mx, avatarMxcUrl, 100, {
          useAuthentication,
          allowDirectLinks: undefined,
          allowRedirects: false,
        })
      : undefined;

    return (
      <AsMemberTile className={css.MemberTile} {...props} ref={ref}>
        <Avatar size="300" radii="400">
          <UserAvatar
            userId={member.userId}
            src={avatarUrl ?? undefined}
            alt={name}
            renderFallback={() => <Icon size="300" src={Icons.User} filled />}
          />
        </Avatar>
        <Box grow="Yes" as="span" direction="Column">
          <Text as="span" size="T300" truncate>
            <b>{name}</b>
          </Text>
          <Box alignItems="Center" justifyContent="SpaceBetween" gap="100">
            <Text as="span" size="T200" priority="300" truncate>
              {username}
            </Text>
          </Box>
        </Box>
        {after}
      </AsMemberTile>
    );
  }
);
