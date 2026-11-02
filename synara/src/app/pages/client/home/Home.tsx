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
  NavSearchField,
  NavSectionHeader,
} from '../../../components/nav';
import {
  getExplorePath,
  getHomeCreatePath,
  getHomeJoinPath,
  getHomeRoomPath,
  getHomeSearchPath,
} from '../../pathUtils';
import { getCanonicalAliasOrRoomId } from '../../../utils/matrix';
import { useSelectedRoom } from '../../../hooks/router/useSelectedRoom';
import {
  useHomeCreateSelected,
  useHomeJoinSelected,
  useHomeSearchSelected,
} from '../../../hooks/router/useHomeSelected';
import { useHomeRooms } from './useHomeRooms';
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
  const { roomIds: orphanRooms, unread } = useNativeNavigationScope('home');

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
/** The "+" menu: the former Create Room / Join with Address utility rows. */
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
            {t('modernization.nav.join_with_address', 'Join with address')}
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
            <Text size="H4" truncate>
              {serverName}
            </Text>
          </Box>
          <Box shrink="No" gap="100">
            <IconButton
              className={depthCss.quietInteractiveSurface}
              aria-label={t('modernization.nav.add_room', 'Add room')}
              aria-haspopup="menu"
              aria-expanded={!!addAnchor}
              aria-pressed={!!addAnchor || createRoomSelected || joinSelected}
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

export function Home() {
  useNavToActivePathMapper('home');
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const rooms = useHomeRooms();
  const notificationPreferences = useRoomsNotificationPreferencesContext();
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const navigate = useNavigate();
  const sortOptions = useRoomListSortOptions();

  const selectedRoomId = useSelectedRoom();
  const searchSelected = useHomeSearchSelected();
  const roomListLoaded = useNativeRoomListLoaded();
  const noRoomToDisplay = rooms.length === 0;
  const [closedCategories, setClosedCategories] = useAtom(useClosedNavCategoriesAtom());

  const nativeRoomList = useNativeRoomListSnapshot();
  const sortStorage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const [favoriteSort, setFavoriteSort] = useState<RoomListSort>(() =>
    readRoomListSort(sortStorage, 'favorites')
  );
  const [roomsSort, setRoomsSort] = useState<RoomListSort>(() =>
    readRoomListSort(sortStorage, 'rooms')
  );
  const favoriteIds = useMemo(
    () => favoriteRoomIdSet(nativeRoomList.presentation),
    [nativeRoomList.presentation]
  );
  const { favoriteRoomIds, remainingRoomIds } = useMemo(
    () => partitionHomeRooms(rooms, favoriteIds),
    [rooms, favoriteIds]
  );

  const favoritesClosed = closedCategories.has(FAVORITES_CATEGORY_ID);
  const roomsClosed = closedCategories.has(DEFAULT_CATEGORY_ID);

  // A collapsed section keeps its unread and selected rows visible (Slack).
  const sortedFavoriteRoomIds = useMemo(() => {
    const items = sortHomeRoomIds(favoriteRoomIds, nativeRoomList.presentation, favoriteSort);
    if (favoritesClosed) {
      return items.filter((rId) => roomToUnread.has(rId) || rId === selectedRoomId);
    }
    return items;
  }, [
    favoriteRoomIds,
    nativeRoomList.presentation,
    favoriteSort,
    favoritesClosed,
    roomToUnread,
    selectedRoomId,
  ]);

  const mainRoomIds = useMemo(() => {
    const items = sortHomeRoomIds(remainingRoomIds, nativeRoomList.presentation, roomsSort);
    if (roomsClosed) {
      return items.filter((rId) => roomToUnread.has(rId) || rId === selectedRoomId);
    }
    return items;
  }, [
    remainingRoomIds,
    nativeRoomList.presentation,
    roomsSort,
    roomsClosed,
    roomToUnread,
    selectedRoomId,
  ]);

  const handleFavoriteSort = (sort: RoomListSort) => {
    setFavoriteSort(sort);
    writeRoomListSort(sortStorage, sort, 'favorites');
  };

  const handleRoomsSort = (sort: RoomListSort) => {
    setRoomsSort(sort);
    writeRoomListSort(sortStorage, sort, 'rooms');
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

  const handleCategoryClick = useCategoryHandler(setClosedCategories, (categoryId) =>
    closedCategories.has(categoryId)
  );

  const favoritesLabel = t('modernization.nav.favorites', 'Favorites');
  const roomsLabel = t('modernization.nav.rooms', 'Rooms');

  return (
    <PageNav label={roomsLabel}>
      <HomeHeader />
      <NavSearchField onOpen={() => navigate(getHomeSearchPath())} selected={searchSelected} />
      {noRoomToDisplay ? (
        roomListLoaded ? (
          <HomeEmpty />
        ) : (
          <NavLoading label={t('modernization.nav.loading_rooms', 'Loading rooms…')} />
        )
      ) : (
        <PageNavContent scrollRef={scrollRef}>
          <Box direction="Column" gap="300">
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
                      linkPath={getHomeRoomPath(getCanonicalAliasOrRoomId(roomId))}
                      notificationMode={getRoomNotificationMode(
                        notificationPreferences,
                        room.roomId
                      )}
                    />
                  );
                })}
              </NavCategory>
            )}

            <NavCategory>
              <NavSectionHeader
                label={roomsLabel}
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
          </Box>
        </PageNavContent>
      )}
    </PageNav>
  );
}
