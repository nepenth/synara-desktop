import { useMemo } from 'react';
import { useAtomValue } from 'jotai';
import type { RoomToParents, Unread } from '../../../types/matrix/room';
import type { RoomSummary } from '../../features/matrix-dto/room';
import type { RoomListPresentation } from '../../features/matrix-dto/generated';
import { useNativeRoomListSnapshot } from '../room-list/roomList';
import { roomToParentsAtom } from '../room/roomToParents';
import { unreadInfosFromPresentation } from '../room/roomToUnread';

type NavigationScope = 'home' | 'direct';

type NativeNavigationScope = {
  roomIds: string[];
  unread: Unread | undefined;
};

/** Keep the rail scope and counts on the same native revision as room tiles. */
export const nativeNavigationScope = (
  rooms: readonly RoomSummary[],
  presentation: RoomListPresentation,
  roomToParents: RoomToParents,
  scope: NavigationScope
): NativeNavigationScope => {
  const scopedRooms = rooms.filter(
    (room) =>
      room.membership === 'join' &&
      !room.isSpace &&
      (scope === 'direct' ? room.isDirect : !room.isDirect && !roomToParents.has(room.roomId))
  );
  const unreadInfos = unreadInfosFromPresentation(
    presentation,
    new Set(scopedRooms.map((room) => room.roomId))
  );
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
    () => nativeNavigationScope(snapshot.rooms, snapshot.presentation, roomToParents, scope),
    [snapshot.rooms, snapshot.presentation, roomToParents, scope]
  );
};
