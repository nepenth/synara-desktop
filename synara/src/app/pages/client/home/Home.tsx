import React, { MouseEventHandler, forwardRef, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Avatar,
  Box,
  Button,
  Icon,
  IconButton,
  Icons,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Text,
  config,
  toRem,
} from 'folds';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useAtom, useAtomValue } from 'jotai';
import FocusTrap from 'focus-trap-react';
import {
  NavButton,
  NavCategory,
  NavCategoryHeader,
  NavEmptyCenter,
  NavEmptyLayout,
  NavItem,
  NavItemContent,
  NavLink,
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
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { useNativeRoomListSnapshot } from '../../../state/room-list/roomList';
import { VirtualTile } from '../../../components/virtualizer';
import { RoomNavCategoryButton, RoomNavItem } from '../../../features/room-nav';
import { makeNavCategoryId } from '../../../state/closedNavCategories';
import { roomToUnreadAtom } from '../../../state/room/roomToUnread';
import { useCategoryHandler } from '../../../hooks/useCategoryHandler';
import { useNavToActivePathMapper } from '../../../hooks/useNavToActivePathMapper';
import { PageNav, PageNavHeader, PageNavContent } from '../../../components/page';
import { useRoomsUnread } from '../../../state/hooks/unread';
import { markAsReadInBackground } from '../../../utils/notifications';
import { useClosedNavCategoriesAtom } from '../../../state/hooks/closedNavCategories';
import { stopPropagation } from '../../../utils/keyboard';
import { useSetting } from '../../../state/hooks/settings';
import { settingsAtom } from '../../../state/settings';
import SynaraPNG from '../../../../../public/res/png/synara.png';
import {
  getRoomNotificationMode,
  useRoomsNotificationPreferencesContext,
} from '../../../hooks/useRoomsNotificationPreferences';
import * as css from './Home.css';

type HomeMenuProps = {
  requestClose: () => void;
};
const HomeMenu = forwardRef<HTMLDivElement, HomeMenuProps>(({ requestClose }, ref) => {
  const orphanRooms = useHomeRooms();
  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');
  const unread = useRoomsUnread(orphanRooms, roomToUnreadAtom);
  const mx = useMatrixClient();

  const handleMarkAsRead = () => {
    if (!unread) return;
    orphanRooms.forEach((rId) => markAsReadInBackground(mx, rId, hideActivity));
    requestClose();
  };

  return (
    <Menu ref={ref} style={{ maxWidth: toRem(160), width: '100vw' }}>
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        <MenuItem
          onClick={handleMarkAsRead}
          size="300"
          after={<Icon size="100" src={Icons.CheckTwice} />}
          radii="300"
          aria-disabled={!unread}
        >
          <Text style={{ flexGrow: 1 }} as="span" size="T300" truncate>
            Mark as Read
          </Text>
        </MenuItem>
      </Box>
    </Menu>
  );
});

function RoomListSortIcons({
  sectionLabel,
  roomSort,
  onRoomSort,
}: {
  sectionLabel: string;
  roomSort: RoomListSort;
  onRoomSort: (sort: RoomListSort) => void;
}) {
  const recentLabel = `Sort ${sectionLabel} by recent activity`;
  const nameLabel = `Sort ${sectionLabel} by name`;
  return (
    <Box shrink="No" alignItems="Center" gap="0">
      <IconButton
        className={css.SortIconButton}
        size="300"
        variant="Surface"
        radii="300"
        fill={roomSort === 'recent' ? 'Soft' : 'None'}
        aria-pressed={roomSort === 'recent'}
        aria-label={recentLabel}
        title={recentLabel}
        onClick={() => onRoomSort('recent')}
      >
        <Icon
          className={css.SortIcon}
          size="50"
          src={Icons.RecentClock}
          filled={roomSort === 'recent'}
        />
      </IconButton>
      <IconButton
        className={css.SortIconButton}
        size="300"
        variant="Surface"
        radii="300"
        fill={roomSort === 'name' ? 'Soft' : 'None'}
        aria-pressed={roomSort === 'name'}
        aria-label={nameLabel}
        title={nameLabel}
        onClick={() => onRoomSort('name')}
      >
        <Icon className={css.SortIcon} size="50" src={Icons.Alphabet} filled={roomSort === 'name'} />
      </IconButton>
    </Box>
  );
}

function HomeHeader() {
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();

  const handleOpenMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    const cords = evt.currentTarget.getBoundingClientRect();
    setMenuAnchor((currentState) => {
      if (currentState) return undefined;
      return cords;
    });
  };

  return (
    <>
      <PageNavHeader>
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
              Home
            </Text>
          </Box>
          <Box>
            <IconButton aria-pressed={!!menuAnchor} variant="Surface" onClick={handleOpenMenu}>
              <Icon src={Icons.VerticalDots} size="200" />
            </IconButton>
          </Box>
        </Box>
      </PageNavHeader>
      <PopOut
        anchor={menuAnchor}
        position="Bottom"
        align="End"
        offset={6}
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
            <HomeMenu requestClose={() => setMenuAnchor(undefined)} />
          </FocusTrap>
        }
      />
    </>
  );
}

