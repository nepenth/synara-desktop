import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useFocusWithin, useHover } from 'react-aria';
import FocusTrap from 'focus-trap-react';
import { ErrorBoundary } from 'react-error-boundary';
import {
  Avatar,
  Box,
  Button,
  Icon,
  IconButton,
  Icons,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Scroll,
  Spinner,
  Text,
  Tooltip,
  TooltipProvider,
  config,
} from 'folds';
import { EmojiBoard } from '../../components/emoji-board';
import {
  clearNativeComposerReplyDraft,
  getNativeComposerReplyDraft,
  setNativeComposerReplyDraft,
} from './nativeComposerDraft';
import type { NativeComposerReplyDraft } from './nativeComposerDraftOwner';
import { createLaterItemFromIds, upsertLaterWithNativeOwner } from './nativeLaterOwner';
import { toggleReactionWithNativeOwner } from './nativeReactionOwner';
import {
  editTextWithNativeTimelineAction,
  forwardMediaWithNativeTimelineAction,
  forwardTextWithNativeTimelineAction,
  pinWithNativeTimelineAction,
  pollVoteWithNativeTimelineAction,
  redactWithNativeTimelineAction,
  reportWithNativeTimelineAction,
  unpinWithNativeTimelineAction,
} from './nativeTimelineAction';
import {
  callDeclineWithNativeTimelineOwner,
  isNativeTimelineForwardMedia,
  selectNativeTimelinePinAction,
} from './nativeTimelineActions';
import {
  filterNativeForwardTargets,
  isNativeTimelineEventPinned,
  nativeThreadFocusEventId,
  nativeTimelineMediaSrc,
  needsNativeForwardEncryptionConfirm,
  parseNativeTimelineAgentCard,
  shouldAttachFormattedBody,
  type NativeTimelineMediaHandle,
  type NativeTimelineRowCapabilities,
  type NativeTimelineViewRow,
  useNativeTimelineView,
} from './nativeTimelineView';
import { useNativeRoomListSnapshot } from '../../state/room-list/roomList';
import { Time } from '../../components/message';
import { UserAvatar } from '../../components/user-avatar';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { getMxIdLocalPart } from '../../utils/matrix';
import { nameInitials } from '../../utils/common';
import colorMXID from '../../../util/colorMXID';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { stopPropagation } from '../../utils/keyboard';
import { NativeFormattedBody } from './nativeTimelineFormattedBody';
import { shouldShowJumpToLatest } from './nativeTimelineViewportPolicy';
import * as htmlCss from './nativeTimelineHtml.css';

const HermesAgentCard = React.lazy(() =>
  import('../../components/hermes/HermesAgentCard').then((module) => ({
    default: module.HermesAgentCard,
  }))
);

type NativeTimelinePresenterProps = {
  roomId: string;
  eventId?: string;
};

type NativeTimelineViewport = {
  atBottom: boolean;
  anchor?: {
    itemId: string;
    eventId?: string;
    offsetPx: number;
  };
};

const NATIVE_TIMELINE_VIEWPORT_LIMIT = 100;
const nativeTimelineViewports = new Map<string, NativeTimelineViewport>();

const setNativeTimelineViewport = (roomId: string, viewport: NativeTimelineViewport) => {
  nativeTimelineViewports.delete(roomId);
  nativeTimelineViewports.set(roomId, viewport);
  if (nativeTimelineViewports.size > NATIVE_TIMELINE_VIEWPORT_LIMIT) {
    const oldestRoomId = nativeTimelineViewports.keys().next().value;
    if (oldestRoomId) nativeTimelineViewports.delete(oldestRoomId);
  }
};

const rowKey = (row: NativeTimelineViewRow): string => {
  if (row.kind === 'sticker') return row.event.itemId;
  return row.itemId;
};

const rowEventId = (row: NativeTimelineViewRow): string | undefined => {
  if (row.kind === 'sticker') return row.event.eventId;
  if ('eventId' in row) return row.eventId;
  return undefined;
};

const findAnchorIndex = (
  rows: NativeTimelineViewRow[],
  anchor: Pick<NonNullable<NativeTimelineViewport['anchor']>, 'itemId' | 'eventId'>
): number =>
  rows.findIndex(
    (row) => rowKey(row) === anchor.itemId || (anchor.eventId && rowEventId(row) === anchor.eventId)
  );

const rowCapabilities = (row: NativeTimelineViewRow): NativeTimelineRowCapabilities | undefined => {
  if (row.kind === 'sticker') return row.event.capabilities;
  if ('capabilities' in row) return row.capabilities;
  return undefined;
};

const rowOriginServerTs = (row: NativeTimelineViewRow): number | undefined => {
  if (row.kind === 'sticker') return row.event.originServerTs;
  if ('originServerTs' in row && typeof row.originServerTs === 'number') return row.originServerTs;
  return undefined;
};

const rowSenderId = (row: NativeTimelineViewRow | undefined): string | undefined => {
  if (!row) return undefined;
  if (row.kind === 'sticker') return row.event.senderId;
  if ('senderId' in row) return row.senderId;
  return undefined;
};

const rowSenderName = (row: NativeTimelineViewRow): string => {
  if (row.kind === 'sticker') return row.event.senderName;
  if ('senderName' in row) return row.senderName;
  return rowSenderId(row) ?? '';
};

const rowSenderAvatarUrl = (row: NativeTimelineViewRow): string | undefined => {
  if (row.kind === 'sticker') return row.event.senderAvatarUrl;
  if ('senderAvatarUrl' in row) return row.senderAvatarUrl;
  return undefined;
};

const displayNameForRow = (row: NativeTimelineViewRow): string => {
  const name = rowSenderName(row).trim();
  const senderId = rowSenderId(row) ?? '';
  if (name && name !== senderId) return name;
  return getMxIdLocalPart(senderId) ?? name ?? senderId;
};

