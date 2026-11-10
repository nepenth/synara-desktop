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
import { useSharedScrollVirtualizer } from '../../../hooks/useSharedScrollVirtualizer';
import { useAtom, useAtomValue } from 'jotai';
import FocusTrap from '../../../components/FocusTrap';
import {
  NavCategory,
  NavEmptyCenter,
  NavLoading,
  NavEmptyLayout,
  NavPaneTitle,
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
import { useHomeCreateSelected, useHomeJoinSelected } from '../../../hooks/router/useHomeSelected';
import { useHomeRooms } from './useHomeRooms';
import { useDirectRooms } from '../direct/useDirectRooms';
import { useDirectCreateSelected } from '../../../hooks/router/useDirectSelected';
import {
  favoriteRoomIdSet,
  partitionHomeRooms,
  readRoomListSort,
  sortHomeRoomIds,
  writeRoomListSort,
  type RoomListSort,
} from './homeRoomList';
import {
  useNativeRoomListLoaded,
  useNativeRoomListSnapshot,
} from '../../../state/room-list/roomList';
import { VirtualTile } from '../../../components/virtualizer';
import { RoomListSortMenu, RoomNavItem } from '../../../features/room-nav';
import { makeNavCategoryId } from '../../../state/closedNavCategories';
import { roomToUnreadAtom } from '../../../state/room/roomToUnread';
import { useCategoryHandler } from '../../../hooks/useCategoryHandler';
import { useNavToActivePathMapper } from '../../../hooks/useNavToActivePathMapper';
import { PageNav, PageNavHeader, PageNavContent } from '../../../components/page';
import { useNativeNavigationScope } from '../../../state/hooks/navigationUnread';
import { useMarkRoomsAsRead } from '../../../hooks/useMarkRoomsAsRead';
import { useClosedNavCategoriesAtom } from '../../../state/hooks/closedNavCategories';
import { stopPropagation } from '../../../utils/keyboard';
import SynaraPNG from '../../../../../public/res/png/synara.png';
import {
  getRoomNotificationMode,
  useRoomsNotificationPreferencesContext,
} from '../../../hooks/useRoomsNotificationPreferences';
import * as depthCss from '../../../styles/Depth.css';
import { getActiveSession } from '../../../state/sessionBootstrap';
import { homeserverDisplayName } from './homeIdentity';

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
  const session = getActiveSession();
  const serverName = homeserverDisplayName(session?.baseUrl, session?.userId);
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
    <>
      <PageNavHeader data-tauri-drag-region>
        <Box alignItems="Center" grow="Yes" gap="300">
          <Box grow="Yes" alignItems="Center" gap="200">
            <img
              src={SynaraPNG}
              alt=""
              width={22}
              height={22}
              style={{ borderRadius: 6, display: 'block' }}
            />
            <NavPaneTitle text={serverName} />
          </Box>
          <Box shrink="No" gap="100">
            <IconButton
              className={depthCss.quietInteractiveSurface}
              aria-label={t('modernization.nav.add_room', 'Add room')}
              aria-haspopup="menu"
              aria-expanded={!!addAnchor}
              aria-pressed={
                !!addAnchor || createRoomSelected || joinSelected || createDirectSelected
              }
              variant="Surface"
              onClick={toggle(setAddAnchor)}
            >
              <Icon src={Icons.Plus} size="200" />
            </IconButton>
            <IconButton
              aria-label={t('modernization.nav.home_options', 'Home options')}
              aria-pressed={!!menuAnchor}
              variant="Surface"
              onClick={toggle(setMenuAnchor)}
            >
              <Icon src={Icons.VerticalDots} size="200" />
            </IconButton>
          </Box>
        </Box>
      </PageNavHeader>
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

const DEFAULT_CATEGORY_ID = makeNavCategoryId('home', 'room');
const FAVORITES_CATEGORY_ID = makeNavCategoryId('home', 'favorites');
/** Same id the former Direct pane used, so its collapsed state carries over. */
const DIRECT_CATEGORY_ID = makeNavCategoryId('direct', 'direct');

/** A collapsed section keeps its unread and selected rows visible (Slack). */
const visibleWhenClosed = (
  roomIds: string[],
  closed: boolean,
  hasUnread: (roomId: string) => boolean,
  selectedRoomId: string | undefined
): string[] =>
  closed ? roomIds.filter((rId) => hasUnread(rId) || rId === selectedRoomId) : roomIds;

/**
 * The one Home list (NAV-12, Slack and iOS): Favorites, Rooms, Direct
 * messages. It is the nav pane for both `/home/` and `/direct/` routes, so a
 * DM keeps its own route (and `IsDirectRoomProvider`) while the list, its
 * scroll position and the rail's Home tab stay put.
 */
export function Home() {
  // `home` remembers the last place in this pane, room or DM.
  useNavToActivePathMapper('home');
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const rooms = useHomeRooms();
  const directs = useDirectRooms();
  const notificationPreferences = useRoomsNotificationPreferencesContext();
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const sortOptions = useRoomListSortOptions();

  const selectedRoomId = useSelectedRoom();
  const roomListLoaded = useNativeRoomListLoaded();
  const noRoomToDisplay = rooms.length === 0 && directs.length === 0;
  const [closedCategories, setClosedCategories] = useAtom(useClosedNavCategoriesAtom());

  const nativeRoomList = useNativeRoomListSnapshot();
  const sortStorage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const [favoriteSort, setFavoriteSort] = useState<RoomListSort>(() =>
    readRoomListSort(sortStorage, 'favorites')
  );
  const [roomsSort, setRoomsSort] = useState<RoomListSort>(() =>
    readRoomListSort(sortStorage, 'rooms')
  );
  const [directSort, setDirectSort] = useState<RoomListSort>(() =>
    readRoomListSort(sortStorage, 'directs')
  );
  const favoriteIds = useMemo(
    () => favoriteRoomIdSet(nativeRoomList.presentation),
    [nativeRoomList.presentation]
  );
  // Favorites span rooms and DMs, as on iOS (`RoomListFavorites.sections`).
  const { favoriteRoomIds, remainingRoomIds, remainingDirectIds } = useMemo(() => {
    const roomPartition = partitionHomeRooms(rooms, favoriteIds);
    const directPartition = partitionHomeRooms(directs, favoriteIds);
    return {
      favoriteRoomIds: [...roomPartition.favoriteRoomIds, ...directPartition.favoriteRoomIds],
      remainingRoomIds: roomPartition.remainingRoomIds,
      remainingDirectIds: directPartition.remainingRoomIds,
    };
  }, [rooms, directs, favoriteIds]);
  const directIdSet = useMemo(() => new Set(directs), [directs]);

  const favoritesClosed = closedCategories.has(FAVORITES_CATEGORY_ID);
  const roomsClosed = closedCategories.has(DEFAULT_CATEGORY_ID);
  const directsClosed = closedCategories.has(DIRECT_CATEGORY_ID);
  const hasUnread = useCallback((rId: string) => roomToUnread.has(rId), [roomToUnread]);

  const sortedFavoriteRoomIds = useMemo(
    () =>
      visibleWhenClosed(
        sortHomeRoomIds(favoriteRoomIds, nativeRoomList.presentation, favoriteSort),
        favoritesClosed,
        hasUnread,
        selectedRoomId
      ),
    [
      favoriteRoomIds,
      nativeRoomList.presentation,
      favoriteSort,
      favoritesClosed,
      hasUnread,
      selectedRoomId,
    ]
  );

  const mainRoomIds = useMemo(
    () =>
      visibleWhenClosed(
        sortHomeRoomIds(remainingRoomIds, nativeRoomList.presentation, roomsSort),
        roomsClosed,
        hasUnread,
        selectedRoomId
      ),
    [
      remainingRoomIds,
      nativeRoomList.presentation,
      roomsSort,
      roomsClosed,
      hasUnread,
      selectedRoomId,
    ]
  );

  const sortedDirectIds = useMemo(
    () =>
      visibleWhenClosed(
        sortHomeRoomIds(remainingDirectIds, nativeRoomList.presentation, directSort),
        directsClosed,
        hasUnread,
        selectedRoomId
      ),
    [
      remainingDirectIds,
      nativeRoomList.presentation,
      directSort,
      directsClosed,
      hasUnread,
      selectedRoomId,
    ]
  );

  const handleFavoriteSort = (sort: RoomListSort) => {
    setFavoriteSort(sort);
    writeRoomListSort(sortStorage, sort, 'favorites');
  };

  const handleRoomsSort = (sort: RoomListSort) => {
    setRoomsSort(sort);
    writeRoomListSort(sortStorage, sort, 'rooms');
  };

  const handleDirectSort = (sort: RoomListSort) => {
    setDirectSort(sort);
    writeRoomListSort(sortStorage, sort, 'directs');
  };

  const getRoomKey = useCallback((index: number) => mainRoomIds[index], [mainRoomIds]);
  const { virtualizer, listRef, scrollMargin } = useSharedScrollVirtualizer(
    scrollRef,
    mainRoomIds.length,
    getRoomKey,
    10,
    // Room rows (`RoomSurface`) are 32px.
    32
  );
  const getDirectKey = useCallback((index: number) => sortedDirectIds[index], [sortedDirectIds]);
  const {
    virtualizer: directVirtualizer,
    listRef: directListRef,
    scrollMargin: directScrollMargin,
  } = useSharedScrollVirtualizer(scrollRef, sortedDirectIds.length, getDirectKey, 10, 32);

  // Previous/next (unread) room walk these rows, not the rendered ones.
  useRoomListNavSource(
    useMemo<RoomListNavSource>(
      () => ({
        selectedRoomId,
        sections: [
          {
            rows: sortedFavoriteRoomIds,
            pathFor: (roomId) => {
              const idOrAlias = getCanonicalAliasOrRoomId(roomId);
              return directIdSet.has(roomId)
                ? getDirectRoomPath(idOrAlias)
                : getHomeRoomPath(idOrAlias);
            },
          },
          {
            rows: mainRoomIds,
            pathFor: (roomId) => getHomeRoomPath(getCanonicalAliasOrRoomId(roomId)),
            reveal: (index) => virtualizer.scrollToIndex(index, { align: 'auto' }),
          },
          {
            rows: sortedDirectIds,
            pathFor: (roomId) => getDirectRoomPath(getCanonicalAliasOrRoomId(roomId)),
            reveal: (index) => directVirtualizer.scrollToIndex(index, { align: 'auto' }),
          },
        ],
      }),
      [
        selectedRoomId,
        sortedFavoriteRoomIds,
        directIdSet,
        mainRoomIds,
        virtualizer,
        sortedDirectIds,
        directVirtualizer,
      ]
    )
  );

  const handleCategoryClick = useCategoryHandler(setClosedCategories, (categoryId) =>
    closedCategories.has(categoryId)
  );

  const favoritesLabel = t('modernization.nav.favorites', 'Favorites');
  const roomsLabel = t('modernization.nav.rooms', 'Rooms');
  const directsLabel = t('modernization.nav.direct_messages', 'Direct messages');

  const roomLinkPath = (roomId: string) => {
    const idOrAlias = getCanonicalAliasOrRoomId(roomId);
    return directIdSet.has(roomId) ? getDirectRoomPath(idOrAlias) : getHomeRoomPath(idOrAlias);
  };

  return (
    <PageNav label={roomsLabel}>
      <HomeHeader />
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
          <Box direction="Column">
            {favoriteRoomIds.length > 0 && (
              <NavCategory>
                <NavSectionHeader
                  label={favoritesLabel}
                  closed={favoritesClosed}
                  categoryId={FAVORITES_CATEGORY_ID}
                  onToggle={handleCategoryClick}
                  actions={
                    <RoomListSortMenu
                      sectionLabel={favoritesLabel}
                      value={favoriteSort}
                      options={sortOptions}
                      onChange={handleFavoriteSort}
                    />
                  }
                />
                {sortedFavoriteRoomIds.map((roomId) => {
                  const room = getNativeRoom(roomId);
                  if (!room) return null;
                  const selected = selectedRoomId === roomId;
                  return (
                    <RoomNavItem
                      key={roomId}
                      room={room}
                      selected={selected}
                      linkPath={roomLinkPath(roomId)}
                      notificationMode={getRoomNotificationMode(
                        notificationPreferences,
                        room.roomId
                      )}
                    />
                  );
                })}
              </NavCategory>
            )}

            {remainingRoomIds.length > 0 && (
              <NavCategory>
                <NavSectionHeader
                  label={roomsLabel}
                  spaced={favoriteRoomIds.length > 0}
                  closed={roomsClosed}
                  categoryId={DEFAULT_CATEGORY_ID}
                  onToggle={handleCategoryClick}
                  actions={
                    <RoomListSortMenu
                      sectionLabel={roomsLabel}
                      value={roomsSort}
                      options={sortOptions}
                      onChange={handleRoomsSort}
                    />
                  }
                />
                <div
                  ref={listRef}
                  style={{
                    position: 'relative',
                    height: virtualizer.getTotalSize(),
                  }}
                >
                  {virtualizer.getVirtualItems().map((vItem) => {
                    const roomId = mainRoomIds[vItem.index];
                    const room = getNativeRoom(roomId);
                    if (!room) return null;
                    const selected = selectedRoomId === roomId;

                    return (
                      <VirtualTile
                        virtualItem={vItem}
                        scrollMargin={scrollMargin}
                        key={vItem.key}
                        ref={virtualizer.measureElement}
                      >
                        <RoomNavItem
                          room={room}
                          selected={selected}
                          linkPath={getHomeRoomPath(getCanonicalAliasOrRoomId(roomId))}
                          notificationMode={getRoomNotificationMode(
                            notificationPreferences,
                            room.roomId
                          )}
                        />
                      </VirtualTile>
                    );
                  })}
                </div>
              </NavCategory>
            )}

            {remainingDirectIds.length > 0 && (
              <NavCategory>
                <NavSectionHeader
                  label={directsLabel}
                  spaced={favoriteRoomIds.length > 0 || remainingRoomIds.length > 0}
                  closed={directsClosed}
                  categoryId={DIRECT_CATEGORY_ID}
                  onToggle={handleCategoryClick}
                  actions={
                    <RoomListSortMenu
                      sectionLabel={directsLabel}
                      value={directSort}
                      options={sortOptions}
                      onChange={handleDirectSort}
                    />
                  }
                />
                <div
                  ref={directListRef}
                  style={{
                    position: 'relative',
                    height: directVirtualizer.getTotalSize(),
                  }}
                >
                  {directVirtualizer.getVirtualItems().map((vItem) => {
                    const roomId = sortedDirectIds[vItem.index];
                    const room = getNativeRoom(roomId);
                    if (!room) return null;
                    const selected = selectedRoomId === roomId;

                    return (
                      <VirtualTile
                        virtualItem={vItem}
                        scrollMargin={directScrollMargin}
                        key={vItem.key}
                        ref={directVirtualizer.measureElement}
                      >
                        <RoomNavItem
                          room={room}
                          selected={selected}
                          linkPath={getDirectRoomPath(getCanonicalAliasOrRoomId(roomId))}
                          notificationMode={getRoomNotificationMode(
                            notificationPreferences,
                            room.roomId
                          )}
                        />
                      </VirtualTile>
                    );
                  })}
                </div>
              </NavCategory>
            )}
          </Box>
        </PageNavContent>
      )}
    </PageNav>
  );
}
