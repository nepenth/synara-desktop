import React, { MouseEventHandler, forwardRef, useState } from 'react';
import { useAtomValue } from 'jotai';
import FocusTrap from 'focus-trap-react';
import {
  Box,
  Text,
  Overlay,
  OverlayCenter,
  OverlayBackdrop,
  IconButton,
  Icon,
  Icons,
  Tooltip,
  TooltipProvider,
  Menu,
  MenuItem,
  toRem,
  config,
  Line,
  PopOut,
  RectCords,
  Badge,
  Spinner,
} from 'folds';
import { useNavigate } from 'react-router-dom';
import { useStateEvent } from '../../hooks/useStateEvent';
import { PageHeader } from '../../components/page';
import { UseStateProvider } from '../../components/UseStateProvider';
import { RoomTopicViewer } from '../../components/room-topic-viewer';
import { StateEvent } from '../../../types/matrix/room';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { useRoom } from '../../hooks/useRoom';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { useSpaceOptionally } from '../../hooks/useSpace';
import { getHomeSearchPath, getSpaceSearchPath, withSearchParam } from '../../pages/pathUtils';
import { getCanonicalAliasOrRoomId, isRoomAlias } from '../../utils/matrix';
import { _SearchPathSearchParams } from '../../pages/paths';
import * as css from './RoomViewHeader.css';
import { useRoomUnread } from '../../state/hooks/unread';
import { usePowerLevelsContext } from '../../hooks/usePowerLevels';
import {
  markAsReadFromExplicitUserActionInBackground,
  markAsUnread,
} from '../../utils/notifications';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { copyToClipboard } from '../../utils/dom';
import { LeaveRoomPrompt } from '../../components/leave-room-prompt';
import { useRoomName, useRoomTopic } from '../../hooks/useRoomMeta';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';
import { stopPropagation } from '../../utils/keyboard';
import { getMatrixToRoom } from '../../plugins/matrix-to';
import { getViaServers } from '../../plugins/via-servers';
import { BackRouteHandler } from '../../components/BackRouteHandler';
import { useRoomPinnedEvents } from '../../hooks/useRoomPinnedEvents';
import { RoomPinMenu } from './room-pin-menu';
import { useOpenRoomSettings } from '../../state/hooks/roomSettings';
import { RoomNotificationModeSwitcher } from '../../components/RoomNotificationSwitcher';
import {
  getRoomNotificationMode,
  getRoomNotificationModeIcon,
  useRoomsNotificationPreferencesContext,
} from '../../hooks/useRoomsNotificationPreferences';
import { JumpToTime } from './jump-to-time';
import { useRoomNavigate } from '../../hooks/useRoomNavigate';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { InviteUserPrompt } from '../../components/invite-user-prompt';
import { ContainerColor } from '../../styles/ContainerColor.css';
import { getRoomNotesSummary } from '../../utils/roomNotes';
import { roomNotesContentAtom } from '../../state/roomNotesList';
import { RoomNotesPanel } from './room-notes/RoomNotesPanel';
import type { RoomSidePanelType } from './RoomSidePanel';
import * as depthCss from '../../styles/Depth.css';

type Room = ReturnType<typeof useRoom>;

