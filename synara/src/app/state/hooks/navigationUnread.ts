import { useMemo } from 'react';
import { useAtomValue } from 'jotai';
import type { RoomToParents, Unread } from '../../../types/matrix/room';
import type { RoomSummary } from '../../features/matrix-dto/room';
import { useNativeRoomListSnapshot } from '../room-list/roomList';
import { roomToParentsAtom } from '../room/roomToParents';
import { unreadInfosFromNativeRooms } from '../room/roomToUnread';

type NavigationScope = 'home' | 'direct';

type NativeNavigationScope = {
  roomIds: string[];
  unread: Unread | undefined;
};

/** Keep the rail scope and counts on the same native revision as room tiles. */
export const nativeNavigationScope = (
  rooms: readonly RoomSummary[],
  roomToParents: RoomToParents,
  scope: NavigationScope
): NativeNavigationScope => {
  const scopedRooms = rooms.filter(
    (room) =>
      room.membership === 'join' &&
      !room.isSpace &&
      (scope === 'direct' ? room.isDirect : !room.isDirect && !roomToParents.has(room.roomId))
  );
  const unreadInfos = unreadInfosFromNativeRooms(scopedRooms);
  const unread: Unread | undefined =
    unreadInfos.length === 0
      ? undefined
      : {
          total: unreadInfos.reduce((sum, info) => sum + info.total, 0),
          highlight: unreadInfos.reduce((sum, info) => sum + info.highlight, 0),
          from: new Set(unreadInfos.map((info) => info.roomId)),
        };
  return { roomIds: scopedRooms.map((room) => room.roomId), unread };
};

export const useNativeNavigationScope = (scope: NavigationScope): NativeNavigationScope => {
  const snapshot = useNativeRoomListSnapshot();
  const roomToParents = useAtomValue(roomToParentsAtom);
  return useMemo(
    () => nativeNavigationScope(snapshot.rooms, roomToParents, scope),
    [snapshot.rooms, roomToParents, scope]
  );
};
