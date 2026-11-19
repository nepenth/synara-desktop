import React, { MouseEventHandler, useState } from 'react';
import {
  Box,
  Icon,
  IconButton,
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
import { NavSectionHeader } from '../../components/nav';
import { UnreadBadge } from '../../components/unread-badge';
import * as navCss from '../../components/nav/styles.css';
import { stopPropagation } from '../../utils/keyboard';
import {
  useOpenRoomFolderDialog,
  useRoomFolderAction,
  useRoomFolderCommands,
  useRoomFoldersReadOnly,
} from './roomFolderState';
import type { RoomFolderUnread } from './roomFolderSections';

type RoomFolderSectionHeaderProps = {
  folderId: string;
  name: string;
  /** Position among all folders (display order), for Move up / Move down. */
  index: number;
  folderCount: number;
  categoryId: string;
  closed: boolean;
  onToggle: MouseEventHandler<HTMLButtonElement>;
  spaced?: boolean;
  /** The folder's unread; shown only while the section is collapsed. */
  unread?: RoomFolderUnread;
};

/** Spoken after the folder name when the collapsed header carries unread. */
const unreadLabel = (unread: RoomFolderUnread): string =>
  unread.highlight > 0
    ? `, ${unread.highlight} ${unread.highlight === 1 ? 'mention' : 'mentions'}`
    : ', unread';

/**
 * A folder's section header: the shared `NavSectionHeader` (name and
 * collapse), its unread while collapsed, and Rename / Move up / Move down /
 * Delete folder behind the hover kebab and on right-click.
 */
export function RoomFolderSectionHeader({
  folderId,
  name,
  index,
  folderCount,
  categoryId,
  closed,
  onToggle,
  spaced,
  unread,
}: RoomFolderSectionHeaderProps) {
  const readOnly = useRoomFoldersReadOnly();
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const showBadge = closed && unread !== undefined && (unread.highlight > 0 || unread.total > 0);

  const handleContextMenu: MouseEventHandler<HTMLDivElement> = (evt) => {
    if (readOnly) return;
    evt.preventDefault();
    setMenuAnchor({ x: evt.clientX, y: evt.clientY, width: 0, height: 0 });
  };

  return (
    <NavSectionHeader
      label={name}
      toggleLabel={showBadge && unread ? `${name}${unreadLabel(unread)}` : undefined}
      closed={closed}
      categoryId={categoryId}
      onToggle={onToggle}
      spaced={spaced}
      onContextMenu={handleContextMenu}
      sectionId={`folder:${folderId}`}
      badge={
        showBadge && unread ? (
          <span aria-hidden="true" data-room-folder-badge="">
            <UnreadBadge highlight={unread.highlight > 0} count={unread.highlight} />
          </span>
        ) : undefined
      }
      actions={
        readOnly ? undefined : (
          <PopOut
            anchor={menuAnchor}
            offset={menuAnchor?.width === 0 ? 0 : undefined}
            position="Bottom"
            align={menuAnchor?.width === 0 ? 'Start' : 'End'}
            content={
              <FocusTrap
                focusTrapOptions={{
                  initialFocus: false,
                  onDeactivate: () => setMenuAnchor(undefined),
                  clickOutsideDeactivates: true,
                  isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
                  isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
                  escapeDeactivates: stopPropagation,
                }}
              >
                <RoomFolderMenu
                  folderId={folderId}
                  name={name}
                  index={index}
                  folderCount={folderCount}
                  requestClose={() => setMenuAnchor(undefined)}
                />
              </FocusTrap>
            }
          >
            <IconButton
              className={navCss.NavSmallIconButton}
              size="300"
              variant="Surface"
              radii="300"
              fill={menuAnchor ? 'Soft' : 'None'}
              aria-haspopup="menu"
              aria-expanded={!!menuAnchor}
              aria-label={`${name} folder options`}
              title={`${name} folder options`}
              onClick={(evt) =>
                setMenuAnchor(menuAnchor ? undefined : evt.currentTarget.getBoundingClientRect())
              }
            >
              <Icon className={navCss.NavSmallIcon} size="50" src={Icons.VerticalDots} />
            </IconButton>
          </PopOut>
        )
      }
    />
  );
}

type RoomFolderMenuProps = {
  folderId: string;
  name: string;
  index: number;
  folderCount: number;
  requestClose: () => void;
};

const RoomFolderMenu = React.forwardRef<HTMLDivElement, RoomFolderMenuProps>(
  ({ folderId, name, index, folderCount, requestClose }, ref) => {
    const openDialog = useOpenRoomFolderDialog();
    const commands = useRoomFolderCommands();
    const { busy, error, run } = useRoomFolderAction();
    const [moving, setMoving] = useState<'up' | 'down'>();

    const move = async (direction: 'up' | 'down') => {
      if (busy) return;
      setMoving(direction);
      const done = await run(() =>
        commands.move(folderId, direction === 'up' ? index - 1 : index + 1)
      );
      setMoving(undefined);
      if (done) requestClose();
    };

    const item = (
      label: string,
      icon: typeof Icons.Pencil,
      onClick: () => void,
      options: { disabled?: boolean; busy?: boolean; critical?: boolean } = {}
    ) => (
      <MenuItem
        variant={options.critical ? 'Critical' : 'Surface'}
        fill="None"
        size="300"
        radii="300"
        disabled={options.disabled}
        onClick={onClick}
        after={
          options.busy ? <Spinner size="100" variant="Secondary" /> : <Icon size="100" src={icon} />
        }
      >
        <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
          {label}
        </Text>
      </MenuItem>
    );

    return (
      <Menu
        ref={ref}
        style={{ maxWidth: toRem(200), width: '100vw' }}
        data-testid="room-folder-menu"
        data-room-folder-id={folderId}
      >
        <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
          {item('Rename', Icons.Pencil, () => {
            requestClose();
            openDialog({ kind: 'rename', folderId, name });
          })}
          {item('Move up', Icons.ArrowTop, () => void move('up'), {
            disabled: busy || index <= 0,
            busy: moving === 'up',
          })}
          {item('Move down', Icons.ArrowBottom, () => void move('down'), {
            disabled: busy || index >= folderCount - 1,
            busy: moving === 'down',
          })}
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
        <Line variant="Surface" size="300" />
        <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
          {item(
            'Delete folder',
            Icons.Delete,
            () => {
              requestClose();
              openDialog({ kind: 'delete', folderId, name });
            },
            { critical: true, disabled: busy }
          )}
        </Box>
      </Menu>
    );
  }
);
