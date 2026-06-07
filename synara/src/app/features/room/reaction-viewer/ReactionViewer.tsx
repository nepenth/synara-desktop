import React, { useCallback, useState } from 'react';
import classNames from 'classnames';
import {
  Avatar,
  Box,
  Header,
  Icon,
  IconButton,
  Icons,
  Line,
  MenuItem,
  Scroll,
  Spinner,
  Text,
  as,
  color,
  config,
} from 'folds';
import { MatrixEvent, Room, RoomMember } from 'matrix-js-sdk';
import { Relations } from 'matrix-js-sdk/lib/models/relations';
import { getMemberDisplayName } from '../../../utils/room';
import { eventWithShortcode, getMxIdLocalPart } from '../../../utils/matrix';
import * as css from './ReactionViewer.css';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { useRelations } from '../../../hooks/useRelations';
import { Reaction } from '../../../components/message';
import { getHexcodeForEmoji, getShortcodeFor } from '../../../plugins/emoji';
import { UserAvatar } from '../../../components/user-avatar';
import { useMediaAuthentication } from '../../../hooks/useMediaAuthentication';
import { useOpenUserRoomProfile } from '../../../state/hooks/userRoomProfile';
import { useSpaceOptionally } from '../../../hooks/useSpace';
import { getMouseEventCords } from '../../../utils/dom';
import { resolveMatrixThumbnailUrl } from '../../../matrix/media';

export type ReactionViewerProps = {
  room: Room;
  initialKey?: string;
  relations: Relations;
  canRedact?: boolean;
  requestClose: () => void;
};
export const ReactionViewer = as<'div', ReactionViewerProps>(
  ({ className, room, initialKey, relations, canRedact, requestClose, ...props }, ref) => {
    const mx = useMatrixClient();
    const useAuthentication = useMediaAuthentication();
    const reactions = useRelations(
      relations,
      useCallback((rel) => [...(rel.getSortedAnnotationsByKey() ?? [])], [])
    );
    const space = useSpaceOptionally();
    const openProfile = useOpenUserRoomProfile();

    const [selectedKey, setSelectedKey] = useState<string>(() => {
      if (initialKey) return initialKey;
      const defaultReaction = reactions.find((reaction) => typeof reaction[0] === 'string');
      return defaultReaction ? defaultReaction[0] : '';
    });
    const [redactingEventId, setRedactingEventId] = useState<string>();
    const [redactError, setRedactError] = useState<string>();

    const getName = (member: RoomMember) =>
      getMemberDisplayName(room, member.userId) ?? getMxIdLocalPart(member.userId) ?? member.userId;

    const getReactionsForKey = (key: string): MatrixEvent[] => {
      const reactSet = reactions.find(([k]) => k === key)?.[1];
      if (!reactSet) return [];
      return Array.from(reactSet);
    };

    const selectedReactions = getReactionsForKey(selectedKey);
    const selectedShortcode =
      selectedReactions.find(eventWithShortcode)?.getContent().shortcode ??
      getShortcodeFor(getHexcodeForEmoji(selectedKey)) ??
      selectedKey;

    const handleRedactReaction = async (mEvent: MatrixEvent) => {
      if (!canRedact) return;
      const eventId = mEvent.getId();
      if (!eventId) return;
      setRedactingEventId(eventId);
      setRedactError(undefined);
      try {
        await mx.redactEvent(room.roomId, eventId, undefined, { reason: 'Removed reaction' });
      } catch {
        setRedactError('Failed to remove reaction.');
      } finally {
        setRedactingEventId(undefined);
      }
    };

    return (
      <Box
        className={classNames(css.ReactionViewer, className)}
        direction="Row"
        {...props}
        ref={ref}
      >
        <Box shrink="No" className={css.Sidebar}>
          <Scroll visibility="Hover" hideTrack size="300">
            <Box className={css.SidebarContent} direction="Column" gap="200">
              {reactions.map(([key, evts]) => {
                if (typeof key !== 'string') return null;
                return (
                  <Reaction
                    key={key}
                    mx={mx}
                    reaction={key}
                    count={evts.size}
                    aria-selected={key === selectedKey}
                    onClick={() => setSelectedKey(key)}
                    useAuthentication={useAuthentication}
                  />
                );
              })}
            </Box>
          </Scroll>
        </Box>
        <Line variant="Surface" direction="Vertical" size="300" />
        <Box grow="Yes" direction="Column">
          <Header className={css.Header} variant="Surface" size="600">
            <Box grow="Yes">
              <Text size="H3" truncate>{`Reacted with :${selectedShortcode}:`}</Text>
            </Box>
            <IconButton size="300" onClick={requestClose}>
              <Icon src={Icons.Cross} />
            </IconButton>
          </Header>
          {redactError && (
            <Box style={{ padding: `0 ${config.space.S400}` }}>
              <Text size="T200" style={{ color: color.Critical.Main }}>
                {redactError}
              </Text>
            </Box>
          )}

          <Box grow="Yes">
            <Scroll visibility="Hover" hideTrack size="300">
              <Box className={css.Content} direction="Column">
                {selectedReactions.map((mEvent) => {
                  const senderId = mEvent.getSender();
                  if (!senderId) return null;
                  const member = room.getMember(senderId);
                  const name = (member ? getName(member) : getMxIdLocalPart(senderId)) ?? senderId;

                  const avatarMxcUrl = member?.getMxcAvatarUrl();
                  const avatarUrl = avatarMxcUrl
                    ? resolveMatrixThumbnailUrl(mx, avatarMxcUrl, 100, { useAuthentication })
                    : undefined;
                  const eventId = mEvent.getId();

                  return (
                    <MenuItem
                      as="div"
                      key={mEvent.getId() ?? senderId}
                      style={{ padding: `0 ${config.space.S200}` }}
                      radii="400"
                      onClick={(event: React.MouseEvent<HTMLElement>) => {
                        openProfile(
                          room.roomId,
                          space?.roomId,
                          senderId,
                          getMouseEventCords(event.nativeEvent),
                          'Bottom'
                        );
                      }}
                      before={
                        <Avatar size="200">
                          <UserAvatar
                            userId={senderId}
                            src={avatarUrl ?? undefined}
                            alt={name}
                            renderFallback={() => <Icon size="50" src={Icons.User} filled />}
                          />
                        </Avatar>
                      }
                      after={
                        canRedact ? (
                          <IconButton
                            size="300"
                            radii="300"
                            variant="Critical"
                            fill="None"
                            aria-label="Remove reaction"
                            disabled={redactingEventId === eventId}
                            onClick={(event: React.MouseEvent<HTMLButtonElement>) => {
                              event.preventDefault();
                              event.stopPropagation();
                              handleRedactReaction(mEvent);
                            }}
                          >
                            {redactingEventId === eventId ? (
                              <Spinner size="100" variant="Critical" />
                            ) : (
                              <Icon size="100" src={Icons.Delete} />
                            )}
                          </IconButton>
                        ) : undefined
                      }
                    >
                      <Box grow="Yes">
                        <Text size="T400" truncate>
                          {name}
                        </Text>
                      </Box>
                    </MenuItem>
                  );
                })}
              </Box>
            </Scroll>
          </Box>
        </Box>
      </Box>
    );
  }
);