function HomeEmpty() {
  const navigate = useNavigate();

  return (
    <NavEmptyCenter>
      <NavEmptyLayout
        icon={<Icon size="600" src={Icons.Hash} />}
        title={
          <Text size="H5" align="Center">
            No Rooms
          </Text>
        }
        content={
          <Text size="T300" align="Center">
            You do not have any rooms yet.
          </Text>
        }
        options={
          <>
            <Button onClick={() => navigate(getHomeCreatePath())} variant="Secondary" size="300">
              <Text size="B300" truncate>
                Create Room
              </Text>
            </Button>
            <Button
              onClick={() => navigate(getExplorePath())}
              variant="Secondary"
              fill="Soft"
              size="300"
            >
              <Text size="B300" truncate>
                Explore Community Rooms
              </Text>
            </Button>
          </>
        }
      />
    </NavEmptyCenter>
  );
}

const DEFAULT_CATEGORY_ID = makeNavCategoryId('home', 'room');

export function Home() {
  const mx = useMatrixClient();
  useNavToActivePathMapper('home');
  const scrollRef = useRef<HTMLDivElement>(null);
  const rooms = useHomeRooms();
  const notificationPreferences = useRoomsNotificationPreferencesContext();
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const navigate = useNavigate();

  const selectedRoomId = useSelectedRoom();
  const createRoomSelected = useHomeCreateSelected();
  const joinSelected = useHomeJoinSelected();
  const searchSelected = useHomeSearchSelected();
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
    () => favoriteRoomIdSet(nativeRoomList.rooms),
    [nativeRoomList.rooms]
  );
  const { favoriteRoomIds, remainingRoomIds } = useMemo(
    () => partitionHomeRooms(rooms, favoriteIds),
    [rooms, favoriteIds]
  );

  const sortedFavoriteRoomIds = useMemo(
    () => sortHomeRoomIds(favoriteRoomIds, nativeRoomList.rooms, favoriteSort),
    [favoriteRoomIds, nativeRoomList.rooms, favoriteSort]
  );

  const mainRoomIds = useMemo(() => {
    const items = sortHomeRoomIds(remainingRoomIds, nativeRoomList.rooms, roomsSort);
    if (closedCategories.has(DEFAULT_CATEGORY_ID)) {
      return items.filter((rId) => roomToUnread.has(rId) || rId === selectedRoomId);
    }
    return items;
  }, [
    remainingRoomIds,
    nativeRoomList.rooms,
    roomsSort,
    closedCategories,
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

  const virtualizer = useVirtualizer({
    count: mainRoomIds.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 40,
    overscan: 10,
  });

  const handleCategoryClick = useCategoryHandler(setClosedCategories, (categoryId) =>
    closedCategories.has(categoryId)
  );

  return (
    <PageNav>
      <HomeHeader />
      {noRoomToDisplay ? (
        <HomeEmpty />
      ) : (
        <PageNavContent scrollRef={scrollRef}>
          <Box direction="Column" gap="300">
            <NavCategory>
              <NavItem variant="Surface" radii="400" aria-selected={createRoomSelected}>
                <NavButton onClick={() => navigate(getHomeCreatePath())}>
                  <NavItemContent>
                    <Box as="span" grow="Yes" alignItems="Center" gap="200">
                      <Avatar size="200" radii="400">
                        <Icon src={Icons.Plus} size="100" />
                      </Avatar>
                      <Box as="span" grow="Yes">
                        <Text as="span" size="Inherit" truncate>
                          Create Room
                        </Text>
                      </Box>
                    </Box>
                  </NavItemContent>
                </NavButton>
              </NavItem>
              <NavItem variant="Surface" radii="400" aria-selected={joinSelected}>
                <NavButton onClick={() => navigate(getHomeJoinPath())}>
                  <NavItemContent>
                    <Box as="span" grow="Yes" alignItems="Center" gap="200">
                      <Avatar size="200" radii="400">
                        <Icon src={Icons.Link} size="100" />
                      </Avatar>
                      <Box as="span" grow="Yes">
                        <Text as="span" size="Inherit" truncate>
                          Join with Address
                        </Text>
                      </Box>
                    </Box>
                  </NavItemContent>
                </NavButton>
              </NavItem>
              <NavItem variant="Surface" radii="400" aria-selected={searchSelected}>
                <NavLink to={getHomeSearchPath()}>
                  <NavItemContent>
                    <Box as="span" grow="Yes" alignItems="Center" gap="200">
                      <Avatar size="200" radii="400">
                        <Icon src={Icons.Search} size="100" filled={searchSelected} />
                      </Avatar>
                      <Box as="span" grow="Yes">
                        <Text as="span" size="Inherit" truncate>
                          Message Search
                        </Text>
                      </Box>
                    </Box>
                  </NavItemContent>
                </NavLink>
              </NavItem>
            </NavCategory>

            {sortedFavoriteRoomIds.length > 0 && (
              <NavCategory>
                <NavCategoryHeader>
                  <Box grow="Yes" alignItems="Center" gap="200">
                    <Text size="B300">Favorites</Text>
                    <Box grow="Yes" />
                    <RoomListSortIcons
                      sectionLabel="Favorites"
                      roomSort={favoriteSort}
                      onRoomSort={handleFavoriteSort}
                    />
                  </Box>
                </NavCategoryHeader>
                {sortedFavoriteRoomIds.map((roomId) => {
                  const room = mx.getRoom(roomId);
                  if (!room) return null;
                  const selected = selectedRoomId === roomId;
                  return (
                    <RoomNavItem
                      key={roomId}
                      room={room}
                      selected={selected}
                      showAvatar
                      linkPath={getHomeRoomPath(getCanonicalAliasOrRoomId(mx, roomId))}
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
              <NavCategoryHeader>
                <Box grow="Yes" alignItems="Center" gap="200">
                  <RoomNavCategoryButton
                    closed={closedCategories.has(DEFAULT_CATEGORY_ID)}
                    data-category-id={DEFAULT_CATEGORY_ID}
                    onClick={handleCategoryClick}
                  >
                    Rooms
                  </RoomNavCategoryButton>
                  <Box grow="Yes" />
                  <RoomListSortIcons
                    sectionLabel="Rooms"
                    roomSort={roomsSort}
                    onRoomSort={handleRoomsSort}
                  />
                </Box>
              </NavCategoryHeader>
              <div
                style={{
                  position: 'relative',
                  height: virtualizer.getTotalSize(),
                }}
              >
                {virtualizer.getVirtualItems().map((vItem) => {
                  const roomId = mainRoomIds[vItem.index];
                  const room = mx.getRoom(roomId);
                  if (!room) return null;
                  const selected = selectedRoomId === roomId;

                  return (
                    <VirtualTile
                      virtualItem={vItem}
                      key={vItem.index}
                      ref={virtualizer.measureElement}
                    >
                      <RoomNavItem
                        room={room}
                        selected={selected}
                        showAvatar
                        linkPath={getHomeRoomPath(getCanonicalAliasOrRoomId(mx, roomId))}
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
