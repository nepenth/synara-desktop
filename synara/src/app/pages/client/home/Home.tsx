import React, {
  MouseEventHandler,
  forwardRef,
  useCallback,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Box,
  Button,
  Icon,
  IconButton,
  Icons,
  Line,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Text,
  config,
  toRem,
} from 'folds';
import { useTranslation } from 'react-i18next';
import { useAtom, useAtomValue } from 'jotai';
import FocusTrap from '../../../components/FocusTrap';
import {
  NavEmptyCenter,
  NavLoading,
  NavEmptyLayout,
  NavSearchField,
  NavSectionHeader,
} from '../../../components/nav';
import {
  getCreatePath,
  getDirectCreatePath,
  getDirectRoomPath,
  getExplorePath,
  getHomeCreatePath,
  getHomeJoinPath,
  getHomeRoomPath,
} from '../../pathUtils';
import { getCanonicalAliasOrRoomId } from '../../../utils/matrix';
import { useSelectedRoom } from '../../../hooks/router/useSelectedRoom';
import {
  useHomeCreateSelected,
  useHomeJoinSelected,
  useHomeSelected,
} from '../../../hooks/router/useHomeSelected';
import { useHomeRooms } from './useHomeRooms';
import { useDirectRooms } from '../direct/useDirectRooms';
import {
  useDirectCreateSelected,
  useDirectSelected,
} from '../../../hooks/router/useDirectSelected';
import {
  buildHomeRoomListSections,
  readRoomListSorts,
  writeRoomListSort,
  type RoomListSection,
  type RoomListSort,
  type RoomListSorts,
} from './homeRoomList';
import {
  roomFoldersSignature,
  roomListNavSections,
  showsEmptyFolderLine,
  type RoomListSectionModel,
} from '../../../features/room-folders/roomFolderSections';
import {
  VirtualRoomSection,
  type RoomSectionReveal,
} from '../../../features/room-nav/VirtualRoomSection';
import { RoomFolderSectionHeader } from '../../../features/room-folders/RoomFolderSectionHeader';
import { RoomFolderDialogs } from '../../../features/room-folders/RoomFolderDialogs';
import {
  useOpenRoomFolderDialog,
  useRoomFoldersReadOnly,
  useRoomFoldersSnapshotLoader,
} from '../../../features/room-folders/roomFolderState';
import * as navCss from '../../../components/nav/styles.css';
import {
  useNativeRoomListLoaded,
  useNativeRoomListSnapshot,
} from '../../../state/room-list/roomList';
import { RoomListSortMenu, RoomNavItem } from '../../../features/room-nav';
import { roomToUnreadAtom } from '../../../state/room/roomToUnread';
import { useCategoryHandler } from '../../../hooks/useCategoryHandler';
import { useNavToActivePathMapper } from '../../../hooks/useNavToActivePathMapper';
import { PageNav, PageNavContent } from '../../../components/page';
import { NavPaneHeader } from '../../../features/nav-pane/NavPaneHeader';
import { NavBottomBarSlot } from '../../../features/nav-pane/NavBottomBarSlot';
import { HideRoomListButton } from '../../../features/room-list-pane/RoomListToggleButton';
import { useNativeNavigationScope } from '../../../state/hooks/navigationUnread';
import { useMarkRoomsAsRead } from '../../../hooks/useMarkRoomsAsRead';
import { useClosedNavCategoriesAtom } from '../../../state/hooks/closedNavCategories';
import { stopPropagation } from '../../../utils/keyboard';
import { SynaraMark } from '../../auth/SynaraMark';
import {
  getRoomNotificationMode,
  useRoomsNotificationPreferencesContext,
} from '../../../hooks/useRoomsNotificationPreferences';
import * as depthCss from '../../../styles/Depth.css';

import { getNativeRoom } from '../../../native/nativeSession';
import {
  useRoomListNavSource,
  type RoomListNavSource,
} from '../../../features/desktop-keyboard/roomListNavigation';

/** Sort options shared by every room-list section (Home, Direct, spaces). */
export const useRoomListSortOptions = () => {
  const { t } = useTranslation();
  return useMemo(
    () => [
      { value: 'recent' as const, label: t('modernization.nav.sort_recent', 'Recent activity') },
      { value: 'name' as const, label: t('modernization.nav.sort_name', 'Name') },
    ],
    [t]
  );
};