const isGroupedWithPrevious = (
  previous: NativeTimelineViewRow | undefined,
  row: NativeTimelineViewRow
): boolean => {
  const previousId = rowSenderId(previous);
  const senderId = rowSenderId(row);
  const previousTs = previous ? rowOriginServerTs(previous) : undefined;
  const ts = rowOriginServerTs(row);
  if (!previousId || !senderId || previousId !== senderId || !previousTs || !ts) return false;
  return Math.abs(ts - previousTs) < 5 * 60 * 1000;
};

const mediaStyle = (media?: NativeTimelineMediaHandle): React.CSSProperties => {
  const maxWidth = media?.width ? Math.min(media.width, 480) : 480;
  const maxHeight = media?.height ? Math.min(media.height, 480) : 480;
  return { maxWidth, maxHeight, width: 'auto', height: 'auto' };
};

const hasMessageSurface = (kind: NativeTimelineViewRow['kind']): boolean =>
  kind === 'message' ||
  kind === 'sticker' ||
  kind === 'poll' ||
  kind === 'call' ||
  kind === 'redacted' ||
  kind === 'encrypted_unavailable';

type NativeTimelineRowProps = {
  row: NativeTimelineViewRow;
  grouped: boolean;
  groupsNext: boolean;
  roomId: string;
  pinnedEventIds?: string[];
  sourceEncrypted?: boolean;
  onActionError: (message: string) => void;
  onReplyDraftChanged: () => void;
  onFocusEvent: (eventId: string) => void;
};

const runNativeRowAction = (
  action: () => Promise<unknown>,
  onActionError: (message: string) => void,
  failureLabel: string
) => {
  void action().catch((error) => {
    onActionError(error instanceof Error ? error.message : failureLabel);
  });
};

type NativeTimelineRowActionsProps = {
  roomId: string;
  eventId?: string;
  body?: string;
  formattedBody?: string;
  rowKind?: NativeTimelineViewRow['kind'];
  messageType?: string;
  hasMedia?: boolean;
  capabilities?: NativeTimelineRowCapabilities;
  pinned?: boolean;
  sourceEncrypted?: boolean;
  onActionError: (message: string) => void;
  onReplyDraftChanged: () => void;
  /** Close the transient row menu after a completed one-shot action. */
  onRequestClose?: () => void;
};

