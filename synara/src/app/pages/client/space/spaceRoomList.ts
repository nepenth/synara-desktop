import { ROOM_LIST_SORT_STORAGE_KEY } from '../home/homeRoomList';

/**
 * Space lists add "Space order" (the `m.space.child` order key, the server's
 * arrangement) to the Home sort options. One device-local preference covers
 * every space, like the Favorites and Rooms sections.
 */
export type SpaceRoomListSort = 'space' | 'recent' | 'name';

export const SPACE_ROOM_LIST_SORT_STORAGE_KEY = `${ROOM_LIST_SORT_STORAGE_KEY}.space`;

export const DEFAULT_SPACE_ROOM_LIST_SORT: SpaceRoomListSort = 'space';

const parse = (value: string | null | undefined): SpaceRoomListSort | undefined =>
  value === 'space' || value === 'recent' || value === 'name' ? value : undefined;

export const readSpaceRoomListSort = (
  storage: Pick<Storage, 'getItem'> | undefined
): SpaceRoomListSort =>
  parse(storage?.getItem(SPACE_ROOM_LIST_SORT_STORAGE_KEY)) ?? DEFAULT_SPACE_ROOM_LIST_SORT;

export const writeSpaceRoomListSort = (
  storage: Pick<Storage, 'setItem'> | undefined,
  sort: SpaceRoomListSort
): void => {
  storage?.setItem(SPACE_ROOM_LIST_SORT_STORAGE_KEY, sort);
};