type HomeMenuProps = {
  requestClose: () => void;
};
const HomeMenu = forwardRef<HTMLDivElement, HomeMenuProps>(({ requestClose }, ref) => {
  const { t } = useTranslation();
  // One list (NAV-12): mark its rooms and its DMs read together.
  const { roomIds: orphanRooms, unread } = useNativeNavigationScope('merged');

  const { markRead, markingRead, markReadError } = useMarkRoomsAsRead();
  const foldersReadOnly = useRoomFoldersReadOnly();
  const openFolderDialog = useOpenRoomFolderDialog();

  const handleMarkAsRead = async () => {
    if (!unread || markingRead) return;
    if (await markRead(orphanRooms)) requestClose();
  };

  return (
    <Menu ref={ref} style={{ maxWidth: toRem(200), width: '100vw' }}>
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          variant="Surface"
          fill="None"
          onClick={handleMarkAsRead}
          size="300"
          after={<Icon size="100" src={Icons.CheckTwice} />}
          radii="300"
          aria-disabled={!unread}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {t('modernization.nav.mark_as_read', 'Mark as read')}
          </Text>
        </MenuItem>
        {markReadError && (
          <Text as="p" size="T200" role="alert" style={{ paddingInline: config.space.S200 }}>
            {markReadError}
          </Text>
        )}
        {/* List-wide actions live in this menu; section headers carry only their sort. */}
        {!foldersReadOnly && (
          <MenuItem
            variant="Surface"
            fill="None"
            onClick={() => {
              requestClose();
              openFolderDialog({ kind: 'create' });
            }}
            size="300"
            after={<Icon size="100" src={Icons.Plus} />}
            radii="300"
          >
            <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
              {t('modernization.nav.new_folder', 'New folder')}
            </Text>
          </MenuItem>
        )}
      </Box>
    </Menu>
  );
});

type HomeAddMenuProps = {
  requestClose: () => void;
};
/**
 * The "+" menu: every way into a new conversation (wave 3 folded the rail's
 * Explore and Add space items in here): create, join, new message, then
 * explore and create space.
 */
const HomeAddMenu = forwardRef<HTMLDivElement, HomeAddMenuProps>(({ requestClose }, ref) => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const go = (path: string) => {
    navigate(path);
    requestClose();
  };
  return (
    <Menu ref={ref} style={{ maxWidth: toRem(200), width: '100vw' }}>
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          variant="Surface"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.Plus} />}
          onClick={() => go(getHomeCreatePath())}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {t('modernization.nav.create_room', 'Create room')}
          </Text>
        </MenuItem>
        <MenuItem
          variant="Surface"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.Link} />}
          onClick={() => go(getHomeJoinPath())}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {t('modernization.nav.join_room', 'Join room')}
          </Text>
        </MenuItem>
        <MenuItem
          variant="Surface"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.User} />}
          onClick={() => go(getDirectCreatePath())}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {t('modernization.nav.new_message', 'New message')}
          </Text>
        </MenuItem>
      </Box>
      <Line variant="Surface" size="300" />
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          variant="Surface"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.Explore} />}
          onClick={() => go(getExplorePath())}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {t('modernization.nav.explore_rooms', 'Explore rooms')}
          </Text>
        </MenuItem>
        <MenuItem
          variant="Surface"
          fill="None"
          size="300"
          radii="300"
          after={<Icon size="100" src={Icons.Space} />}
          onClick={() => go(getCreatePath())}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            {t('modernization.nav.create_space', 'Create space')}
          </Text>
        </MenuItem>
      </Box>
    </Menu>
  );
});

function HomeHeader() {
  const { t } = useTranslation();
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const [addAnchor, setAddAnchor] = useState<RectCords>();
  const createRoomSelected = useHomeCreateSelected();
  const joinSelected = useHomeJoinSelected();
  const createDirectSelected = useDirectCreateSelected();

  const toggle =
    (
      set: React.Dispatch<React.SetStateAction<RectCords | undefined>>
    ): MouseEventHandler<HTMLButtonElement> =>
    (evt) => {
      const cords = evt.currentTarget.getBoundingClientRect();
      set((currentState) => (currentState ? undefined : cords));
    };

  const trapOptions = (close: () => void) => ({
    initialFocus: false as const,
    onDeactivate: close,
    clickOutsideDeactivates: true,
    isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
    isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
    escapeDeactivates: stopPropagation,
  });

  return (
    <NavPaneHeader
      // The switcher's own name for this list, not the homeserver's host name.
      title="Home"
      mark={<SynaraMark size={22} decorative />}
      actions={
        <>
          <HideRoomListButton />
          <IconButton
            className={depthCss.quietInteractiveSurface}
            aria-label={t('modernization.nav.add_room', 'Add room')}
            aria-haspopup="menu"
            aria-expanded={!!addAnchor}
            aria-pressed={!!addAnchor || createRoomSelected || joinSelected || createDirectSelected}
            size="300"
            radii="300"
            variant="Surface"
            onClick={toggle(setAddAnchor)}
          >
            <Icon src={Icons.Plus} size="200" />
          </IconButton>
          <IconButton
            aria-label={t('modernization.nav.home_options', 'Home options')}
            aria-pressed={!!menuAnchor}
            size="300"
            radii="300"
            variant="Surface"
            onClick={toggle(setMenuAnchor)}
          >
            <Icon src={Icons.VerticalDots} size="200" />
          </IconButton>
          <PopOut
            anchor={addAnchor}
            position="Bottom"
            align="End"
            offset={6}
            content={
              <FocusTrap focusTrapOptions={trapOptions(() => setAddAnchor(undefined))}>
                <HomeAddMenu requestClose={() => setAddAnchor(undefined)} />
              </FocusTrap>
            }
          />
          <PopOut
            anchor={menuAnchor}
            position="Bottom"
            align="End"
            offset={6}
            content={
              <FocusTrap focusTrapOptions={trapOptions(() => setMenuAnchor(undefined))}>
                <HomeMenu requestClose={() => setMenuAnchor(undefined)} />
              </FocusTrap>
            }
          />
        </>
      }
    />
  );
}