const NativeTimelineRowActions = ({
  roomId,
  eventId,
  body,
  formattedBody,
  rowKind,
  messageType,
  hasMedia,
  capabilities,
  pinned,
  sourceEncrypted,
  onActionError,
  onReplyDraftChanged,
  onRequestClose,
}: NativeTimelineRowActionsProps) => {
  const roomList = useNativeRoomListSnapshot();
  const [editing, setEditing] = useState(false);
  const [editBody, setEditBody] = useState(body ?? '');
  const [editFormattedBody, setEditFormattedBody] = useState(formattedBody ?? '');
  const [forwarding, setForwarding] = useState(false);
  const [forwardQuery, setForwardQuery] = useState('');
  const [forwardAsQuote, setForwardAsQuote] = useState(false);
  const [forwardConfirm, setForwardConfirm] = useState<{
    roomId: string;
    name?: string;
  } | null>(null);

  useEffect(() => {
    if (!editing) {
      setEditBody(body ?? '');
      setEditFormattedBody(formattedBody ?? '');
    }
  }, [body, editing, formattedBody]);

  const forwardTargets = useMemo(
    () =>
      filterNativeForwardTargets(
        roomList.rooms.map((room) => ({
          roomId: room.roomId,
          name: room.name,
          isEncrypted: room.isEncrypted,
          isSpace: room.isSpace,
        })),
        roomId,
        forwardQuery
      ),
    [forwardQuery, roomId, roomList.rooms]
  );

  if (!eventId || !capabilities) return null;
  const buttons: React.ReactNode[] = [];
  const closeAfterOneShotAction = () => onRequestClose?.();
  if (capabilities.reply) {
    buttons.push(
      <MenuItem
        key="reply"
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            async () => {
              const result = await setNativeComposerReplyDraft({ roomId, eventId });
              if (result === 'unavailable') {
                throw new Error('Native reply draft is unavailable.');
              }
              onReplyDraftChanged();
            },
            onActionError,
            'Native reply draft failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        Reply
      </MenuItem>
    );
    buttons.push(
      <MenuItem
        key="reply-thread"
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            async () => {
              const result = await setNativeComposerReplyDraft({
                roomId,
                eventId,
                startThread: true,
              });
              if (result === 'unavailable') {
                throw new Error('Native thread reply draft is unavailable.');
              }
              onReplyDraftChanged();
            },
            onActionError,
            'Native thread reply draft failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        Reply in thread
      </MenuItem>
    );
  }
  if (capabilities.edit) {
    buttons.push(
      <MenuItem
        key="edit"
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          setForwarding(false);
          setEditing((open) => !open);
          setEditBody(body ?? '');
          setEditFormattedBody(formattedBody ?? '');
        }}
      >
        {editing ? 'Cancel edit' : 'Edit'}
      </MenuItem>
    );
  }
  if (capabilities.forward) {
    buttons.push(
      <MenuItem
        key="forward"
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          setEditing(false);
          setForwarding((open) => !open);
          setForwardConfirm(null);
        }}
      >
        {forwarding ? 'Cancel forward' : 'Forward'}
      </MenuItem>
    );
  }
  if (capabilities.redact) {
    buttons.push(
      <MenuItem
        key="redact"
        variant="Critical"
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            () => redactWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native redact failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        Redact
      </MenuItem>
    );
  }
  if (capabilities.report) {
    buttons.push(
      <MenuItem
        key="report"
        variant="Critical"
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            () => reportWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native report failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        Report
      </MenuItem>
    );
  }
  if (capabilities.pin) {
    const pinAction = selectNativeTimelinePinAction(Boolean(pinned));
    buttons.push(
      <MenuItem
        key={pinAction}
        size="300"
        fill="Soft"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            () =>
              pinAction === 'unpin'
                ? unpinWithNativeTimelineAction({ roomId, eventId })
                : pinWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            pinAction === 'unpin' ? 'Native unpin failed.' : 'Native pin failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        {pinAction === 'unpin' ? 'Unpin' : 'Pin'}
      </MenuItem>
    );
  }
  // Later is a room-event affordance for any remote timeline item with an id.
  buttons.push(
    <MenuItem
      key="later"
      size="300"
      fill="Soft"
      radii="300"
      onClick={() => {
        runNativeRowAction(
          () => upsertLaterWithNativeOwner(createLaterItemFromIds(roomId, eventId, 'saved')),
          onActionError,
          'Native later save failed.'
        );
        closeAfterOneShotAction();
      }}
    >
      Save for later
    </MenuItem>
  );
  if (buttons.length === 0) return null;

  const submitEdit = () => {
    const nextBody = editBody.trim();
    if (!nextBody) {
      onActionError('Edited body cannot be empty.');
      return;
    }
    const nextFormatted = shouldAttachFormattedBody(nextBody, editFormattedBody)
      ? editFormattedBody.trim()
      : undefined;
    runNativeRowAction(
      async () => {
        await editTextWithNativeTimelineAction({
          roomId,
          eventId,
          body: nextBody,
          formattedBody: nextFormatted,
        });
        setEditing(false);
        onRequestClose?.();
      },
      onActionError,
      'Native edit failed.'
    );
  };

  const sendForward = (targetRoomId: string) => {
    const useMedia =
      !forwardAsQuote &&
      isNativeTimelineForwardMedia({
        kind: rowKind,
        messageType,
        hasMedia,
      });
    runNativeRowAction(
      async () => {
        if (useMedia) {
          await forwardMediaWithNativeTimelineAction({
            sourceRoomId: roomId,
            eventId,
            targetRoomId,
          });
        } else {
          await forwardTextWithNativeTimelineAction({
            sourceRoomId: roomId,
            eventId,
            targetRoomId,
            asQuote: forwardAsQuote,
          });
        }
        setForwarding(false);
        setForwardQuery('');
        setForwardAsQuote(false);
        setForwardConfirm(null);
        onRequestClose?.();
      },
      onActionError,
      'Native forward failed.'
    );
  };

  const requestForward = (target: { roomId: string; name?: string; isEncrypted?: boolean }) => {
    if (needsNativeForwardEncryptionConfirm(sourceEncrypted, target.isEncrypted)) {
      setForwardConfirm({ roomId: target.roomId, name: target.name });
      return;
    }
    sendForward(target.roomId);
  };

  return (
    <Box direction="Column" gap="100" style={{ minWidth: 240, padding: config.space.S100 }}>
      <Box direction="Column" gap="100">
        {buttons}
      </Box>
      {editing && (
        <Box direction="Column" gap="100">
          <textarea
            value={editBody}
            onChange={(event) => setEditBody(event.target.value)}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
            aria-label="Edit message body"
          />
          <textarea
            value={editFormattedBody}
            onChange={(event) => setEditFormattedBody(event.target.value)}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
            aria-label="Edit message HTML body"
            placeholder="Optional Matrix HTML (org.matrix.custom.html)"
          />
          <Box gap="100">
            <Button size="300" onClick={submitEdit}>
              Save edit
            </Button>
          </Box>
        </Box>
      )}
      {forwarding && (
        <Box direction="Column" gap="100">
          {forwardConfirm ? (
            <Box direction="Column" gap="100">
              <Text size="T200">
                Forward from an encrypted room into {forwardConfirm.name || forwardConfirm.roomId}{' '}
                (not encrypted)? Media keys will not transfer.
              </Text>
              <Box gap="100">
                <Button size="300" onClick={() => sendForward(forwardConfirm.roomId)}>
                  Forward anyway
                </Button>
                <Button size="300" fill="Soft" onClick={() => setForwardConfirm(null)}>
                  Cancel
                </Button>
              </Box>
            </Box>
          ) : (
            <>
              <input
                value={forwardQuery}
                onChange={(event) => setForwardQuery(event.target.value)}
                placeholder="Filter rooms by name or id"
                style={{ width: '100%' }}
                aria-label="Forward target room filter"
              />
              <label style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                <input
                  type="checkbox"
                  checked={forwardAsQuote}
                  onChange={(event) => setForwardAsQuote(event.target.checked)}
                />
                <Text size="T200">Forward as quote</Text>
              </label>
              <Box direction="Column" gap="100" style={{ maxHeight: 180, overflow: 'auto' }}>
                {forwardTargets.length === 0 ? (
                  <Text size="T200">No matching rooms.</Text>
                ) : (
                  forwardTargets.slice(0, 40).map((target) => (
                    <Button
                      key={target.roomId}
                      size="300"
                      fill="Soft"
                      onClick={() => requestForward(target)}
                    >
                      {target.name || target.roomId}
                      {target.isEncrypted ? ' · encrypted' : ''}
                    </Button>
                  ))
                )}
              </Box>
            </>
          )}
        </Box>
      )}
    </Box>
  );
};

type NativeTimelineRowActionSurfaceProps = {
  children: React.ReactNode;
  actionProps: Omit<NativeTimelineRowActionsProps, 'onRequestClose'>;
  onReaction: (key: string) => void;
};

/**
 * The native presenter owns the action UI as well as the data/actions behind it.
 * Keep the legacy Message component out of this path: it requires the retired
 * Matrix event graph, while these controls consume only native DTO capabilities
 * and native command owners.
 */
