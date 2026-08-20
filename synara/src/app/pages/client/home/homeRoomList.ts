import type { RoomSummary } from '../../../features/matrix-dto/room';
import { factoryRoomIdByActivity, factoryRoomIdByAtoZ } from '../../../utils/sort';
import type { MatrixClientReading } from '../../../utils/room';

export const HOME_ROOM_SORT_STORAGE_KEY = 'synara.homeRoomSort';

export type HomeRoomSort = 'name' | 'recent';

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

export const readHomeRoomSort = (storage: Pick<Storage, 'getItem'> | undefined): HomeRoomSort => {
  const value = storage?.getItem(HOME_ROOM_SORT_STORAGE_KEY);
  return value === 'recent' ? 'recent' : 'name';
};

export const writeHomeRoomSort = (
  storage: Pick<Storage, 'setItem'> | undefined,
  sort: HomeRoomSort
): void => {
  storage?.setItem(HOME_ROOM_SORT_STORAGE_KEY, sort);
};

export const sortHomeRoomIds = (
  mx: MatrixClientReading,
  roomIds: readonly string[],
  sort: HomeRoomSort
): string[] =>
  Array.from(roomIds).sort(
    sort === 'recent' ? factoryRoomIdByActivity(mx) : factoryRoomIdByAtoZ(mx)
  );