function HomeEmpty() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <NavEmptyCenter>
      <NavEmptyLayout
        icon={<Icon size="600" src={Icons.Hash} />}
        title={
          <Text size="H5" align="Center">
            {t('modernization.nav.no_rooms_title', 'No rooms')}
          </Text>
        }
        content={
          <Text size="T300" align="Center">
            {t('modernization.nav.no_rooms_description', 'You do not have any rooms yet.')}
          </Text>
        }
        options={
          <>
            <Button onClick={() => navigate(getHomeCreatePath())} variant="Secondary" size="300">
              <Text size="B300" truncate>
                {t('modernization.nav.create_room', 'Create room')}
              </Text>
            </Button>
            <Button
              onClick={() => navigate(getExplorePath())}
              variant="Secondary"
              fill="Soft"
              size="300"
            >
              <Text size="B300" truncate>
                {t('modernization.nav.explore_rooms', 'Explore rooms')}
              </Text>
            </Button>
          </>
        }
      />
    </NavEmptyCenter>
  );
}

/** Section titles; a folder's title is its name. */
const useHomeSectionLabels = (): Record<RoomListSection, string> => {
  const { t } = useTranslation();
  return useMemo(
    () => ({
      favorites: t('modernization.nav.favorites', 'Favorites'),
      rooms: t('modernization.nav.rooms', 'Rooms'),
      directs: t('modernization.nav.direct_messages', 'Direct messages'),
    }),
    [t]
  );
};

/**
 * The one Home list (NAV-12, Slack and iOS): Favorites, one section per room
 * folder, Rooms, Direct messages. It is the nav pane for both `/home/` and
 * `/direct/` routes, so a DM keeps its own route (and `IsDirectRoomProvider`)
 * while the list, its scroll position and the bottom bar's Home stay put.
 * `/approvals` borrows it as its list column (D2), so the bar never
 * disappears.
 *
 * Every section, its header and its rows are rows of one virtual list, so a
 * folder of hundreds of rooms costs what the Rooms section does.
 */