const NativeTimelineRowActionSurface = ({
  children,
  actionProps,
  onReaction,
}: NativeTimelineRowActionSurfaceProps) => {
  const { eventId, capabilities } = actionProps;
  const [hovered, setHovered] = useState(false);
  const [focusWithin, setFocusWithin] = useState(false);
  const [emojiBoardAnchor, setEmojiBoardAnchor] = useState<RectCords>();
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const { hoverProps } = useHover({ onHoverChange: setHovered });
  const { focusWithinProps } = useFocusWithin({ onFocusWithinChange: setFocusWithin });
  const hasActionMenu = Boolean(eventId && capabilities);
  const menuOpen = Boolean(menuAnchor);
  const emojiBoardOpen = Boolean(emojiBoardAnchor);
  const showActionRail = hasActionMenu;
  const actionsActive = hovered || focusWithin || menuOpen || emojiBoardOpen;

  const closeMenu = () => setMenuAnchor(undefined);
  const openMenu = (event: React.MouseEvent<HTMLButtonElement>) => {
    setEmojiBoardAnchor(undefined);
    setMenuAnchor(event.currentTarget.getBoundingClientRect());
  };
  const openEmojiBoard = (event: React.MouseEvent<HTMLButtonElement>) => {
    setMenuAnchor(undefined);
    const target = event.currentTarget.parentElement?.parentElement ?? event.currentTarget;
    setEmojiBoardAnchor(target.getBoundingClientRect());
  };

  return (
    <div
      {...(hasActionMenu ? hoverProps : {})}
      {...(hasActionMenu ? focusWithinProps : {})}
      tabIndex={hasActionMenu ? 0 : undefined}
      role={hasActionMenu ? 'group' : undefined}
      aria-label={hasActionMenu ? 'Message actions' : undefined}
      className={htmlCss.MessageActionSurface}
    >
      {showActionRail && (
        <div data-native-timeline-action-rail="true" className={htmlCss.MessageActionRail}>
          <Menu variant="SurfaceVariant" style={{ padding: config.space.S100 }}>
            <Box gap="100">
              {capabilities?.react && (
                <PopOut
                  anchor={emojiBoardAnchor}
                  position="Bottom"
                  align="End"
                  content={
                    <EmojiBoard
                      imagePackRooms={[actionProps.roomId]}
                      returnFocusOnDeactivate={false}
                      addToRecentEmoji={false}
                      onEmojiSelect={(unicode) => {
                        onReaction(unicode);
                        setEmojiBoardAnchor(undefined);
                      }}
                      onCustomEmojiSelect={(mxc) => {
                        onReaction(mxc);
                        setEmojiBoardAnchor(undefined);
                      }}
                      requestClose={() => setEmojiBoardAnchor(undefined)}
                    />
                  }
                >
                  <IconButton
                    variant={actionsActive ? 'Surface' : 'SurfaceVariant'}
                    size="300"
                    radii="300"
                    title="Add reaction"
                    aria-label="Add reaction"
                    aria-pressed={emojiBoardOpen}
                    onClick={openEmojiBoard}
                  >
                    <Icon src={Icons.SmilePlus} size="100" />
                  </IconButton>
                </PopOut>
              )}
              <PopOut
                anchor={menuAnchor}
                position="Bottom"
                align="End"
                content={
                  <FocusTrap
                    focusTrapOptions={{
                      initialFocus: false,
                      onDeactivate: closeMenu,
                      clickOutsideDeactivates: true,
                      isKeyForward: (event: KeyboardEvent) => event.key === 'ArrowDown',
                      isKeyBackward: (event: KeyboardEvent) => event.key === 'ArrowUp',
                      escapeDeactivates: stopPropagation,
                    }}
                  >
                    <Menu data-native-timeline-action-menu="true">
                      <NativeTimelineRowActions {...actionProps} onRequestClose={closeMenu} />
                    </Menu>
                  </FocusTrap>
                }
              >
                <IconButton
                  variant={actionsActive ? 'Surface' : 'SurfaceVariant'}
                  size="300"
                  radii="300"
                  title="More message actions"
                  aria-label="More message actions"
                  aria-haspopup="menu"
                  aria-expanded={menuOpen}
                  onClick={openMenu}
                >
                  <Icon src={Icons.VerticalDots} size="100" />
                </IconButton>
              </PopOut>
            </Box>
          </Menu>
        </div>
      )}
      {children}
    </div>
  );
};

const NativeTimelineMedia = ({
  media,
  messageType,
  body,
  sticker,
}: {
  media?: NativeTimelineMediaHandle;
  messageType?: string;
  body?: string;
  sticker?: boolean;
}) => {
  const mediaSrc = media ? nativeTimelineMediaSrc(media) : undefined;
  if (!mediaSrc) {
    return sticker ? <Text size="T300">Sticker media is unavailable.</Text> : null;
  }
  if (sticker) {
    return <img src={mediaSrc} alt="Sticker" style={{ maxWidth: 256, maxHeight: 256 }} />;
  }
  if (messageType === 'image') {
    return <img src={mediaSrc} alt={body || 'Image'} style={mediaStyle(media)} />;
  }
  if (messageType === 'audio') {
    return (
      // Matrix media metadata does not provide a captions track.
      // eslint-disable-next-line jsx-a11y/media-has-caption
      <audio
        src={mediaSrc}
        controls
        {...(media?.durationMs ? { 'data-duration-ms': String(media.durationMs) } : {})}
      />
    );
  }
  if (messageType === 'video') {
    return (
      // Matrix media metadata does not provide a captions track.
      // eslint-disable-next-line jsx-a11y/media-has-caption
      <video
        src={mediaSrc}
        controls
        style={mediaStyle(media)}
        {...(media?.durationMs ? { 'data-duration-ms': String(media.durationMs) } : {})}
      />
    );
  }
  if (messageType === 'file') {
    return (
      <Box direction="Column" gap="100">
        <a href={mediaSrc} download>
          {body || 'Download file'}
        </a>
        {media?.mimeType ? (
          <Text size="T200" style={{ opacity: 0.7 }}>
            {media.mimeType}
          </Text>
        ) : null}
      </Box>
    );
  }
  return null;
};

const NativeTimelineSenderAvatar = ({ row }: { row: NativeTimelineViewRow }) => {
  const senderId = rowSenderId(row) ?? '';
  const displayName = displayNameForRow(row);
  return (
    <Avatar size="300" radii="400">
      <UserAvatar
        userId={senderId}
        src={rowSenderAvatarUrl(row)}
        alt={displayName}
        renderFallback={() => (
          <Text as="span" size="T200" style={{ textTransform: 'uppercase' }}>
            {nameInitials(displayName)}
          </Text>
        )}
      />
    </Avatar>
  );
};

