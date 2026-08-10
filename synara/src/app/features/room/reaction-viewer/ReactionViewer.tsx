import React, { useState } from 'react';
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
import * as css from './ReactionViewer.css';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { Reaction } from '../../../components/message';
import { getHexcodeForEmoji, getShortcodeFor } from '../../../plugins/emoji';
import { UserAvatar } from '../../../components/user-avatar';
import { useMediaAuthentication } from '../../../hooks/useMediaAuthentication';
import { useOpenUserRoomProfile } from '../../../state/hooks/userRoomProfile';
import { getMouseEventCords } from '../../../utils/dom';
import { redactReactionWithNativeOwner, type NativeReactionReadback } from '../nativeReactionOwner';

type LegacyRoomReference = { roomId: string };

export type ReactionViewerProps = {
  /** Native room identity and target event identity. */
  roomId?: string;
  targetEventId?: string;
  reactions?: readonly NativeReactionReadback[];
  initialKey?: string;
  canRedact?: boolean;
  requestClose: () => void;
  /**
   * Compatibility fields for the retired JS message surface. They are not
   * read: without a native projection this viewer is intentionally empty.
   */
  room?: LegacyRoomReference;
  relations?: unknown;
};

export const ReactionViewer = as<'div', ReactionViewerProps>(
  (
    {
      className,
      roomId,
      targetEventId,
      reactions = [],
      initialKey,
      canRedact,
      requestClose,
      room,
      relations: legacyRelations,
      ...props
    },
    ref
  ) => {
    void legacyRelations;
    const mx = useMatrixClient();
    const useAuthentication = useMediaAuthentication();
    const openProfile = useOpenUserRoomProfile();
    const resolvedRoomId = roomId ?? room?.roomId;

    const [selectedKey, setSelectedKey] = useState<string>(() => {
      if (initialKey) return initialKey;
      const defaultReaction = reactions[0];
      return defaultReaction?.key ?? '';
    });
    const [redactingEventId, setRedactingEventId] = useState<string>();
    const [redactError, setRedactError] = useState<string>();

    const selectedReaction = reactions.find(({ key }) => key === selectedKey);
    const selectedSenders = selectedReaction?.senders ?? [];
    const selectedShortcode = getShortcodeFor(getHexcodeForEmoji(selectedKey)) ?? selectedKey;

    const handleRedactReaction = async (key: string, reactionEventId: string | undefined) => {
      if (!canRedact || !resolvedRoomId || !targetEventId || !reactionEventId) return;
      setRedactingEventId(reactionEventId);
      setRedactError(undefined);
      try {
        await redactReactionWithNativeOwner({
          roomId: resolvedRoomId,
          eventId: targetEventId,
          reactionEventId,
          key,
        });
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
              {reactions.map((reaction) => (
                <Reaction
                  key={reaction.key}
                  mx={mx}
                  reaction={reaction.key}
                  count={reaction.count}
                  aria-selected={reaction.key === selectedKey}
                  onClick={() => setSelectedKey(reaction.key)}
                  useAuthentication={useAuthentication}
                />
              ))}
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
                {selectedSenders.map((sender) => {
                  const senderId = sender.userId;
                  const reactionEventId = sender.reactionEventId;
                  const isRedacting = redactingEventId === reactionEventId;

                  return (
                    <MenuItem
                      as="div"
                      key={`${senderId}:${reactionEventId ?? 'local'}`}
                      style={{ padding: `0 ${config.space.S200}` }}
                      radii="400"
                      onClick={(event: React.MouseEvent<HTMLElement>) => {
                        if (!resolvedRoomId) return;
                        openProfile(
                          resolvedRoomId,
                          undefined,
                          senderId,
                          getMouseEventCords(event.nativeEvent),
                          'Bottom'
                        );
                      }}
                      before={
                        <Avatar size="200">
                          <UserAvatar
                            userId={senderId}
                            alt={senderId}
                            renderFallback={() => <Icon size="50" src={Icons.User} filled />}
                          />
                        </Avatar>
                      }
                      after={
                        canRedact && reactionEventId ? (
                          <IconButton
                            size="300"
                            radii="300"
                            variant="Critical"
                            fill="None"
                            aria-label="Remove reaction"
                            disabled={isRedacting}
                            onClick={(event: React.MouseEvent<HTMLButtonElement>) => {
                              event.preventDefault();
                              event.stopPropagation();
                              void handleRedactReaction(selectedKey, reactionEventId);
                            }}
                          >
                            {isRedacting ? (
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
                          {senderId}
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
