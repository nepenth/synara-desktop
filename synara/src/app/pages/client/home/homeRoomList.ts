import type { RoomSummary } from '../../../features/matrix-dto/room';

/**
 * Device-local room-list sort chrome. Same key and default as iOS
 * `UserDefaults` (`synara.roomListSort`). Not Matrix account data; favorites
 * sync via `m.favourite`, sort stays per-device.
 */
export const ROOM_LIST_SORT_STORAGE_KEY = 'synara.roomListSort';

export type RoomListSort = 'recent' | 'name';

export const DEFAULT_ROOM_LIST_SORT: RoomListSort = 'recent';

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

export const favoriteRoomIdSet = (rooms: readonly RoomSummary[]): Set<string> =>
  new Set(rooms.filter((room) => room.isFavorite).map((room) => room.roomId));

export const readRoomListSort = (storage: Pick<Storage, 'getItem'> | undefined): RoomListSort => {
  const value = storage?.getItem(ROOM_LIST_SORT_STORAGE_KEY);
  return value === 'name' ? 'name' : DEFAULT_ROOM_LIST_SORT;
};

export const writeRoomListSort = (
  storage: Pick<Storage, 'setItem'> | undefined,
  sort: RoomListSort
): void => {
  storage?.setItem(ROOM_LIST_SORT_STORAGE_KEY, sort);
};

const compareNames = (left?: string, right?: string, leftId?: string, rightId?: string): number => {
  const ln = (left ?? '').replace(/#/g, '').toLocaleLowerCase();
  const rn = (right ?? '').replace(/#/g, '').toLocaleLowerCase();
  if (ln && !rn) return -1;
  if (!ln && rn) return 1;
  const nameDelta = ln.localeCompare(rn, undefined, { sensitivity: 'base' });
  return nameDelta || (leftId ?? '').localeCompare(rightId ?? '');
};

/**
 * One global sort for Favorites and Rooms. Recent uses native
 * `lastActivityTs` only — missing timestamps sort last and are not invented.
 */
export const sortHomeRoomIds = (
  roomIds: readonly string[],
  rooms: readonly RoomSummary[],
  sort: RoomListSort
): string[] => {
  const byId = new Map(rooms.map((room) => [room.roomId, room]));
  return Array.from(roomIds).sort((leftId, rightId) => {
    const left = byId.get(leftId);
    const right = byId.get(rightId);
    if (sort === 'name') {
      return compareNames(left?.name, right?.name, leftId, rightId);
    }
    const leftTs = left?.lastActivityTs;
    const rightTs = right?.lastActivityTs;
    const leftHasTs = typeof leftTs === 'number' && Number.isFinite(leftTs);
    const rightHasTs = typeof rightTs === 'number' && Number.isFinite(rightTs);
    if (leftHasTs && rightHasTs && leftTs !== rightTs) {
      return (rightTs as number) - (leftTs as number);
    }
    if (leftHasTs && !rightHasTs) return -1;
    if (!leftHasTs && rightHasTs) return 1;
    return compareNames(left?.name, right?.name, leftId, rightId);
  });
};
