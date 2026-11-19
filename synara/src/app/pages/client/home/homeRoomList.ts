import type { RoomListPresentation } from '../../../features/matrix-dto/generated';
import {
  favoriteRoomIdsFromPresentation,
  orderRoomIdsByPresentation,
} from '../../../state/room-list/roomListPresentation';
import { makeNavCategoryId } from '../../../state/closedNavCategories';
import {
  buildRoomFolderSections,
  roomIdsInFolderSections,
  visibleWhenClosed,
  type RoomListSectionModel,
} from '../../../features/room-folders/roomFolderSections';

/**
 * Device-local room-list sort chrome. Legacy key `synara.roomListSort` is the
 * fallback when a section has no preference yet. Favorites and Rooms (and iOS
 * Channels / Direct messages) each persist their own order. Not Matrix account
 * data; favorites sync via `m.favourite`, sort stays per-device.
 */
export const ROOM_LIST_SORT_STORAGE_KEY = 'synara.roomListSort';

export type RoomListSort = 'recent' | 'name';

/**
 * The default sections, each with its own stored sort. Room folders are
 * sections too (`buildHomeRoomListSections`) but follow the Rooms sort.
 */
export type RoomListSection = 'favorites' | 'rooms' | 'directs';

export const DEFAULT_ROOM_LIST_SORT: RoomListSort = 'recent';

export const roomListSortStorageKey = (section: RoomListSection): string =>
  `${ROOM_LIST_SORT_STORAGE_KEY}.${section}`;

export const partitionHomeRooms = (
  roomIds: readonly string[],
  favoriteIds: ReadonlySet<string>
): { favoriteRoomIds: string[]; remainingRoomIds: string[] } => {
  const favoriteRoomIds: string[] = [];
  const remainingRoomIds: string[] = [];
  for (const roomId of roomIds) {
    if (favoriteIds.has(roomId)) {
      favoriteRoomIds.push(roomId);
    } else {
      remainingRoomIds.push(roomId);
    }
  }
  return { favoriteRoomIds, remainingRoomIds };
};

/** Joined `m.favourite` rooms, as Core classifies them. */
export const favoriteRoomIdSet = (presentation: RoomListPresentation): Set<string> =>
  favoriteRoomIdsFromPresentation(presentation);

const parseRoomListSort = (value: string | null | undefined): RoomListSort | undefined =>
  value === 'name' || value === 'recent' ? value : undefined;

export const readRoomListSort = (
  storage: Pick<Storage, 'getItem'> | undefined,
  section?: RoomListSection
): RoomListSort => {
  if (section) {
    const scoped = parseRoomListSort(storage?.getItem(roomListSortStorageKey(section)));
    if (scoped) return scoped;
  }
  return parseRoomListSort(storage?.getItem(ROOM_LIST_SORT_STORAGE_KEY)) ?? DEFAULT_ROOM_LIST_SORT;
};

export const writeRoomListSort = (
  storage: Pick<Storage, 'setItem'> | undefined,
  sort: RoomListSort,
  section?: RoomListSection
): void => {
  storage?.setItem(section ? roomListSortStorageKey(section) : ROOM_LIST_SORT_STORAGE_KEY, sort);
};

/**
 * Sort one section's room ids by Core's order for the chosen sort. Core owns
 * name normalization and the recent-activity rules (missing timestamps last).
 */
export const sortHomeRoomIds = (
  roomIds: readonly string[],
  presentation: RoomListPresentation,
  sort: RoomListSort
): string[] => orderRoomIdsByPresentation(roomIds, presentation, sort);

/** One sort per default section; folders follow `rooms`. */
export type RoomListSorts = Record<RoomListSection, RoomListSort>;

export const readRoomListSorts = (
  storage: Pick<Storage, 'getItem'> | undefined
): RoomListSorts => ({
  favorites: readRoomListSort(storage, 'favorites'),
  rooms: readRoomListSort(storage, 'rooms'),
  directs: readRoomListSort(storage, 'directs'),
});

/** Collapse ids. Direct messages keeps the former Direct pane's id, so its state carries over. */
export const HOME_SECTION_CATEGORY_IDS: Record<RoomListSection, string> = {
  favorites: makeNavCategoryId('home', 'favorites'),
  rooms: makeNavCategoryId('home', 'room'),
  directs: makeNavCategoryId('direct', 'direct'),
};

export type HomeRoomListSectionsInput = {
  /** Home rooms (outside any space, not DMs). */
  roomIds: readonly string[];
  directIds: readonly string[];
  presentation: RoomListPresentation;
  sorts: RoomListSorts;
  closedCategories: ReadonlySet<string>;
  hasUnread: (roomId: string) => boolean;
  selectedRoomId?: string;
};

/**
 * The one Home list (NAV-12): Favorites (rooms and DMs, as on iOS), one
 * section per room folder in folder order, Rooms, then Direct messages. A
 * room in a folder leaves Rooms; a favorite stays in Favorites (Core keeps it
 * out of its folder's rooms). Every folder is listed, with no rooms too.
 */
export const buildHomeRoomListSections = ({
  roomIds,
  directIds,
  presentation,
  sorts,
  closedCategories,
  hasUnread,
  selectedRoomId,
}: HomeRoomListSectionsInput): RoomListSectionModel[] => {
  const favoriteIds = favoriteRoomIdSet(presentation);
  const roomPartition = partitionHomeRooms(roomIds, favoriteIds);
  const directPartition = partitionHomeRooms(directIds, favoriteIds);
  const homeRoomIds = new Set(roomPartition.remainingRoomIds);
  const folders = buildRoomFolderSections({
    presentation,
    scope: 'home',
    inView: (roomId) => homeRoomIds.has(roomId),
    order: sorts.rooms,
    closedCategories,
    hasUnread,
    selectedRoomId,
    keepEmpty: true,
  });
  const filed = roomIdsInFolderSections(folders);
  const section = (kind: RoomListSection, ids: readonly string[]): RoomListSectionModel[] => {
    if (ids.length === 0) return [];
    const categoryId = HOME_SECTION_CATEGORY_IDS[kind];
    const closed = closedCategories.has(categoryId);
    const sorted = sortHomeRoomIds(ids, presentation, sorts[kind]);
    return [
      {
        id: kind,
        kind,
        categoryId,
        closed,
        roomIds: sorted,
        rows: visibleWhenClosed(sorted, closed, hasUnread, selectedRoomId),
      },
    ];
  };
  return [
    ...section('favorites', [...roomPartition.favoriteRoomIds, ...directPartition.favoriteRoomIds]),
    ...folders,
    ...section(
      'rooms',
      roomPartition.remainingRoomIds.filter((roomId) => !filed.has(roomId))
    ),
    ...section('directs', directPartition.remainingRoomIds),
  ];
};
