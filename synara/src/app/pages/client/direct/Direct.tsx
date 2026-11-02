import React, {
  MouseEventHandler,
  forwardRef,
  useCallback,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useAtom, useAtomValue } from 'jotai';
import {
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
import { useTranslation } from 'react-i18next';
import { useSharedScrollVirtualizer } from '../../../hooks/useSharedScrollVirtualizer';
import FocusTrap from '../../../components/FocusTrap';
import { useNavigate } from 'react-router-dom';
import {
  NavCategory,
  NavEmptyCenter,
  NavEmptyLayout,
  NavLoading,
  NavSectionHeader,
} from '../../../components/nav';
import {
  useNativeRoomListLoaded,
  useNativeRoomListSnapshot,
} from '../../../state/room-list/roomList';
import { getDirectCreatePath, getDirectRoomPath } from '../../pathUtils';
import { getCanonicalAliasOrRoomId } from '../../../utils/matrix';
import { useSelectedRoom } from '../../../hooks/router/useSelectedRoom';
import { VirtualTile } from '../../../components/virtualizer';
import { RoomListSortMenu, RoomNavItem } from '../../../features/room-nav';
import { makeNavCategoryId } from '../../../state/closedNavCategories';
import { roomToUnreadAtom } from '../../../state/room/roomToUnread';
import { useCategoryHandler } from '../../../hooks/useCategoryHandler';
import { useNavToActivePathMapper } from '../../../hooks/useNavToActivePathMapper';
import { useDirectRooms } from './useDirectRooms';
import { PageNav, PageNavContent, PageNavHeader } from '../../../components/page';
import { useClosedNavCategoriesAtom } from '../../../state/hooks/closedNavCategories';
import { useNativeNavigationScope } from '../../../state/hooks/navigationUnread';
import { useMarkRoomsAsRead } from '../../../hooks/useMarkRoomsAsRead';
import { stopPropagation } from '../../../utils/keyboard';
import {
  getRoomNotificationMode,
  useRoomsNotificationPreferencesContext,
} from '../../../hooks/useRoomsNotificationPreferences';
import { useDirectCreateSelected } from '../../../hooks/router/useDirectSelected';
import {
  readRoomListSort,
  sortHomeRoomIds,
  writeRoomListSort,
  type RoomListSort,
} from '../home/homeRoomList';
import { useRoomListSortOptions } from '../home/Home';
import * as depthCss from '../../../styles/Depth.css';

import { getNativeRoom } from '../../../native/nativeSession';
type DirectMenuProps = {
  requestClose: () => void;
};
const DirectMenu = forwardRef<HTMLDivElement, DirectMenuProps>(({ requestClose }, ref) => {
  const { t } = useTranslation();
  const { roomIds: orphanRooms, unread } = useNativeNavigationScope('direct');

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

function DirectHeader() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const createDirectSelected = useDirectCreateSelected();
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
      <PageNavHeader data-tauri-drag-region>
        <Box alignItems="Center" grow="Yes" gap="300">
          <Box grow="Yes">
            <Text size="H4" truncate>
              {t('modernization.nav.direct_messages', 'Direct messages')}
            </Text>
          </Box>
          <Box shrink="No" gap="100">
            <IconButton
              className={depthCss.quietInteractiveSurface}
              aria-label={t('modernization.nav.new_message', 'New message')}
              aria-pressed={createDirectSelected}
              variant="Surface"
              onClick={() => navigate(getDirectCreatePath())}
            >
              <Icon src={Icons.Plus} size="200" />
            </IconButton>
            <IconButton
              aria-label={t('modernization.nav.direct_messages_options', 'Direct messages options')}
              aria-pressed={!!menuAnchor}
              variant="Surface"
              onClick={handleOpenMenu}
            >
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
              onDeactivate: () => setMenuAnchor(undefined),
              clickOutsideDeactivates: true,
              isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
              isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
              escapeDeactivates: stopPropagation,
            }}
          >
            <DirectMenu requestClose={() => setMenuAnchor(undefined)} />
          </FocusTrap>
        }
      />
    </>
  );
}

