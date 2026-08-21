import React, { useMemo } from 'react';
import { Box, Icon, IconButton, Icons, Text, as } from 'folds';
import classNames from 'classnames';
import { useSetAtom } from 'jotai';
import { roomIdToTypingMembersAtom } from '../../state/typingMembers';
import { TypingIndicator } from '../../components/typing-indicator';
import { getMxIdLocalPart } from '../../utils/matrix';
import * as css from './RoomViewTyping.css';
import { useRoomTypingMember } from '../../hooks/useRoomTypingMembers';
import { isNativeMatrixSession } from '../verification/nativeVerification';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useRoomMembers, type RoomMemberListItem } from '../../hooks/useRoomMembers';

type RoomIdentity = {
  roomId: string;
};

const getProjectedMemberDisplayName = (
  member: RoomMemberListItem | undefined
): string | undefined => {
  if (!member) return undefined;
  let displayName: string | undefined;
  if ('displayName' in member && typeof member.displayName === 'string') {
    displayName = member.displayName;
  } else if ('rawDisplayName' in member) {
    displayName = member.rawDisplayName;
  }
  if (!displayName || displayName === member.userId) return undefined;
  return displayName;
};

export type RoomViewTypingProps = {
  room: RoomIdentity;
};
export const RoomViewTyping = as<'div', RoomViewTypingProps>(
  ({ className, room, ...props }, ref) => {
    const mx = useMatrixClient();
    const nativeSession = isNativeMatrixSession();
    const memberSnapshot = useRoomMembers(mx, room.roomId, nativeSession);
    const setTypingMembers = useSetAtom(roomIdToTypingMembersAtom);
    const typingMembers = useRoomTypingMember(room.roomId);
    const memberDisplayNames = useMemo(() => {
      const names = new Map<string, string>();
      for (const member of memberSnapshot ?? []) {
        const displayName = getProjectedMemberDisplayName(member);
        if (displayName) names.set(member.userId, displayName);
      }
      return names;
    }, [memberSnapshot]);

    // Own user is already excluded by the native typing projection.
    const typingNames = typingMembers
      .map(
        (receipt) =>
          memberDisplayNames.get(receipt.userId) ??
          getMxIdLocalPart(receipt.userId) ??
          receipt.userId
      )
      .reverse();

    if (typingNames.length === 0) {
      return null;
    }

    const handleDropAll = () => {
      // some homeserver does not timeout typing status
      // we have given option so user can drop their typing status
      typingMembers.forEach((receipt) =>
        setTypingMembers({
          type: 'DELETE',
          roomId: room.roomId,
          userId: receipt.userId,
        })
      );
    };

    return (
      <div className={css.RoomViewTypingSlot}>
        <Box
          className={classNames(css.RoomViewTyping, className)}
          alignItems="Center"
          gap="400"
          shrink="No"
          {...props}
          ref={ref}
        >
          <TypingIndicator />
          <Text className={css.TypingText} size="T300" truncate>
            {typingNames.length === 1 && (
              <>
                <b>{typingNames[0]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {' is typing...'}
                </Text>
              </>
            )}
            {typingNames.length === 2 && (
              <>
                <b>{typingNames[0]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {' and '}
                </Text>
                <b>{typingNames[1]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {' are typing...'}
                </Text>
              </>
            )}
            {typingNames.length === 3 && (
              <>
                <b>{typingNames[0]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {', '}
                </Text>
                <b>{typingNames[1]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {' and '}
                </Text>
                <b>{typingNames[2]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {' are typing...'}
                </Text>
              </>
            )}
            {typingNames.length > 3 && (
              <>
                <b>{typingNames[0]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {', '}
                </Text>
                <b>{typingNames[1]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {', '}
                </Text>
                <b>{typingNames[2]}</b>
                <Text as="span" size="Inherit" priority="300">
                  {' and '}
                </Text>
                <b>{typingNames.length - 3} others</b>
                <Text as="span" size="Inherit" priority="300">
                  {' are typing...'}
                </Text>
              </>
            )}
          </Text>
          <IconButton title="Drop Typing Status" size="300" radii="Pill" onClick={handleDropAll}>
            <Icon size="50" src={Icons.Cross} />
          </IconButton>
        </Box>
      </div>
    );
  }
);
