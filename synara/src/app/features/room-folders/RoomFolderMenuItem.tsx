import React, { MouseEventHandler, useState } from 'react';
import {
  Box,
  Icon,
  Icons,
  Line,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Spinner,
  Text,
  color,
  config,
  toRem,
} from 'folds';
import FocusTrap from '../../components/FocusTrap';
import { stopPropagation } from '../../utils/keyboard';
import { useNativeRoomListSnapshot, useNativeRoomSummary } from '../../state/room-list/roomList';
import type { RoomSummary } from '../matrix-dto/room';
import {
  useOpenRoomFolderDialog,
  useRoomFolderAction,
  useRoomFolderCommands,
  useRoomFoldersReadOnly,
} from './roomFolderState';

/** Core files joined rooms that are neither DMs nor spaces; favorites too. */
export const canFileRoomInFolder = (room: RoomSummary | undefined): boolean =>
  !!room && room.membership === 'join' && !room.isDirect && !room.isSpace;

type RoomFolderMenuItemProps = {
  roomId: string;
  /** Closes the room menu after a change. */
  requestClose: () => void;
};

/**
 * "Move to folder" in a room's menu: a submenu (the notification switcher's
 * pattern) with every folder (a check on the current one), New folder… and,
 * when filed, Remove from folder. Nothing for DMs, spaces, or while a newer
 * client's folder format is stored.
 */
export function RoomFolderMenuItem({ roomId, requestClose }: RoomFolderMenuItemProps) {
  const room = useNativeRoomSummary(roomId);
  const readOnly = useRoomFoldersReadOnly();
  const { folders } = useNativeRoomListSnapshot().presentation;
  const openDialog = useOpenRoomFolderDialog();
  const commands = useRoomFolderCommands();
  const { busy, error, run } = useRoomFolderAction();
  const [pending, setPending] = useState<string | null>();
  const [anchor, setAnchor] = useState<RectCords>();

  if (readOnly || !canFileRoomInFolder(room)) return null;
  const currentFolderId = room?.folderId;

  const handleOpen: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setAnchor(evt.currentTarget.getBoundingClientRect());
  };
  const close = () => setAnchor(undefined);

  const file = async (folderId: string | null) => {
    if (busy) return;
    if (folderId !== null && folderId === currentFolderId) {
      close();
      requestClose();
      return;
    }
    setPending(folderId);
    const done = await run(() => commands.setRoom(roomId, folderId));
    setPending(undefined);
    if (done) {
      close();
      requestClose();
    }
  };

  return (
    <PopOut
      anchor={anchor}
      offset={5}
      position="Right"
      align="Start"
      content={
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: close,
            clickOutsideDeactivates: true,
            isKeyForward: (evt: KeyboardEvent) =>
              evt.key === 'ArrowDown' || evt.key === 'ArrowRight',
            isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp' || evt.key === 'ArrowLeft',
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu style={{ maxWidth: toRem(240), width: '100vw' }} data-testid="room-folder-submenu">
            {folders.length > 0 && (
              <>
                <Box
                  direction="Column"
                  gap="100"
                  style={{
                    padding: config.space.S100,
                    maxHeight: toRem(320),
                    overflowY: 'auto',
                  }}
                >
                  {folders.map((folder) => {
                    const current = folder.id === currentFolderId;
                    return (
                      <MenuItem
                        key={folder.id}
                        variant="Surface"
                        fill="None"
                        size="300"
                        radii="300"
                        aria-pressed={current}
                        disabled={busy}
                        onClick={() => void file(folder.id)}
                        after={
                          pending === folder.id ? (
                            <Spinner size="100" variant="Secondary" />
                          ) : current ? (
                            <Icon size="100" src={Icons.Check} />
                          ) : undefined
                        }
                      >
                        <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                          {folder.name}
                        </Text>
                      </MenuItem>
                    );
                  })}
                </Box>
                <Line variant="Surface" size="300" />
              </>
            )}
            <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
              <MenuItem
                variant="Surface"
                fill="None"
                size="300"
                radii="300"
                disabled={busy}
                after={<Icon size="100" src={Icons.Plus} />}
                onClick={() => {
                  close();
                  requestClose();
                  openDialog({ kind: 'create', roomId });
                }}
              >
                <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                  New folder…
                </Text>
              </MenuItem>
              {currentFolderId && (
                <MenuItem
                  variant="Surface"
                  fill="None"
                  size="300"
                  radii="300"
                  disabled={busy}
                  onClick={() => void file(null)}
                  after={
                    pending === null ? (
                      <Spinner size="100" variant="Secondary" />
                    ) : (
                      <Icon size="100" src={Icons.Cross} />
                    )
                  }
                >
                  <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
                    Remove from folder
                  </Text>
                </MenuItem>
              )}
              {error && (
                <Text
                  as="p"
                  size="T200"
                  role="alert"
                  style={{ paddingInline: config.space.S200, color: color.Critical.Main }}
                >
                  {error}
                </Text>
              )}
            </Box>
          </Menu>
        </FocusTrap>
      }
    >
      <MenuItem
        variant="Surface"
        fill="None"
        size="300"
        radii="300"
        aria-haspopup="menu"
        aria-expanded={!!anchor}
        aria-pressed={!!anchor}
        onClick={handleOpen}
        after={<Icon size="100" src={Icons.ChevronRight} />}
      >
        <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
          Move to folder
        </Text>
      </MenuItem>
    </PopOut>
  );
}
