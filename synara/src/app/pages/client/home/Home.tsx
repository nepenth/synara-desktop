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
import { markAsReadFromExplicitUserActionInBackground } from '../../../utils/notifications';
import { useClosedNavCategoriesAtom } from '../../../state/hooks/closedNavCategories';
import { stopPropagation } from '../../../utils/keyboard';
import SynaraPNG from '../../../../../public/res/png/synara.png';
import {
  getRoomNotificationMode,
  useRoomsNotificationPreferencesContext,
} from '../../../hooks/useRoomsNotificationPreferences';
import * as css from './Home.css';
import * as depthCss from '../../../styles/Depth.css';
import { getActiveSession } from '../../../state/sessionBootstrap';
import { homeserverDisplayName } from './homeIdentity';

type HomeMenuProps = {
  requestClose: () => void;
};
const HomeMenu = forwardRef<HTMLDivElement, HomeMenuProps>(({ requestClose }, ref) => {
  const orphanRooms = useHomeRooms();
  const unread = useRoomsUnread(orphanRooms, roomToUnreadAtom);
  const mx = useMatrixClient();

  const handleMarkAsRead = () => {
    if (!unread) return;
    orphanRooms.forEach((rId) => markAsReadFromExplicitUserActionInBackground(mx, rId));
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
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const activeLabel = roomSort === 'recent' ? 'Recent activity' : 'Name';
  const closeMenu = () => setMenuAnchor(undefined);
  return (
    <PopOut
      anchor={menuAnchor}
      position="Bottom"
      align="End"
      content={
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: closeMenu,
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu style={{ width: toRem(190) }}>
            <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
              <MenuItem
                size="300"
                radii="300"
                onClick={() => {
                  onRoomSort('recent');
                  closeMenu();
                }}
                after={roomSort === 'recent' ? <Icon size="100" src={Icons.Check} /> : undefined}
              >
                <Text size="T300">Recent activity</Text>
              </MenuItem>
              <MenuItem
                size="300"
                radii="300"
                onClick={() => {
                  onRoomSort('name');
                  closeMenu();
                }}
                after={roomSort === 'name' ? <Icon size="100" src={Icons.Check} /> : undefined}
              >
                <Text size="T300">Name</Text>
              </MenuItem>
            </Box>
          </Menu>
        </FocusTrap>
      }
    >
      <IconButton
        className={css.SortIconButton}
        size="300"
        variant="Surface"
        radii="300"
        fill={menuAnchor ? 'Soft' : 'None'}
        aria-expanded={!!menuAnchor}
        aria-label={`Sort ${sectionLabel}. Current: ${activeLabel}`}
        title={`Sort ${sectionLabel}. Current: ${activeLabel}`}
        onClick={(event) =>
          setMenuAnchor(menuAnchor ? undefined : event.currentTarget.getBoundingClientRect())
        }
      >
        <Icon
          className={css.SortIcon}
          size="50"
          src={roomSort === 'recent' ? Icons.RecentClock : Icons.Alphabet}
          filled
        />
      </IconButton>
    </PopOut>
  );
}

function HomeHeader() {
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const session = getActiveSession();
  const serverName = homeserverDisplayName(session?.baseUrl, session?.userId);

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
              {serverName}
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
              <NavItem
                className={depthCss.quietInteractiveSurface}
                variant="Surface"
                radii="400"
                aria-selected={createRoomSelected}
              >
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
              <NavItem
                className={depthCss.quietInteractiveSurface}
                variant="Surface"
                radii="400"
                aria-selected={joinSelected}
              >
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
              <NavItem
                className={depthCss.quietInteractiveSurface}
                variant="Surface"
                radii="400"
                aria-selected={searchSelected}
              >
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
