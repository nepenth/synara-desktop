import React, {
  KeyboardEventHandler,
  MouseEventHandler,
  forwardRef,
  useRef,
  useState,
} from 'react';
import {
  Box,
  Icon,
  IconButton,
  Icons,
  Text,
  Menu,
  MenuItem,
  config,
  PopOut,
  toRem,
  Line,
  RectCords,
  Badge,
  Spinner,
} from 'folds';
import { useFocusWithin, useHover } from 'react-aria';
import FocusTrap, { returnFocusOrFallback } from '../../components/FocusTrap';
import { NavItem, NavItemContent, NavItemOptions, NavLink } from '../../components/nav';
import { UnreadBadge, UnreadBadgeCenter } from '../../components/unread-badge';
import { usePowerLevels } from '../../hooks/usePowerLevels';
import { copyToClipboard } from '../../utils/dom';
import { setRoomReadStateWithNativeOwner } from '../../utils/nativeRoomReadStateOwner';
import { markAsReadFromExplicitUserAction } from '../../utils/notifications';
import { UseStateProvider } from '../../components/UseStateProvider';
import { LeaveRoomPrompt } from '../../components/leave-room-prompt';
import { useRoomTypingMember } from '../../hooks/useRoomTypingMembers';
import { TypingIndicator } from '../../components/typing-indicator';
import { listNavigationIndex, stopPropagation } from '../../utils/keyboard';
import type { EventedRoomReading } from '../../utils/roomEvents';
import { getMatrixToRoom } from '../../plugins/matrix-to';
import { getCanonicalAliasOrRoomId, isRoomAlias } from '../../utils/matrix';
import { getViaServers } from '../../plugins/via-servers';
import { useOpenRoomSettings } from '../../state/hooks/roomSettings';
import { useSpaceOptionally } from '../../hooks/useSpace';
import {
  getRoomNotificationModeIcon,
  RoomNotificationMode,
} from '../../hooks/useRoomsNotificationPreferences';
import { RoomNotificationModeSwitcher } from '../../components/RoomNotificationSwitcher';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { InviteUserPrompt } from '../../components/invite-user-prompt';
import { useRoomName } from '../../hooks/useRoomMeta';
import { useNativeRoomSummary, useNativeRoomUnread } from '../../state/room-list/roomList';
import { setRoomFavoriteWithNativeOwner } from '../../components/nativeRoomFavoriteOwner';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { LiveCallChip } from './LiveCallChip';
import { useNativeUserStatus } from '../matrix-presence/nativeUserStatus';
import * as css from './styles.css';
import * as depthCss from '../../styles/Depth.css';
import { getMyUserId } from '../../state/nativeIdentity';