function DirectEmpty() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <NavEmptyCenter>
      <NavEmptyLayout
        icon={<Icon size="600" src={Icons.Mention} />}
        title={
          <Text size="H5" align="Center">
            {t('modernization.nav.no_direct_messages_title', 'No direct messages')}
          </Text>
        }
        content={
          <Text size="T300" align="Center">
            {t(
              'modernization.nav.no_direct_messages_description',
              'You do not have any direct messages yet.'
            )}
          </Text>
        }
        options={
          <Button variant="Secondary" size="300" onClick={() => navigate(getDirectCreatePath())}>
            <Text size="B300" truncate>
              {t('modernization.nav.new_message', 'New message')}
            </Text>
          </Button>
        }
      />
    </NavEmptyCenter>
  );
}

const DEFAULT_CATEGORY_ID = makeNavCategoryId('direct', 'direct');
export function Direct() {
  useNavToActivePathMapper('direct');
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const directs = useDirectRooms();
  const notificationPreferences = useRoomsNotificationPreferencesContext();
  const roomToUnread = useAtomValue(roomToUnreadAtom);
  const nativeRoomList = useNativeRoomListSnapshot();
  const sortOptions = useRoomListSortOptions();

  const selectedRoomId = useSelectedRoom();
  const roomListLoaded = useNativeRoomListLoaded();
  const noRoomToDisplay = directs.length === 0;
  const [closedCategories, setClosedCategories] = useAtom(useClosedNavCategoriesAtom());

  // Direct messages share Core's presentation order with Home (NAV-11).
  const sortStorage = typeof localStorage === 'undefined' ? undefined : localStorage;
  const [directSort, setDirectSort] = useState<RoomListSort>(() =>
    readRoomListSort(sortStorage, 'directs')
  );
  const handleDirectSort = (sort: RoomListSort) => {
    setDirectSort(sort);
    writeRoomListSort(sortStorage, sort, 'directs');
  };

  const closed = closedCategories.has(DEFAULT_CATEGORY_ID);
  const sortedDirects = useMemo(() => {
    const items = sortHomeRoomIds(directs, nativeRoomList.presentation, directSort);
    if (closed) {
      return items.filter((rId) => roomToUnread.has(rId) || rId === selectedRoomId);
    }
    return items;
  }, [directs, nativeRoomList.presentation, directSort, closed, roomToUnread, selectedRoomId]);

  const getRoomKey = useCallback((index: number) => sortedDirects[index], [sortedDirects]);
  const { virtualizer, listRef, scrollMargin } = useSharedScrollVirtualizer(
    scrollRef,
    sortedDirects.length,
    getRoomKey,
    10,
    // Room rows (`RoomSurface`) are 32px.
    32
  );

  const handleCategoryClick = useCategoryHandler(setClosedCategories, (categoryId) =>
    closedCategories.has(categoryId)
  );

  const label = t('modernization.nav.direct_messages', 'Direct messages');

  return (
    <PageNav label={label}>
      <DirectHeader />
      {noRoomToDisplay ? (
        roomListLoaded ? (
          <DirectEmpty />
        ) : (
          <NavLoading
            label={t('modernization.nav.loading_direct_messages', 'Loading direct messages…')}
          />
        )
      ) : (
        <PageNavContent scrollRef={scrollRef}>
          <Box direction="Column" gap="300">
            <NavCategory>
              <NavSectionHeader
                label={label}
                closed={closed}
                categoryId={DEFAULT_CATEGORY_ID}
                onToggle={handleCategoryClick}
                actions={
                  <RoomListSortMenu
                    sectionLabel={label}
                    value={directSort}
                    options={sortOptions}
                    onChange={handleDirectSort}
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
                  const roomId = sortedDirects[vItem.index];
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
          </Box>
        </PageNavContent>
      )}
    </PageNav>
  );
}
