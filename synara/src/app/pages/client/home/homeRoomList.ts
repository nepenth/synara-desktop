import type { RoomListPresentation } from '../../../features/matrix-dto/generated';
import {
  favoriteRoomIdsFromPresentation,
  orderRoomIdsByPresentation,
} from '../../../state/room-list/roomListPresentation';

/**
 * Device-local room-list sort chrome. Legacy key `synara.roomListSort` is the
 * fallback when a section has no preference yet. Favorites and Rooms (and iOS
 * Channels / Direct messages) each persist their own order. Not Matrix account
 * data; favorites sync via `m.favourite`, sort stays per-device.
 */
export const ROOM_LIST_SORT_STORAGE_KEY = 'synara.roomListSort';

export type RoomListSort = 'recent' | 'name';

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