type RoomNavItemMenuProps = {
  room: EventedRoomReading;
  requestClose: () => void;
  notificationMode?: RoomNotificationMode;
};
const RoomNavItemMenu = forwardRef<HTMLDivElement, RoomNavItemMenuProps>(
  ({ room, requestClose, notificationMode }, ref) => {
    const nativeRoom = useNativeRoomSummary(room.roomId);
    const unread = useNativeRoomUnread(room.roomId);
    const isFavorite = nativeRoom?.isFavorite === true;
    const [favoriteError, setFavoriteError] = useState<string>();
    const [favoriteBusy, setFavoriteBusy] = useState(false);
    const [readError, setReadError] = useState<string>();
    const [readBusy, setReadBusy] = useState(false);
    const powerLevels = usePowerLevels(room);

    const permissions = useRoomPermissions(powerLevels);
    const canInvite = permissions.action('invite');
    const openRoomSettings = useOpenRoomSettings();
    const space = useSpaceOptionally();

    const [invitePrompt, setInvitePrompt] = useState(false);

    const handleMarkAsRead = async () => {
      if (readBusy) return;
      setReadError(undefined);
      setReadBusy(true);
      try {
        await markAsReadFromExplicitUserAction(room.roomId);
        requestClose();
      } catch {
        setReadError("Couldn't mark this channel as read.");
      } finally {
        setReadBusy(false);
      }
    };

    const handleMarkAsUnread = async () => {
      if (readBusy) return;
      setReadError(undefined);
      setReadBusy(true);
      try {
        await setRoomReadStateWithNativeOwner(
          room.roomId,
          'mark_unread',
          isSynaraDesktop(),
          invokeDesktopWithAvailability
        );
        requestClose();
      } catch {
        setReadError("Couldn't mark this channel as unread.");
      } finally {
        setReadBusy(false);
      }
    };

    const handleToggleFavorite = async () => {
      if (favoriteBusy) return;
      setFavoriteError(undefined);
      setFavoriteBusy(true);
      try {
        await setRoomFavoriteWithNativeOwner(
          room.roomId,
          !isFavorite,
          isSynaraDesktop(),
          invokeDesktopWithAvailability
        );
        requestClose();
      } catch {
        setFavoriteError('Could not update favorite.');
      } finally {
        setFavoriteBusy(false);
      }
    };

    const handleInvite = () => {
      setInvitePrompt(true);
    };

    const handleCopyLink = async () => {
      const roomIdOrAlias = getCanonicalAliasOrRoomId(room.roomId);
      const viaServers = isRoomAlias(roomIdOrAlias) ? undefined : await getViaServers(room);
      copyToClipboard(getMatrixToRoom(roomIdOrAlias, viaServers));
      requestClose();
    };

    const handleRoomSettings = () => {
      openRoomSettings(room.roomId, space?.roomId);
      requestClose();
    };

    return (
      <Menu ref={ref} style={{ maxWidth: toRem(160), width: '100vw' }}>
        {invitePrompt && room && (
          <InviteUserPrompt
            room={room}
            requestClose={() => {
              setInvitePrompt(false);
              requestClose();
            }}
          />
        )}
        <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
          <MenuItem
            variant="Surface"
            fill="None"
            onClick={() => {
              if (unread) void handleMarkAsRead();
              else void handleMarkAsUnread();
            }}
            disabled={readBusy}
            size="300"
            after={<Icon size="100" src={unread ? Icons.CheckTwice : Icons.MessageUnread} />}
            radii="300"
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              {unread ? 'Mark as Read' : 'Mark as Unread'}
            </Text>
          </MenuItem>
          {readError && (
            <Text as="p" size="T200" role="alert" style={{ paddingInline: config.space.S200 }}>
              {readError}
            </Text>
          )}
          <MenuItem
            variant="Surface"
            fill="None"
            onClick={() => {
              void handleToggleFavorite();
            }}
            size="300"
            disabled={favoriteBusy}
            after={
              favoriteBusy ? (
                <Spinner size="100" variant="Secondary" />
              ) : (
                <Icon size="100" src={Icons.Pin} filled={isFavorite} />
              )
            }
            radii="300"
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              {isFavorite ? 'Remove from Favorites' : 'Add to Favorites'}
            </Text>
          </MenuItem>
          {favoriteError && (
            <Text as="p" size="T200" style={{ paddingInline: config.space.S200 }}>
              {favoriteError}
            </Text>
          )}
          <RoomNotificationModeSwitcher roomId={room.roomId} value={notificationMode}>
            {(handleOpen, opened, changing) => (
              <MenuItem
                variant="Surface"
                fill="None"
                size="300"
                after={
                  changing ? (
                    <Spinner size="100" variant="Secondary" />
                  ) : (
                    <Icon size="100" src={getRoomNotificationModeIcon(notificationMode)} />
                  )
                }
                radii="300"
                aria-pressed={opened}
                onClick={handleOpen}
              >
                <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                  Notifications
                </Text>
              </MenuItem>
            )}
          </RoomNotificationModeSwitcher>
        </Box>
        <Line variant="Surface" size="300" />
        <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
          <MenuItem
            onClick={handleInvite}
            variant="Surface"
            fill="None"
            size="300"
            after={<Icon size="100" src={Icons.UserPlus} />}
            radii="300"
            className={depthCss.quietInteractiveSurface}
            aria-pressed={invitePrompt}
            disabled={!canInvite}
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              Invite
            </Text>
          </MenuItem>
          <MenuItem
            variant="Surface"
            fill="None"
            onClick={handleCopyLink}
            size="300"
            after={<Icon size="100" src={Icons.Link} />}
            radii="300"
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              Copy Link
            </Text>
          </MenuItem>
          <MenuItem
            variant="Surface"
            fill="None"
            onClick={handleRoomSettings}
            size="300"
            after={<Icon size="100" src={Icons.Setting} />}
            radii="300"
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              Room Settings
            </Text>
          </MenuItem>
        </Box>
        <Line variant="Surface" size="300" />
        <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
          <UseStateProvider initial={false}>
            {(promptLeave, setPromptLeave) => (
              <>
                <MenuItem
                  onClick={() => setPromptLeave(true)}
                  variant="Critical"
                  fill="None"
                  size="300"
                  after={<Icon size="100" src={Icons.ArrowGoLeft} />}
                  radii="300"
                  aria-pressed={promptLeave}
                >
                  <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                    Leave Room
                  </Text>
                </MenuItem>
                {promptLeave && (
                  <LeaveRoomPrompt
                    roomId={room.roomId}
                    onDone={requestClose}
                    onCancel={() => setPromptLeave(false)}
                  />
                )}
              </>
            )}
          </UseStateProvider>
        </Box>
      </Menu>
    );
  }
);

