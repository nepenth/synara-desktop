import React from 'react';
import { Icon, IconButton, IconSrc, Text, Tooltip, TooltipProvider } from 'folds';
import { UnreadBadge } from '../../components/unread-badge';
import {
  PageHeader,
  usePageNavCollapsed,
  useRoomListHiddenBesidePage,
} from '../../components/page';
import { useSelectedSpace } from '../../hooks/router/useSelectedSpace';
import { useRoomUnread } from '../../state/hooks/unread';
import { roomToUnreadAtom } from '../../state/room/roomToUnread';
import { useNativeNavigationScope } from '../../state/hooks/navigationUnread';
import { isMacOS } from '../../utils/user-agent';
import {
  ROOM_LIST_PANE_ID,
  ROOM_LIST_TOGGLE_ID,
  roomListShortcut,
  roomListToggleBadge,
  roomListToggleLabel,
  roomListToggleTitle,
} from './roomListPane';
import { useToggleRoomList } from './useRoomListPane';
import * as css from './RoomListToggleButton.css';

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

function ShortcutTooltip({
  title,
  children,
}: {
  title: string;
  children: Parameters<typeof TooltipProvider>[0]['children'];
}) {
  const shortcut = roomListShortcut(isMacOS());
  return (
    <TooltipProvider
      delay={400}
      position="Bottom"
      tooltip={
        <Tooltip>
          <Text size="T300">{`${title} (${shortcut.label})`}</Text>
        </Tooltip>
      }
    >
      {children}
    </TooltipProvider>
  );
}

/**
 * The hidden list's unread, by the rail's Slack rule (a red count for
 * mentions and DMs, a dot for plain unread): the open space's on a space
 * route, Home's (rooms and DMs) everywhere else. With the list hidden its
 * bottom bar is hidden too, so this is the one place that unread shows.
 */
const useHiddenListBadge = () => {
  const selectedSpaceId = useSelectedSpace();
  const space = useRoomUnread(selectedSpaceId ?? '', roomToUnreadAtom);
  const home = useNativeNavigationScope('merged');
  return selectedSpaceId
    ? roomListToggleBadge(true, !!space, space?.highlight ?? 0)
    : roomListToggleBadge(true, !!home.unread, home.attention);
};

/**
 * "Show room list" for a content header (N5). Renders only while the list
 * beside this page is hidden; it takes focus when ⌘⇧D hides the list from
 * inside it (`ROOM_LIST_TOGGLE_ID`), and showing the list moves focus back in.
 */
export function ShowRoomListButton() {
  const hidden = useRoomListHiddenBesidePage();
  if (!hidden) return null;
  return <ShowRoomListButtonVisible />;
}

function ShowRoomListButtonVisible() {
  const toggleRoomList = useToggleRoomList();
  const badge = useHiddenListBadge();
  const shortcut = roomListShortcut(isMacOS());
  return (
    <ShortcutTooltip title={roomListToggleTitle(true)}>
      {(triggerRef) => (
        <span className={css.ToggleWrap}>
          <IconButton
            ref={triggerRef}
            id={ROOM_LIST_TOGGLE_ID}
            size="300"
            radii="300"
            variant="SurfaceVariant"
            fill="None"
            aria-label={roomListToggleLabel(true, badge)}
            aria-expanded={false}
            aria-controls={ROOM_LIST_PANE_ID}
            aria-keyshortcuts={shortcut.ariaKeyShortcuts}
            onClick={() => toggleRoomList({ revealFocus: true })}
          >
            <Icon size="200" src={RoomListIconSrc} />
          </IconButton>
          {badge.kind !== 'none' && (
            <span className={css.ToggleBadge} aria-hidden="true" data-testid="room-list-badge">
              <UnreadBadge
                highlight={badge.kind === 'count'}
                count={badge.kind === 'count' ? badge.count : 0}
              />
            </span>
          )}
        </span>
      )}
    </ShortcutTooltip>
  );
}

/**
 * A slim header holding only "Show room list", for pages that have no
 * desktop header of their own (Welcome, Featured, the create forms).
 */
export function ShowRoomListHeader() {
  const hidden = useRoomListHiddenBesidePage();
  if (!hidden) return null;
  return (
    <PageHeader outlined={false}>
      <ShowRoomListButtonVisible />
    </PageHeader>
  );
}

/** "Hide room list" in the list's own header, on desktop where the list can hide. */
export function HideRoomListButton() {
  const collapsed = usePageNavCollapsed();
  const toggleRoomList = useToggleRoomList();
  if (collapsed === undefined) return null;
  const shortcut = roomListShortcut(isMacOS());
  return (
    <ShortcutTooltip title={roomListToggleTitle(false)}>
      {(triggerRef) => (
        <IconButton
          ref={triggerRef}
          size="300"
          radii="300"
          variant="Surface"
          fill="None"
          aria-label={roomListToggleLabel(false, { kind: 'none' })}
          aria-expanded
          aria-controls={ROOM_LIST_PANE_ID}
          aria-keyshortcuts={shortcut.ariaKeyShortcuts}
          data-testid="room-list-hide"
          onClick={() => toggleRoomList()}
        >
          <Icon size="200" src={RoomListIconSrc} filled />
        </IconButton>
      )}
    </ShortcutTooltip>
  );
}
