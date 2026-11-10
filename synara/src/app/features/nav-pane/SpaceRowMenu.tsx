import React, { forwardRef, useState } from 'react';
import { Box, Icon, Icons, Line, Menu, MenuItem, Text, config, toRem } from 'folds';
import { useAtomValue } from 'jotai';
import type { EventedRoomReading } from '../../utils/roomEvents';
import { useRecursiveChildScopeFactory, useSpaceChildren } from '../../state/hooks/roomList';
import { roomToParentsAtom } from '../../state/room/roomToParents';
import { allRoomsAtom } from '../../state/room-list/roomList';
import { usePowerLevels } from '../../hooks/usePowerLevels';
import { useRoomsUnread } from '../../state/hooks/unread';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { useMarkRoomsAsRead } from '../../hooks/useMarkRoomsAsRead';
import { copyToClipboard } from '../../utils/dom';
import { getCanonicalAliasOrRoomId, isRoomAlias } from '../../utils/matrix';
import { getMatrixToRoom } from '../../plugins/matrix-to';
import { getViaServers } from '../../plugins/via-servers';
import { useOpenSpaceSettings } from '../../state/hooks/spaceSettings';
import { useRoomPermissions } from '../../hooks/useRoomPermissions';
import { InviteUserPrompt } from '../../components/invite-user-prompt';
import * as depthCss from '../../styles/Depth.css';

type SpaceRowMenuProps = {
  room: EventedRoomReading;
  requestClose: () => void;
  /** Absent for spaces that are always listed (no parent space). */
  onUnpin?: (roomId: string) => void;
};

/**
 * A space's context menu in the switcher (right-click, Shift+F10 or the
 * row's options button): the former rail menu, unchanged in what it offers.
 */
export const SpaceRowMenu = forwardRef<HTMLDivElement, SpaceRowMenuProps>(
  ({ room, requestClose, onUnpin }, ref) => {
    const roomToParents = useAtomValue(roomToParentsAtom);
    const powerLevels = usePowerLevels(room);

    const permissions = useRoomPermissions(powerLevels);
    const canInvite = permissions.action('invite');
    const openSpaceSettings = useOpenSpaceSettings();

    const [invitePrompt, setInvitePrompt] = useState(false);

    const allChild = useSpaceChildren(
      allRoomsAtom,
      room.roomId,
      useRecursiveChildScopeFactory(roomToParents)
    );
    const unread = useRoomsUnread(allChild, roomToUnreadAtom);

    const { markRead, markingRead, markReadError } = useMarkRoomsAsRead();

    const handleMarkAsRead = async () => {
      if (markingRead) return;
      if (await markRead(allChild)) requestClose();
    };

    const handleUnpin = () => {
      onUnpin?.(room.roomId);
      requestClose();
    };

    const handleCopyLink = async () => {
      const roomIdOrAlias = getCanonicalAliasOrRoomId(room.roomId);
      const viaServers = isRoomAlias(roomIdOrAlias) ? undefined : await getViaServers(room);
      copyToClipboard(getMatrixToRoom(roomIdOrAlias, viaServers));
      requestClose();
    };

    const handleInvite = () => {
      setInvitePrompt(true);
    };

    const handleRoomSettings = () => {
      openSpaceSettings(room.roomId);
      requestClose();
    };

    return (
      <Menu
        ref={ref}
        aria-label={`${room.name} options`}
        style={{ maxWidth: toRem(200), width: '100vw' }}
      >
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
            onClick={handleMarkAsRead}
            size="300"
            after={<Icon size="100" src={Icons.CheckTwice} />}
            radii="300"
            disabled={!unread}
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              Mark as read
            </Text>
          </MenuItem>
          {markReadError && (
            <Text as="p" size="T200" role="alert" style={{ paddingInline: config.space.S200 }}>
              {markReadError}
            </Text>
          )}
          {onUnpin && (
            <MenuItem
              variant="Surface"
              fill="None"
              size="300"
              radii="300"
              onClick={handleUnpin}
              after={<Icon size="100" src={Icons.Pin} />}
            >
              <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                Unpin
              </Text>
            </MenuItem>
          )}
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
              Copy link
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
              Space settings
            </Text>
          </MenuItem>
        </Box>
      </Menu>
    );
  }
);