type RoomMenuProps = {
  room: Room;
  requestClose: () => void;
};
const RoomMenu = forwardRef<HTMLDivElement, RoomMenuProps>(({ room, requestClose }, ref) => {
  const mx = useMatrixClient();
  const unread = useRoomUnread(room.roomId, roomToUnreadAtom);
  const powerLevels = usePowerLevelsContext();
  const creators = useRoomCreators(room);

  const permissions = useRoomPermissions(creators, powerLevels);
  const canInvite = permissions.action('invite', mx.getSafeUserId());
  const notificationPreferences = useRoomsNotificationPreferencesContext();
  const notificationMode = getRoomNotificationMode(notificationPreferences, room.roomId);
  const { navigateRoom } = useRoomNavigate();

  const [invitePrompt, setInvitePrompt] = useState(false);

  const handleMarkAsRead = () => {
    markAsReadFromExplicitUserActionInBackground(mx, room.roomId);
    requestClose();
  };

  const handleMarkAsUnread = () => {
    markAsUnread(mx, room.roomId);
    requestClose();
  };

  const handleInvite = () => {
    setInvitePrompt(true);
  };

  const handleCopyLink = async () => {
    const roomIdOrAlias = getCanonicalAliasOrRoomId(mx, room.roomId);
    const viaServers = isRoomAlias(roomIdOrAlias) ? undefined : await getViaServers(room);
    copyToClipboard(getMatrixToRoom(roomIdOrAlias, viaServers));
    requestClose();
  };

  const openSettings = useOpenRoomSettings();
  const parentSpace = useSpaceOptionally();
  const handleOpenSettings = () => {
    openSettings(room.roomId, parentSpace?.roomId);
    requestClose();
  };

  return (
    <Menu
      ref={ref}
      className={depthCss.floatingSurface}
      style={{ maxWidth: toRem(160), width: '100vw' }}
    >
      {invitePrompt && (
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
          onClick={unread ? handleMarkAsRead : handleMarkAsUnread}
          size="300"
          after={<Icon size="100" src={unread ? Icons.CheckTwice : Icons.MessageUnread} />}
          radii="300"
          className={depthCss.quietInteractiveSurface}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {unread ? 'Mark as Read' : 'Mark as Unread'}
          </Text>
        </MenuItem>
        <RoomNotificationModeSwitcher roomId={room.roomId} value={notificationMode}>
          {(handleOpen, opened, changing) => (
            <MenuItem
              size="300"
              after={
                changing ? (
                  <Spinner size="100" variant="Secondary" />
                ) : (
                  <Icon size="100" src={getRoomNotificationModeIcon(notificationMode)} />
                )
              }
              radii="300"
              className={depthCss.quietInteractiveSurface}
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
          variant="Primary"
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
          onClick={handleCopyLink}
          size="300"
          after={<Icon size="100" src={Icons.Link} />}
          radii="300"
          className={depthCss.quietInteractiveSurface}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            Copy Link
          </Text>
        </MenuItem>
        <MenuItem
          onClick={handleOpenSettings}
          size="300"
          after={<Icon size="100" src={Icons.Setting} />}
          radii="300"
          className={depthCss.quietInteractiveSurface}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            Room Settings
          </Text>
        </MenuItem>
        <UseStateProvider initial={false}>
          {(promptJump, setPromptJump) => (
            <>
              <MenuItem
                onClick={() => setPromptJump(true)}
                size="300"
                after={<Icon size="100" src={Icons.RecentClock} />}
                radii="300"
                className={depthCss.quietInteractiveSurface}
                aria-pressed={promptJump}
              >
                <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                  Jump to Time
                </Text>
              </MenuItem>
              {promptJump && (
                <JumpToTime
                  onSubmit={(eventId) => {
                    setPromptJump(false);
                    navigateRoom(room.roomId, eventId);
                    requestClose();
                  }}
                  onCancel={() => setPromptJump(false)}
                />
              )}
            </>
          )}
        </UseStateProvider>
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
                className={depthCss.quietInteractiveSurface}
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
});

type RoomViewHeaderProps = {
  activeSidePanel?: RoomSidePanelType | 'members';
  onToggleSidePanel?: (panel: RoomSidePanelType) => void;
  onToggleMembers?: () => void;
};

export function RoomViewHeader({
  activeSidePanel,
  onToggleSidePanel,
  onToggleMembers,
}: RoomViewHeaderProps) {
  const navigate = useNavigate();
  const mx = useMatrixClient();
  const screenSize = useScreenSizeContext();
  const room = useRoom();
  const space = useSpaceOptionally();
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const [pinMenuAnchor, setPinMenuAnchor] = useState<RectCords>();
  const [notesOverlayOpen, setNotesOverlayOpen] = useState(false);

  const pinnedEvents = useRoomPinnedEvents(room);
  const notesContent = useAtomValue(roomNotesContentAtom);
  const notesSummary = getRoomNotesSummary(notesContent, room.roomId);
  const encryptionEvent = useStateEvent(room, StateEvent.RoomEncryption);
  const encryptedRoom = !!encryptionEvent;
  const name = useRoomName(room);
  const topic = useRoomTopic(room);

  const [peopleDrawer, setPeopleDrawer] = useSetting(settingsAtom, 'isPeopleDrawer');
  const pinsOpen = activeSidePanel === 'pins' || !!pinMenuAnchor;
  const notesOpen = activeSidePanel === 'notes' || notesOverlayOpen;

  const handleSearchClick = () => {
    if (onToggleSidePanel) {
      onToggleSidePanel('search');
      return;
    }
    const searchParams: _SearchPathSearchParams = {
      rooms: room.roomId,
    };
    const path = space
      ? getSpaceSearchPath(getCanonicalAliasOrRoomId(mx, space.roomId))
      : getHomeSearchPath();
    navigate(withSearchParam(path, searchParams));
  };

  const handleOpenMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuAnchor(evt.currentTarget.getBoundingClientRect());
  };

  const handleOpenPinMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    if (onToggleSidePanel) {
      onToggleSidePanel('pins');
      return;
    }
    setPinMenuAnchor(evt.currentTarget.getBoundingClientRect());
  };

  const handleOpenNotes = () => {
    if (onToggleSidePanel) {
      onToggleSidePanel('notes');
      return;
    }
    setNotesOverlayOpen(true);
  };

  const handleMemberToggle = () => {
    if (onToggleMembers) {
      onToggleMembers();
      return;
    }
    setPeopleDrawer(!peopleDrawer);
  };

  return (
    <PageHeader
      data-tauri-drag-region
      className={ContainerColor({ variant: 'SurfaceVariant' })}
      balance={screenSize === ScreenSize.Mobile}
    >
      <Box grow="Yes" gap="300">
        {screenSize === ScreenSize.Mobile && (
          <BackRouteHandler>
            {(onBack) => (
              <Box shrink="No" alignItems="Center">
                <IconButton
                  className={depthCss.quietInteractiveSurface}
                  fill="None"
                  onClick={onBack}
                >
                  <Icon src={Icons.ArrowLeft} />
                </IconButton>
              </Box>
            )}
          </BackRouteHandler>
        )}
        <Box grow="Yes" alignItems="Center" gap="300">
          {screenSize !== ScreenSize.Mobile && (
            <Text as="span" className={css.RoomChannelGlyph} aria-hidden="true">
              #
            </Text>
          )}
          <Box direction="Column">
            <Text size={topic ? 'H5' : 'H3'} className={css.RoomTitle} truncate>
              {name}
            </Text>
            {topic && (
              <UseStateProvider initial={false}>
                {(viewTopic, setViewTopic) => (
                  <>
                    <Overlay open={viewTopic} backdrop={<OverlayBackdrop />}>
                      <OverlayCenter>
                        <FocusTrap
                          focusTrapOptions={{
                            initialFocus: false,
                            clickOutsideDeactivates: true,
                            onDeactivate: () => setViewTopic(false),
                            escapeDeactivates: stopPropagation,
                          }}
                        >
                          <RoomTopicViewer
                            name={name}
                            topic={topic}
                            requestClose={() => setViewTopic(false)}
                          />
                        </FocusTrap>
                      </OverlayCenter>
                    </Overlay>
                    <Text
                      as="button"
                      type="button"
                      onClick={() => setViewTopic(true)}
                      className={css.HeaderTopic}
                      size="T200"
                      priority="300"
                      truncate
                    >
                      {topic}
                    </Text>
                  </>
                )}
              </UseStateProvider>
            )}
          </Box>
        </Box>

        <Box shrink="No">
          {!encryptedRoom && (
            <TooltipProvider
              position="Bottom"
              offset={4}
              tooltip={
                <Tooltip>
                  <Text>Search</Text>
                </Tooltip>
              }
            >
              {(triggerRef) => (
                <IconButton
                  className={depthCss.quietInteractiveSurface}
                  fill="None"
                  ref={triggerRef}
                  onClick={handleSearchClick}
                >
                  <Icon size="400" src={Icons.Search} />
                </IconButton>
              )}
            </TooltipProvider>
          )}
          <TooltipProvider
            position="Bottom"
            offset={4}
            tooltip={
              <Tooltip>
                <Text>Pinned Messages</Text>
              </Tooltip>
            }
          >
            {(triggerRef) => (
              <IconButton
                className={depthCss.quietInteractiveSurface}
                fill="None"
                style={{ position: 'relative' }}
                onClick={handleOpenPinMenu}
                ref={triggerRef}
                aria-pressed={pinsOpen}
              >
                {pinnedEvents.length > 0 && (
                  <Badge
                    style={{
                      position: 'absolute',
                      left: toRem(3),
                      top: toRem(3),
                    }}
                    variant="Secondary"
                    size="400"
                    fill="Solid"
                    radii="Pill"
                  >
                    <Text as="span" size="L400">
                      {pinnedEvents.length}
                    </Text>
                  </Badge>
                )}
                <Icon size="400" src={Icons.Pin} filled={pinsOpen} />
              </IconButton>
            )}
          </TooltipProvider>
          {!onToggleSidePanel && (
            <PopOut
              anchor={pinMenuAnchor}
              position="Bottom"
              content={
                <FocusTrap
                  focusTrapOptions={{
                    initialFocus: false,
                    returnFocusOnDeactivate: false,
                    onDeactivate: () => setPinMenuAnchor(undefined),
                    clickOutsideDeactivates: true,
                    isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
                    isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
                    escapeDeactivates: stopPropagation,
                  }}
                >
                  <RoomPinMenu
                    room={room as unknown as React.ComponentProps<typeof RoomPinMenu>['room']}
                    requestClose={() => setPinMenuAnchor(undefined)}
                  />
                </FocusTrap>
              }
            />
          )}

          <TooltipProvider
            position="Bottom"
            offset={4}
            tooltip={
              <Tooltip>
                <Text>Personal Notes</Text>
              </Tooltip>
            }
          >
            {(triggerRef) => (
              <IconButton
                className={depthCss.quietInteractiveSurface}
                fill="None"
                style={{ position: 'relative' }}
                ref={triggerRef}
                onClick={handleOpenNotes}
                aria-pressed={notesOpen}
              >
                {notesSummary.total > 0 && (
                  <Badge
                    style={{
                      position: 'absolute',
                      left: toRem(3),
                      top: toRem(3),
                    }}
                    variant={notesSummary.activeTodos > 0 ? 'Warning' : 'Secondary'}
                    size="400"
                    fill="Solid"
                    radii="Pill"
                  >
                    <Text as="span" size="L400">
                      {notesSummary.activeTodos || notesSummary.total}
                    </Text>
                  </Badge>
                )}
                <Icon size="400" src={Icons.Pencil} filled={notesOpen} />
              </IconButton>
            )}
          </TooltipProvider>
          {!onToggleSidePanel && (
            <Overlay open={notesOverlayOpen} backdrop={<OverlayBackdrop />}>
              <OverlayCenter>
                <FocusTrap
                  focusTrapOptions={{
                    initialFocus: false,
                    returnFocusOnDeactivate: false,
                    onDeactivate: () => setNotesOverlayOpen(false),
                    clickOutsideDeactivates: true,
                    escapeDeactivates: stopPropagation,
                  }}
                >
                  <RoomNotesPanel room={room} requestClose={() => setNotesOverlayOpen(false)} />
                </FocusTrap>
              </OverlayCenter>
            </Overlay>
          )}

          {screenSize === ScreenSize.Desktop && (
            <TooltipProvider
              position="Bottom"
              offset={4}
              tooltip={
                <Tooltip>
                  <Text>
                    {activeSidePanel === 'members' || peopleDrawer
                      ? 'Hide Members'
                      : 'Show Members'}
                  </Text>
                </Tooltip>
              }
            >
              {(triggerRef) => (
                <IconButton
                  className={depthCss.quietInteractiveSurface}
                  fill="None"
                  ref={triggerRef}
                  aria-label={
                    activeSidePanel === 'members' || peopleDrawer ? 'Hide Members' : 'Show Members'
                  }
                  onClick={handleMemberToggle}
                >
                  <Icon size="400" src={Icons.User} />
                </IconButton>
              )}
            </TooltipProvider>
          )}

          <TooltipProvider
            position="Bottom"
            align="End"
            offset={4}
            tooltip={
              <Tooltip>
                <Text>More Options</Text>
              </Tooltip>
            }
          >
            {(triggerRef) => (
              <IconButton
                className={depthCss.quietInteractiveSurface}
                fill="None"
                onClick={handleOpenMenu}
                ref={triggerRef}
                aria-label="More Options"
                aria-pressed={!!menuAnchor}
              >
                <Icon size="400" src={Icons.VerticalDots} filled={!!menuAnchor} />
              </IconButton>
            )}
          </TooltipProvider>
          <PopOut
            anchor={menuAnchor}
            position="Bottom"
            align="End"
            content={
              <FocusTrap
                focusTrapOptions={{
                  initialFocus: false,
                  returnFocusOnDeactivate: false,
                  onDeactivate: () => setMenuAnchor(undefined),
                  clickOutsideDeactivates: true,
                  isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
                  isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
                  escapeDeactivates: stopPropagation,
                }}
              >
                <RoomMenu room={room} requestClose={() => setMenuAnchor(undefined)} />
              </FocusTrap>
            }
          />
        </Box>
      </Box>
    </PageHeader>
  );
}