type RoomNavItemProps = {
  room: EventedRoomReading;
  selected: boolean;
  linkPath: string;
  notificationMode?: RoomNotificationMode;
};
function RoomNavItemImpl({ room, selected, notificationMode, linkPath }: RoomNavItemProps) {
  const [hover, setHover] = useState(false);
  const { hoverProps } = useHover({ onHoverChange: setHover });
  const { focusWithinProps } = useFocusWithin({ onFocusWithinChange: setHover });
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  // The options button is hover-revealed: focus inside the menu portal clears
  // the row's focus-within, so it must stay mounted while focus returns to it.
  const [returningFocus, setReturningFocus] = useState(false);
  const linkRef = useRef<HTMLAnchorElement>(null);
  // Per-room subscriptions: a row re-renders when its own summary or unread
  // state changes, not on every other room's activity (D10). A new summary
  // also refreshes the reads that go through the in-place room facade.
  const nativeRoom = useNativeRoomSummary(room.roomId);
  const unread = useNativeRoomUnread(room.roomId);
  const dmPeerId =
    nativeRoom?.isDirect === true &&
    typeof nativeRoom.directUserId === 'string' &&
    nativeRoom.hasActiveCall !== true
      ? nativeRoom.directUserId
      : undefined;
  const dmPeerStatus = useNativeUserStatus(dmPeerId);
  const showDmInCall = Boolean(dmPeerId && dmPeerStatus?.inCall);
  const typingMember = useRoomTypingMember(room.roomId).filter(
    (receipt) => receipt.userId !== getMyUserId()
  );

  const roomName = useRoomName(room);

  const handleContextMenu: MouseEventHandler<HTMLElement> = (evt) => {
    evt.preventDefault();
    setMenuAnchor({
      x: evt.clientX,
      y: evt.clientY,
      width: 0,
      height: 0,
    });
  };

  // Arrow keys, Home and End move between rooms; Tab still leaves the list.
  // Rows render in a virtual list, so only the rows on screen are reachable.
  const handleLinkKeyDown: KeyboardEventHandler<HTMLAnchorElement> = (evt) => {
    if (evt.altKey || evt.ctrlKey || evt.metaKey || evt.shiftKey) return;
    const links = Array.from(document.querySelectorAll<HTMLAnchorElement>('a[data-room-nav-link]'));
    const next = listNavigationIndex(evt.key, links.indexOf(evt.currentTarget), links.length);
    if (next === undefined) return;
    evt.preventDefault();
    links[next]?.focus();
  };

  const handleOpenMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuAnchor(evt.currentTarget.getBoundingClientRect());
  };

  const optionsVisible = hover || !!menuAnchor || returningFocus;

  return (
    <NavItem
      className={css.RoomSurface}
      variant="Surface"
      radii="300"
      highlight={unread !== undefined}
      selected={selected}
      data-hover={!!menuAnchor}
      onContextMenu={handleContextMenu}
      {...hoverProps}
      {...focusWithinProps}
    >
      <NavLink
        to={linkPath}
        ref={linkRef}
        data-room-nav-link
        aria-current={selected ? 'page' : undefined}
        onKeyDown={handleLinkKeyDown}
      >
        <NavItemContent>
          <Box as="span" grow="Yes" alignItems="Center" gap="200">
            <Text as="span" className={css.RoomGlyph} aria-hidden="true">
              #
            </Text>
            <Box as="span" grow="Yes" direction="Column">
              <Text
                as="span"
                size="Inherit"
                className={unread ? css.RoomName : `${css.RoomName} ${css.RoomNameRead}`}
                truncate
              >
                {roomName}
              </Text>
            </Box>
            {nativeRoom?.hasActiveCall && (
              <LiveCallChip participantCount={nativeRoom.activeCallParticipantCount} />
            )}
            {showDmInCall && !nativeRoom?.hasActiveCall && <LiveCallChip participantCount={1} />}
            {!optionsVisible && !unread && !selected && typingMember.length > 0 && (
              <Badge size="300" variant="Secondary" fill="Soft" radii="Pill" outlined>
                <TypingIndicator size="300" disableAnimation />
              </Badge>
            )}
            {!optionsVisible && unread && (
              <UnreadBadgeCenter>
                <UnreadBadge highlight={unread.highlight > 0} count={unread.total} />
              </UnreadBadgeCenter>
            )}
            {!optionsVisible && notificationMode !== RoomNotificationMode.Unset && (
              <Icon
                size="50"
                src={getRoomNotificationModeIcon(notificationMode)}
                aria-label={
                  ROOM_NOTIFICATION_MODE_LABELS[notificationMode ?? RoomNotificationMode.Unset]
                }
              />
            )}
          </Box>
        </NavItemContent>
      </NavLink>
      {optionsVisible && (
        <NavItemOptions>
          <PopOut
            id={`menu-${room.roomId}`}
            aria-expanded={!!menuAnchor}
            anchor={menuAnchor}
            offset={menuAnchor?.width === 0 ? 0 : undefined}
            alignOffset={menuAnchor?.width === 0 ? 0 : -5}
            position="Bottom"
            align={menuAnchor?.width === 0 ? 'Start' : 'End'}
            content={
              <FocusTrap
                focusTrapOptions={{
                  initialFocus: false,
                  // Keep the trigger mounted until focus is back on it (or the row).
                  setReturnFocus: returnFocusOrFallback(() => linkRef.current),
                  onDeactivate: () => {
                    setReturningFocus(true);
                    setMenuAnchor(undefined);
                  },
                  onPostDeactivate: () => setReturningFocus(false),
                  clickOutsideDeactivates: true,
                  isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
                  isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
                  escapeDeactivates: stopPropagation,
                }}
              >
                <RoomNavItemMenu
                  room={room}
                  requestClose={() => setMenuAnchor(undefined)}
                  notificationMode={notificationMode}
                />
              </FocusTrap>
            }
          >
            <IconButton
              className={depthCss.quietInteractiveSurface}
              onClick={handleOpenMenu}
              aria-expanded={!!menuAnchor}
              aria-controls={`menu-${room.roomId}`}
              aria-label="More Options"
              variant="Background"
              fill="None"
              size="300"
              radii="300"
            >
              <Icon size="50" src={Icons.VerticalDots} />
            </IconButton>
          </PopOut>
        </NavItemOptions>
      )}
    </NavItem>
  );
}

function areRoomNavItemPropsEqual(prev: RoomNavItemProps, next: RoomNavItemProps): boolean {
  // Room objects are normally stable and internal hooks subscribe to mutable room state. If the
  // SDK replaces an object, render against the new instance so those subscriptions move with it.
  if (prev.room !== next.room) return false;
  if (prev.selected !== next.selected) return false;
  if (prev.linkPath !== next.linkPath) return false;
  if (prev.notificationMode !== next.notificationMode) return false;
  return true;
}

/** Spoken names for the room row's notification icon (not the enum value). */
const ROOM_NOTIFICATION_MODE_LABELS: Record<RoomNotificationMode, string> = {
  [RoomNotificationMode.Unset]: 'Default notifications',
  [RoomNotificationMode.AllMessages]: 'Notifications: all messages',
  [RoomNotificationMode.SpecialMessages]: 'Notifications: mentions and keywords',
  [RoomNotificationMode.Mute]: 'Muted',
};

export const RoomNavItem = React.memo(RoomNavItemImpl, areRoomNavItemPropsEqual);