const NativeTimelineRow = ({
  row,
  grouped,
  groupsNext,
  roomId,
  pinnedEventIds,
  sourceEncrypted,
  onActionError,
  onReplyDraftChanged,
  onFocusEvent,
}: NativeTimelineRowProps) => {
  const [hour24Clock] = useSetting(settingsAtom, 'hour24Clock');
  const [dateFormatString] = useSetting(settingsAtom, 'dateFormatString');
  const surface = hasMessageSurface(row.kind);
  const rowClassName = htmlCss.MessageRow({
    surface,
    grouped: surface && grouped,
    groupsNext: surface && groupsNext,
  });
  const capabilities = rowCapabilities(row);
  const eventId = rowEventId(row);
  const originServerTs = rowOriginServerTs(row);
  const pinned = isNativeTimelineEventPinned(pinnedEventIds, eventId);
  const runReaction = (key: string) => {
    if (!eventId || !capabilities?.react) return;
    void toggleReactionWithNativeOwner({ roomId, eventId, key }).catch((error) => {
      onActionError(error instanceof Error ? error.message : 'Native reaction failed.');
    });
  };
  const runDecline = () => {
    if (!eventId || !capabilities?.declineCall) return;
    void callDeclineWithNativeTimelineOwner(
      { roomId, eventId },
      isSynaraDesktop(),
      invokeDesktopWithAvailability
    )
      .then((result) => {
        if (result === 'unavailable') onActionError('Native call decline is unavailable.');
      })
      .catch((error) => {
        onActionError(error instanceof Error ? error.message : 'Native call decline failed.');
      });
  };
  const runPollVote = (answerId: string) => {
    if (!eventId || !capabilities?.vote) return;
    runNativeRowAction(
      () =>
        pollVoteWithNativeTimelineAction({
          roomId,
          eventId,
          answerIds: [answerId],
        }),
      onActionError,
      'Native poll vote failed.'
    );
  };

  switch (row.kind) {
    case 'message': {
      const isEmote = row.messageType === 'emote';
      const threadFocus = nativeThreadFocusEventId(row.thread);
      const agentPayload = parseNativeTimelineAgentCard(row.agentCardJson);
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            roomId,
            eventId,
            body: row.body,
            formattedBody: row.formattedBody,
            rowKind: row.kind,
            messageType: row.messageType,
            hasMedia: Boolean(row.media),
            capabilities,
            pinned,
            sourceEncrypted,
            onActionError,
            onReplyDraftChanged,
          }}
          onReaction={runReaction}
        >
          <Box direction="Column" gap="100" className={rowClassName}>
            <Box gap="300" alignItems="Start">
              <Box direction="Column" alignItems="Center" style={{ width: 36, flexShrink: 0 }}>
                {grouped ? (
                  originServerTs ? (
                    <Time
                      compact
                      ts={originServerTs}
                      hour24Clock={hour24Clock}
                      dateFormatString={dateFormatString}
                    />
                  ) : null
                ) : (
                  <NativeTimelineSenderAvatar row={row} />
                )}
              </Box>
              <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0 }}>
                {grouped ? null : (
                  <Box gap="200" alignItems="Baseline">
                    <Text size="T300" style={{ color: colorMXID(row.senderId), fontWeight: 600 }}>
                      {displayNameForRow(row)}
                    </Text>
                    {originServerTs ? (
                      <Time
                        ts={originServerTs}
                        hour24Clock={hour24Clock}
                        dateFormatString={dateFormatString}
                      />
                    ) : null}
                    {pinned ? (
                      <Text size="T200" style={{ opacity: 0.7 }}>
                        Pinned
                      </Text>
                    ) : null}
                  </Box>
                )}
                {row.reply && (
                  <Box
                    as="button"
                    direction="Column"
                    gap="100"
                    onClick={() => onFocusEvent(row.reply!.eventId)}
                    style={{
                      opacity: 0.8,
                      borderLeft: '2px solid currentColor',
                      paddingLeft: config.space.S200,
                      background: 'transparent',
                      borderTop: 'none',
                      borderRight: 'none',
                      borderBottom: 'none',
                      textAlign: 'left',
                      cursor: 'pointer',
                      color: 'inherit',
                    }}
                    aria-label={`Jump to replied message ${row.reply.eventId}`}
                  >
                    <Text size="T200">{row.reply.senderName}</Text>
                    <Text size="T200" style={{ whiteSpace: 'pre-wrap' }}>
                      {row.reply.body}
                    </Text>
                  </Box>
                )}
                {agentPayload ? (
                  <ErrorBoundary fallback={<Text size="T300">Agent output unavailable</Text>}>
                    <React.Suspense
                      fallback={<Spinner size="200" aria-label="Loading agent output" />}
                    >
                      <HermesAgentCard payload={agentPayload} />
                    </React.Suspense>
                  </ErrorBoundary>
                ) : (
                  <div className={htmlCss.MessageBody}>
                    {row.formattedBody ? (
                      <NativeFormattedBody
                        html={row.formattedBody}
                        style={{
                          fontStyle: isEmote ? 'italic' : undefined,
                        }}
                      />
                    ) : (
                      <Text
                        size="T300"
                        style={{
                          whiteSpace: 'pre-wrap',
                          fontWeight: 400,
                          lineHeight: 1.55,
                          fontStyle: isEmote ? 'italic' : undefined,
                        }}
                      >
                        {isEmote ? `* ${row.body}` : row.body}
                      </Text>
                    )}
                    {row.edited ? (
                      <Text size="T200" style={{ opacity: 0.7 }}>
                        Edited
                      </Text>
                    ) : null}
                  </div>
                )}
                {row.thread && threadFocus ? (
                  <Button size="300" fill="Soft" onClick={() => onFocusEvent(threadFocus)}>
                    Thread · {row.thread.replyCount}{' '}
                    {row.thread.replyCount === 1 ? 'reply' : 'replies'}
                    {row.thread.latestEventId ? ' · open latest' : ' · open root'}
                  </Button>
                ) : null}
                <NativeTimelineMedia
                  media={row.media}
                  messageType={row.messageType}
                  body={row.body}
                />
                {row.reactions?.length ? (
                  <Box gap="100" wrap="Wrap">
                    {row.reactions.map((reaction) => (
                      <Button
                        key={reaction.key}
                        size="300"
                        variant={reaction.own ? 'Primary' : 'Secondary'}
                        fill="Soft"
                        disabled={!capabilities?.react}
                        onClick={() => runReaction(reaction.key)}
                      >
                        {reaction.key} {reaction.count}
                      </Button>
                    ))}
                  </Box>
                ) : null}
              </Box>
            </Box>
          </Box>
        </NativeTimelineRowActionSurface>
      );
    }
    case 'membership':
    case 'state':
      return (
        <Box className={rowClassName}>
          <Text size="T300">{row.summary}</Text>
        </Box>
      );
    case 'poll':
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            roomId,
            eventId,
            rowKind: row.kind,
            capabilities,
            pinned,
            sourceEncrypted,
            onActionError,
            onReplyDraftChanged,
          }}
          onReaction={runReaction}
        >
          <Box direction="Column" gap="100" className={rowClassName}>
            {originServerTs ? (
              <Time
                ts={originServerTs}
                hour24Clock={hour24Clock}
                dateFormatString={dateFormatString}
              />
            ) : null}
            <Text size="L400">{row.question}</Text>
            <Text size="T300">{row.closed ? 'Poll closed' : 'Poll open'}</Text>
            {(row.answers ?? []).map((answer) => (
              <Button
                key={answer.id}
                size="300"
                variant={answer.own ? 'Primary' : 'Secondary'}
                fill="Soft"
                disabled={row.closed || !capabilities?.vote}
                onClick={() => runPollVote(answer.id)}
              >
                {answer.text} ({answer.voteCount})
              </Button>
            ))}
          </Box>
        </NativeTimelineRowActionSurface>
      );
    case 'call':
      return (
        <Box direction="Column" gap="100" className={rowClassName}>
          <Text size="T300">{row.callKind}</Text>
          {capabilities?.declineCall && (
            <Button size="300" fill="Soft" onClick={runDecline}>
              Decline call
            </Button>
          )}
        </Box>
      );
    case 'date_separator':
      return row.timestampMs && Number.isFinite(row.timestampMs) ? (
        <Box className={rowClassName}>
          <Text size="T300">{new Date(row.timestampMs).toLocaleDateString()}</Text>
        </Box>
      ) : null;
    case 'read_marker':
      return (
        <Box className={rowClassName}>
          <Text size="T300">Read up to here</Text>
        </Box>
      );
    case 'unread_marker':
      return (
        <Box className={rowClassName}>
          <Text size="T300">New messages</Text>
        </Box>
      );
    case 'timeline_start':
      return (
        <Box className={rowClassName}>
          <Text size="T300">Beginning of timeline</Text>
        </Box>
      );
    case 'redacted':
      return (
        <Box className={rowClassName}>
          <Text size="T300">{row.summary ?? 'Message removed'}</Text>
        </Box>
      );
    case 'encrypted_unavailable':
      return (
        <Box className={rowClassName}>
          <Text size="T300">This encrypted message is not available on this device.</Text>
        </Box>
      );
    case 'other':
      return row.summary ? (
        <Box className={rowClassName}>
          <Text size="T300">{row.summary}</Text>
        </Box>
      ) : null;
    case 'sticker': {
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            roomId,
            eventId,
            rowKind: row.kind,
            hasMedia: Boolean(row.media),
            capabilities,
            pinned,
            sourceEncrypted,
            onActionError,
            onReplyDraftChanged,
          }}
          onReaction={runReaction}
        >
          <Box direction="Column" gap="100" className={rowClassName}>
            <Box gap="200" alignItems="Baseline">
              <Text size="L400">{row.event.senderName}</Text>
              {originServerTs ? (
                <Time
                  ts={originServerTs}
                  hour24Clock={hour24Clock}
                  dateFormatString={dateFormatString}
                />
              ) : null}
              {pinned ? (
                <Text size="T200" style={{ opacity: 0.7 }}>
                  Pinned
                </Text>
              ) : null}
            </Box>
            <NativeTimelineMedia media={row.media} sticker />
          </Box>
        </NativeTimelineRowActionSurface>
      );
    }
    case 'pagination':
      return row.state === 'loading' ? (
        <Box className={rowClassName}>
          <Box justifyContent="Center">
            <Spinner size="200" aria-label="Loading messages" />
          </Box>
        </Box>
      ) : null;
    default:
      return null;
  }
};

