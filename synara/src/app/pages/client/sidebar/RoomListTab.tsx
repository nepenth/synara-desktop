import React from 'react';
import { Icon, IconSrc } from 'folds';
import {
  SidebarAvatar,
  SidebarItem,
  SidebarItemBadge,
  SidebarItemTooltip,
} from '../../../components/sidebar';
import { UnreadBadge } from '../../../components/unread-badge';
import { useSelectedSpace } from '../../../hooks/router/useSelectedSpace';
import { useNativeNavigationScope } from '../../../state/hooks/navigationUnread';
import { useRoomUnread } from '../../../state/hooks/unread';
import { roomToUnreadAtom } from '../../../state/room/roomToUnread';
import { isMacOS } from '../../../utils/user-agent';
import {
  ROOM_LIST_PANE_ID,
  ROOM_LIST_TOGGLE_ID,
  roomListShortcut,
  roomListToggleBadge,
  roomListToggleLabel,
  roomListToggleTitle,
} from '../../../features/room-list-pane/roomListPane';
import {
  useRoomListCollapsed,
  useToggleRoomList,
} from '../../../features/room-list-pane/useRoomListPane';

/** A window with a left column; folds has no sidebar glyph. Filled: the list is shown. */
const RoomListIconSrc: IconSrc = (filled) => (
  <>
    <path
      fillRule="evenodd"
      clipRule="evenodd"
      d="M5 4H19C20.1 4 21 4.9 21 6V18C21 19.1 20.1 20 19 20H5C3.9 20 3 19.1 3 18V6C3 4.9 3.9 4 5 4ZM5 6V18H19V6H5Z"
      fill="currentColor"
    />
    <path d={filled ? 'M5 6H10.5V18H5V6Z' : 'M9 6H10.75V18H9V6Z'} fill="currentColor" />
  </>
);

/**
 * Shows or hides the room list pane (⌘⇧D / Ctrl+Shift+D). While the list is
 * hidden it carries the unread that list would show: the Home list's, or on
 * a space route the space's, by the rail's rule.
 */
export function RoomListTab() {
  const [collapsed] = useRoomListCollapsed();
  const toggleRoomList = useToggleRoomList();
  const selectedSpaceId = useSelectedSpace();
  const home = useNativeNavigationScope('merged');
  const space = useRoomUnread(selectedSpaceId ?? '', roomToUnreadAtom);

  const badge = selectedSpaceId
    ? roomListToggleBadge(collapsed, !!space, space?.highlight ?? 0)
    : roomListToggleBadge(collapsed, !!home.unread, home.attention);
  const shortcut = roomListShortcut(isMacOS());

  return (
    <SidebarItem>
      <SidebarItemTooltip tooltip={`${roomListToggleTitle(collapsed)} (${shortcut.label})`}>
        {(triggerRef) => (
          <SidebarAvatar
            as="button"
            id={ROOM_LIST_TOGGLE_ID}
            ref={triggerRef}
            outlined
            aria-label={roomListToggleLabel(collapsed, badge)}
            aria-expanded={!collapsed}
            aria-controls={ROOM_LIST_PANE_ID}
            aria-keyshortcuts={shortcut.ariaKeyShortcuts}
            onClick={() => toggleRoomList()}
          >
            <Icon src={RoomListIconSrc} filled={!collapsed} />
          </SidebarAvatar>
        )}
      </SidebarItemTooltip>
      {badge.kind !== 'none' && (
        <SidebarItemBadge hasCount={badge.kind === 'count'}>
          <UnreadBadge
            highlight={badge.kind === 'count'}
            count={badge.kind === 'count' ? badge.count : 0}
          />
        </SidebarItemBadge>
      )}
    </SidebarItem>
  );
}