export function Home() {
  // `home` remembers the last place in this pane, room or DM, never Approvals.
  const homeRoute = useHomeSelected();
  const directRoute = useDirectSelected();
  useNavToActivePathMapper('home', homeRoute || directRoute);
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const rooms = useHomeRooms();
  const directs = useDirectRooms();
  const notificationPreferences = useRoomsNotificationPreferencesContext();
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const sortOptions = useRoomListSortOptions();
  const labels = useHomeSectionLabels();
  const roomsLabel = labels.rooms;

  const selectedRoomId = useSelectedRoom();
  const roomListLoaded = useNativeRoomListLoaded();
  const [closedCategories, setClosedCategories] = useAtom(useClosedNavCategoriesAtom());

  const nativeRoomList = useNativeRoomListSnapshot();
  const { presentation } = nativeRoomList;
  useRoomFoldersSnapshotLoader(roomFoldersSignature(presentation));
  const noRoomToDisplay =
    rooms.length === 0 && directs.length === 0 && presentation.folders.length === 0;

  const sortStorage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const [sorts, setSorts] = useState<RoomListSorts>(() => readRoomListSorts(sortStorage));
  const handleSectionSort = (section: RoomListSection) => (sort: RoomListSort) => {
    setSorts((current) => ({ ...current, [section]: sort }));
    writeRoomListSort(sortStorage, sort, section);
  };

  const hasUnread = useCallback((rId: string) => roomToUnread.has(rId), [roomToUnread]);
  const sections = useMemo(
    () =>
      buildHomeRoomListSections({
        roomIds: rooms,
        directIds: directs,
        presentation,
        sorts,
        closedCategories,
        hasUnread,
        selectedRoomId,
      }),
    [rooms, directs, presentation, sorts, closedCategories, hasUnread, selectedRoomId]
  );
  const directIdSet = useMemo(() => new Set(directs), [directs]);

  const roomLinkPath = useCallback(
    (roomId: string) => {
      const idOrAlias = getCanonicalAliasOrRoomId(roomId);
      return directIdSet.has(roomId) ? getDirectRoomPath(idOrAlias) : getHomeRoomPath(idOrAlias);
    },
    [directIdSet]
  );

  // Each section's virtual list registers how to scroll a row into view.
  const reveals = useRef(new Map<string, RoomSectionReveal>());
  const revealSetters = useRef(new Map<string, (reveal?: RoomSectionReveal) => void>());
  const revealSetter = (sectionId: string) => {
    let setter = revealSetters.current.get(sectionId);
    if (!setter) {
      setter = (reveal) => {
        if (reveal) reveals.current.set(sectionId, reveal);
        else reveals.current.delete(sectionId);
      };
      revealSetters.current.set(sectionId, setter);
    }
    return setter;
  };

  // Previous/next (unread) room walk these rows, not the rendered ones, in
  // display order: Favorites, each folder, Rooms, Direct messages.
  useRoomListNavSource(
    useMemo<RoomListNavSource>(
      () => ({
        selectedRoomId,
        sections: roomListNavSections(sections, roomLinkPath, (sectionId, index) =>
          reveals.current.get(sectionId)?.(index)
        ),
      }),
      [selectedRoomId, sections, roomLinkPath]
    )
  );

  const handleCategoryClick = useCategoryHandler(setClosedCategories, (categoryId) =>
    closedCategories.has(categoryId)
  );

  const renderHeader = (section: RoomListSectionModel, spaced: boolean) => {
    if (section.kind === 'folder' && section.folder) {
      return (
        <RoomFolderSectionHeader
          folderId={section.folder.id}
          name={section.folder.name}
          index={section.folder.index}
          folderCount={section.folder.count}
          categoryId={section.categoryId}
          closed={section.closed}
          onToggle={handleCategoryClick}
          spaced={spaced}
          unread={section.unread}
        />
      );
    }
    const kind = section.kind as RoomListSection;
    return (
      <NavSectionHeader
        label={labels[kind]}
        sectionId={section.id}
        spaced={spaced}
        closed={section.closed}
        categoryId={section.categoryId}
        onToggle={handleCategoryClick}
        actions={
          <RoomListSortMenu
            sectionLabel={labels[kind]}
            value={sorts[kind]}
            options={sortOptions}
            onChange={handleSectionSort(kind)}
          />
        }
      />
    );
  };

  const renderRoom = (roomId: string) => {
    const room = getNativeRoom(roomId);
    if (!room) return null;
    return (
      <RoomNavItem
        room={room}
        selected={selectedRoomId === roomId}
        linkPath={roomLinkPath(roomId)}
        notificationMode={getRoomNotificationMode(notificationPreferences, room.roomId)}
      />
    );
  };

  const emptyFolderLine = (
    <Text
      as="p"
      size="T200"
      priority="300"
      className={navCss.NavSectionEmpty}
      data-room-folder-empty=""
    >
      {t('modernization.nav.folder_no_rooms', 'No rooms')}
    </Text>
  );

  return (
    <PageNav label={roomsLabel} header={<HomeHeader />} footer={<NavBottomBarSlot />}>
      <NavSearchField />
      {noRoomToDisplay ? (
        roomListLoaded ? (
          <HomeEmpty />
        ) : (
          <NavLoading label={t('modernization.nav.loading_rooms', 'Loading rooms…')} />
        )
      ) : (
        <PageNavContent scrollRef={scrollRef}>
          {/* Sections are spaced by their headers (`spaced`), not a gap. */}
          <Box direction="Column" data-testid="home-room-list">
            {sections.map((section, index) => (
              <VirtualRoomSection
                key={section.id}
                sectionId={section.id}
                scrollRef={scrollRef}
                header={renderHeader(section, index > 0)}
                rows={section.rows}
                renderRoom={renderRoom}
                empty={showsEmptyFolderLine(section) ? emptyFolderLine : undefined}
                onReveal={revealSetter(section.id)}
              />
            ))}
          </Box>
        </PageNavContent>
      )}
      <RoomFolderDialogs />
    </PageNav>
  );
}