/**
 * SDK-neutral, virtualized presentation of the native timeline DTO.
 * Active owner mounted by RoomView after V-TIMELINE.C1; JS RoomTimeline deleted
 * in V-TIMELINE.C2 (dual_backend false).
 */
export function NativeTimelinePresenter({ roomId, eventId }: NativeTimelinePresenterProps) {
  const [focusEventId, setFocusEventId] = useState(eventId);
  useEffect(() => {
    setFocusEventId(eventId);
  }, [eventId, roomId]);

  const openingViewport = useMemo(
    () => (focusEventId ? undefined : nativeTimelineViewports.get(roomId)),
    [focusEventId, roomId]
  );
  const input = useMemo(
    () => ({
      roomId,
      position: focusEventId
        ? ({ kind: 'focused', eventId: focusEventId } as const)
        : ({
            kind: 'normal',
            restoredAnchorEventId: openingViewport?.atBottom
              ? undefined
              : openingViewport?.anchor?.eventId,
          } as const),
    }),
    [focusEventId, openingViewport, roomId]
  );
  const controller = useNativeTimelineView(input);
  const timelineState = controller.state;
  const readyState = timelineState.status === 'ready' ? timelineState : undefined;
  const [actionError, setActionError] = useState<string>();
  const [atLiveBottom, setAtLiveBottom] = useState(false);
  const [replyDraft, setReplyDraft] = useState<NativeComposerReplyDraft | undefined>();
  const roomList = useNativeRoomListSnapshot();
  const sourceEncrypted = roomList.rooms.find((room) => room.roomId === roomId)?.isEncrypted;
  const scrollRef = useRef<HTMLDivElement>(null);
  const paginationInFlightRef = useRef<'backwards' | 'forwards' | undefined>(undefined);
  const pendingBackwardGrowRef = useRef(false);
  const userInitiatedScrollRef = useRef(false);
  const followingLiveRef = useRef(false);
  const programmaticScrollUntilRef = useRef(0);
  const lastTotalSizeRef = useRef(0);
  const rows = useMemo(() => readyState?.snapshot.rows ?? [], [readyState?.snapshot.rows]);
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: rows.length,
    getScrollElement: useCallback(() => scrollRef.current, []),
    getItemKey: useCallback(
      (index: number) => {
        const row = rows[index];
        return row ? rowKey(row) : index;
      },
      [rows]
    ),
    estimateSize: useCallback(() => 64, []),
    overscan: 8,
  });

  const refreshReplyDraft = useCallback(() => {
    void getNativeComposerReplyDraft({ roomId }).then((result) => {
      if (result === 'unavailable') {
        setReplyDraft(undefined);
        return;
      }
      setReplyDraft(result.status === 'set' ? result.draft : undefined);
    });
  }, [roomId]);

  useEffect(() => {
    refreshReplyDraft();
  }, [refreshReplyDraft]);

  const initialPlacementRef = useRef<string | undefined>(undefined);
  const saveViewport = useCallback(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || rows.length === 0) return;
    const atBottom = scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 8;
    if (atBottom) {
      setNativeTimelineViewport(roomId, { atBottom: true });
      return;
    }
    const visible = virtualizer.getVirtualItems().find((item) => item.end > scrollEl.scrollTop);
    const row = visible ? rows[visible.index] : undefined;
    if (!visible || !row) return;
    setNativeTimelineViewport(roomId, {
      atBottom: false,
      anchor: {
        itemId: rowKey(row),
        eventId: rowEventId(row),
        offsetPx: scrollEl.scrollTop - visible.start,
      },
    });
  }, [roomId, rows, virtualizer]);

  useEffect(() => {
    userInitiatedScrollRef.current = false;
    followingLiveRef.current = false;
    programmaticScrollUntilRef.current = 0;
    initialPlacementRef.current = undefined;
    lastTotalSizeRef.current = 0;
    pendingBackwardGrowRef.current = false;
    setAtLiveBottom(false);
  }, [roomId]);

  useEffect(() => {
    if (!readyState) return undefined;
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;
    const paginateAtEdge = () => {
      if (paginationInFlightRef.current) return;
      if (!userInitiatedScrollRef.current) return;
      const { snapshot } = readyState;
      const distanceFromBottom = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
      const direction =
        scrollEl.scrollTop <= 96 &&
        snapshot.capabilities.paginateBackward &&
        snapshot.pagination.backward === 'available'
          ? 'backwards'
          : distanceFromBottom <= 96 &&
            snapshot.capabilities.paginateForward &&
            snapshot.pagination.forward === 'available'
          ? 'forwards'
          : undefined;
      if (!direction) return;

      paginationInFlightRef.current = direction;
      if (direction === 'backwards') pendingBackwardGrowRef.current = true;
      setActionError(undefined);
      void controller
        .paginate(direction)
        .catch((error) => {
          setActionError(
            error instanceof Error ? error.message : 'Native timeline pagination failed.'
          );
        })
        .finally(() => {
          paginationInFlightRef.current = undefined;
        });
    };
    const onScroll = () => {
      saveViewport();
      const distanceFromBottom = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
      const atBottom = distanceFromBottom <= 8;
      followingLiveRef.current = readyState.selectedPosition.kind === 'live_bottom' && atBottom;
      setAtLiveBottom((previous) => (previous === atBottom ? previous : atBottom));
      if (performance.now() < programmaticScrollUntilRef.current) return;
      userInitiatedScrollRef.current = true;
      paginateAtEdge();
    };
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    saveViewport();
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      saveViewport();
    };
  }, [controller, readyState, saveViewport]);

  useLayoutEffect(() => {
    if (!readyState || rows.length === 0) return undefined;
    const { snapshot, selectedPosition } = readyState;
    const selectedAnchor =
      selectedPosition.kind === 'focused'
        ? { eventId: selectedPosition.target_event_id, itemId: selectedPosition.target_event_id }
        : selectedPosition.kind === 'unread'
        ? { eventId: selectedPosition.anchor_event_id, itemId: selectedPosition.anchor_event_id }
        : selectedPosition.kind === 'restored' && selectedPosition.anchor_event_id
        ? {
            eventId: selectedPosition.anchor_event_id,
            itemId: selectedPosition.anchor_event_id,
          }
        : undefined;
    const placementKey = `${roomId}:${snapshot.sessionGeneration}:${selectedPosition.kind}:${
      selectedAnchor?.eventId ?? ''
    }`;
    const initialPlacement = initialPlacementRef.current !== placementKey;
    const savedViewport = nativeTimelineViewports.get(roomId);
    const totalSize = virtualizer.getTotalSize();
    const previousTotalSize = lastTotalSizeRef.current;
    lastTotalSizeRef.current = totalSize;

    if (initialPlacement) {
      const anchor = selectedAnchor ?? savedViewport?.anchor;
      const anchorIndex = anchor ? findAnchorIndex(rows, anchor) : -1;
      if (selectedPosition.kind === 'live_bottom' || savedViewport?.atBottom) {
        followingLiveRef.current = true;
        setAtLiveBottom(true);
        programmaticScrollUntilRef.current = performance.now() + 250;
        virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      } else if (anchorIndex >= 0) {
        followingLiveRef.current = false;
        setAtLiveBottom(false);
        programmaticScrollUntilRef.current = performance.now() + 250;
        virtualizer.scrollToIndex(anchorIndex, { align: 'start', behavior: 'auto' });
        const offsetPx = selectedAnchor ? 0 : savedViewport?.anchor?.offsetPx ?? 0;
        const animationFrame = window.requestAnimationFrame(() => {
          if (scrollRef.current && offsetPx !== 0) scrollRef.current.scrollTop += offsetPx;
        });
        initialPlacementRef.current = placementKey;
        return () => window.cancelAnimationFrame(animationFrame);
      }
      initialPlacementRef.current = placementKey;
      return undefined;
    }

    if (followingLiveRef.current) {
      programmaticScrollUntilRef.current = performance.now() + 250;
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      return undefined;
    }

    if (pendingBackwardGrowRef.current && previousTotalSize > 0 && totalSize > previousTotalSize) {
      pendingBackwardGrowRef.current = false;
      const scrollEl = scrollRef.current;
      if (scrollEl) scrollEl.scrollTop += totalSize - previousTotalSize;
    }
    return undefined;
  }, [readyState, roomId, rows, virtualizer]);

  const onFocusEvent = useCallback(
    (targetEventId: string) => {
      const index = findAnchorIndex(rows, { itemId: targetEventId, eventId: targetEventId });
      if (index >= 0) {
        virtualizer.scrollToIndex(index, { align: 'center', behavior: 'smooth' });
        return;
      }
      // Not in the current window: reopen focused through the native owner.
      setFocusEventId(targetEventId);
    },
    [rows, virtualizer]
  );

  if (timelineState.status === 'unavailable') {
    return (
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Text size="T300">The native timeline is unavailable in this window.</Text>
      </Box>
    );
  }
  if (timelineState.status === 'loading') {
    return (
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Text size="T300">Opening native timeline…</Text>
      </Box>
    );
  }
  if (timelineState.status === 'error') {
    return (
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Text size="T300">{timelineState.error.message}</Text>
      </Box>
    );
  }

  if (!readyState) return null;
  const { snapshot } = readyState;
  const runAction = (action: () => Promise<void>) => {
    setActionError(undefined);
    void action().catch((error) => {
      setActionError(error instanceof Error ? error.message : 'Native timeline action failed.');
    });
  };

  const jumpToLatest = () => {
    followingLiveRef.current = true;
    programmaticScrollUntilRef.current = performance.now() + 500;
    setAtLiveBottom(true);
    if (rows.length > 0) {
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
    }
    runAction(async () => {
      await controller.jumpLatest();
      followingLiveRef.current = true;
      programmaticScrollUntilRef.current = performance.now() + 500;
      if (rows.length > 0) {
        virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      }
    });
  };

  return (
    <Box grow="Yes" direction="Column" style={{ minHeight: 0 }}>
      {replyDraft && (
        <Box
          direction="Column"
          gap="100"
          style={{
            padding: config.space.S200,
            borderBottom: '1px solid currentColor',
            opacity: 0.9,
          }}
        >
          <Text size="T200">
            Replying to {replyDraft.senderId}
            {replyDraft.threadRootEventId ? ' · in thread' : ''}
          </Text>
          <Text size="T200" style={{ whiteSpace: 'pre-wrap' }}>
            {replyDraft.body}
          </Text>
          <Button
            size="300"
            fill="Soft"
            onClick={() =>
              runAction(async () => {
                const result = await clearNativeComposerReplyDraft({ roomId });
                if (result === 'unavailable') {
                  throw new Error('Native reply draft clear is unavailable.');
                }
                refreshReplyDraft();
              })
            }
          >
            Cancel reply
          </Button>
        </Box>
      )}
      {actionError && <Text size="T300">{actionError}</Text>}
      <Box grow="Yes" style={{ minHeight: 0, position: 'relative' }}>
        <Scroll ref={scrollRef} visibility="Hover" style={{ height: '100%' }}>
          {snapshot.pagination.backward === 'loading' && (
            <Box justifyContent="Center" style={{ padding: config.space.S200 }}>
              <Spinner size="200" aria-label="Loading older messages" />
            </Box>
          )}
          {rows.length === 0 ? (
            <Box
              alignItems="Center"
              justifyContent="Center"
              style={{ minHeight: '100%', padding: config.space.S400 }}
            >
              <Text size="T300">No messages in this view yet.</Text>
            </Box>
          ) : null}
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative', width: '100%' }}>
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const row = rows[virtualItem.index];
              if (!row) return null;
              return (
                <div
                  key={virtualItem.key}
                  ref={virtualizer.measureElement}
                  data-index={virtualItem.index}
                  data-native-timeline-row-kind={row.kind}
                  data-native-timeline-event-id={rowEventId(row)}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    transform: `translateY(${virtualItem.start}px)`,
                    width: '100%',
                  }}
                >
                  <NativeTimelineRow
                    row={row}
                    grouped={isGroupedWithPrevious(
                      virtualItem.index > 0 ? rows[virtualItem.index - 1] : undefined,
                      row
                    )}
                    groupsNext={isGroupedWithPrevious(
                      row,
                      virtualItem.index + 1 < rows.length
                        ? rows[virtualItem.index + 1]
                        : undefined
                    )}
                    roomId={roomId}
                    pinnedEventIds={snapshot.pinnedEventIds}
                    sourceEncrypted={sourceEncrypted}
                    onActionError={setActionError}
                    onReplyDraftChanged={refreshReplyDraft}
                    onFocusEvent={onFocusEvent}
                  />
                </div>
              );
            })}
          </div>
          {snapshot.pagination.forward === 'loading' && (
            <Box justifyContent="Center" style={{ padding: config.space.S200 }}>
              <Spinner size="200" aria-label="Loading newer messages" />
            </Box>
          )}
        </Scroll>
        {shouldShowJumpToLatest(readyState.selectedPosition.kind, atLiveBottom) && (
          <Box
            style={{ position: 'absolute', right: config.space.S400, bottom: config.space.S300 }}
          >
            <TooltipProvider
              position="Top"
              offset={4}
              tooltip={
                <Tooltip>
                  <Text>Jump to latest</Text>
                </Tooltip>
              }
            >
              {(triggerRef) => (
                <IconButton
                  ref={triggerRef}
                  variant="SurfaceVariant"
                  radii="Pill"
                  outlined
                  size="300"
                  aria-label="Jump to latest"
                  onClick={jumpToLatest}
                >
                  <Icon src={Icons.ChevronBottom} size="300" />
                </IconButton>
              )}
            </TooltipProvider>
          </Box>
        )}
      </Box>
    </Box>
  );
}
