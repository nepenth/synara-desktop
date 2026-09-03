import React, { MouseEventHandler, useCallback, useState } from 'react';
import {
  Box,
  Modal,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Text,
  Tooltip,
  TooltipProvider,
  as,
  toRem,
} from 'folds';
import classNames from 'classnames';
import FocusTrap from 'focus-trap-react';
import { Reaction } from '../../../components/message';
import * as css from './styles.css';
import { ReactionViewer } from '../reaction-viewer';
import { stopPropagation } from '../../../utils/keyboard';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { useMediaAuthentication } from '../../../hooks/useMediaAuthentication';
import { getHexcodeForEmoji, getShortcodeFor } from '../../../plugins/emoji';
import { toggleReactionWithNativeOwner, type NativeReactionReadback } from '../nativeReactionOwner';

export type ReactionsProps = {
  roomId: string;
  mEventId: string;
  canSendReaction?: boolean;
  reactions: readonly NativeReactionReadback[];
  canRedact?: boolean;
};

type NativeReactionSender = NativeReactionReadback['senders'][number];

const NativeReactionTooltipMsg = ({
  reaction,
  senders,
}: {
  reaction: string;
  senders: readonly NativeReactionSender[];
}) => {
  const shortcode = getShortcodeFor(getHexcodeForEmoji(reaction)) ?? reaction;
  const names = senders.map((sender) => sender.userId);

  return (
    <>
      {names.length === 1 && <b>{names[0]}</b>}
      {names.length === 2 && (
        <>
          <b>{names[0]}</b>
          <Text as="span" size="Inherit" priority="300">
            {' and '}
          </Text>
          <b>{names[1]}</b>
        </>
      )}
      {names.length === 3 && (
        <>
          <b>{names[0]}</b>
          <Text as="span" size="Inherit" priority="300">
            {', '}
          </Text>
          <b>{names[1]}</b>
          <Text as="span" size="Inherit" priority="300">
            {' and '}
          </Text>
          <b>{names[2]}</b>
        </>
      )}
      {names.length > 3 && (
        <>
          <b>{names[0]}</b>
          <Text as="span" size="Inherit" priority="300">
            {', '}
          </Text>
          <b>{names[1]}</b>
          <Text as="span" size="Inherit" priority="300">
            {', '}
          </Text>
          <b>{names[2]}</b>
          <Text as="span" size="Inherit" priority="300">
            {' and '}
          </Text>
          <b>{names.length - 3} others</b>
        </>
      )}
      <Text as="span" size="Inherit" priority="300">
        {' reacted with '}
      </Text>
      :<b>{shortcode}</b>:
    </>
  );
};

export const Reactions = as<'div', ReactionsProps>(
  ({ className, roomId, mEventId, canSendReaction, reactions, canRedact, ...props }, ref) => {
    const mx = useMatrixClient();
    const useAuthentication = useMediaAuthentication();
    const [viewer, setViewer] = useState<boolean | string>(false);
    const [reactionError, setReactionError] = useState<string>();

    const handleReactionToggle = useCallback(
      (key: string) => {
        setReactionError(undefined);
        const projected = reactions.find((reaction) => reaction.key === key);
        if (!projected) {
          setReactionError('Reaction ownership is unavailable.');
          return;
        }
        void toggleReactionWithNativeOwner({
          roomId,
          eventId: mEventId,
          key,
          expectedOwn: !projected.me,
        }).catch(() => {
          setReactionError('Failed to update reaction.');
        });
      },
      [mEventId, reactions, roomId]
    );

    const handleViewReaction: MouseEventHandler<HTMLButtonElement> = (evt) => {
      evt.stopPropagation();
      evt.preventDefault();
      const key = evt.currentTarget.getAttribute('data-reaction-key');
      if (!key) setViewer(true);
      else setViewer(key);
    };

    return (
      <Box
        className={classNames(css.ReactionsContainer, className)}
        gap="200"
        wrap="Wrap"
        {...props}
        ref={ref}
      >
        {reactions.map((reaction) => (
          <TooltipProvider
            key={reaction.key}
            position="Top"
            tooltip={
              <Tooltip style={{ maxWidth: toRem(200) }}>
                <Text className={css.ReactionsTooltipText} size="T300">
                  <NativeReactionTooltipMsg reaction={reaction.key} senders={reaction.senders} />
                </Text>
              </Tooltip>
            }
          >
            {(targetRef) => (
              <Reaction
                ref={targetRef}
                data-reaction-key={reaction.key}
                aria-pressed={reaction.me}
                mx={mx}
                reaction={reaction.key}
                count={reaction.count}
                onClick={canSendReaction ? () => handleReactionToggle(reaction.key) : undefined}
                onContextMenu={handleViewReaction}
                aria-disabled={!canSendReaction}
                useAuthentication={useAuthentication}
              />
            )}
          </TooltipProvider>
        ))}
        {reactionError && <Text size="T200">{reactionError}</Text>}
        {reactions.length > 0 && (
          <Overlay
            onContextMenu={(evt: any) => {
              evt.stopPropagation();
            }}
            open={!!viewer}
            backdrop={<OverlayBackdrop />}
          >
            <OverlayCenter>
              <FocusTrap
                focusTrapOptions={{
                  initialFocus: false,
                  returnFocusOnDeactivate: false,
                  onDeactivate: () => setViewer(false),
                  clickOutsideDeactivates: true,
                  escapeDeactivates: stopPropagation,
                }}
              >
                <Modal variant="Surface" size="300">
                  <ReactionViewer
                    roomId={roomId}
                    targetEventId={mEventId}
                    initialKey={typeof viewer === 'string' ? viewer : undefined}
                    reactions={reactions}
                    canRedact={canRedact}
                    requestClose={() => setViewer(false)}
                  />
                </Modal>
              </FocusTrap>
            </OverlayCenter>
          </Overlay>
        )}
      </Box>
    );
  }
);
