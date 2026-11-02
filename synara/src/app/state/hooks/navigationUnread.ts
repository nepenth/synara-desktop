import { useMemo } from 'react';
import { useAtomValue } from 'jotai';
import type { RoomToParents, Unread } from '../../../types/matrix/room';
import type { RoomSummary } from '../../features/matrix-dto/room';
import type { RoomListPresentation } from '../../features/matrix-dto/generated';
import { useNativeRoomListSnapshot } from '../room-list/roomList';
import { roomToParentsAtom } from '../room/roomToParents';
import { unreadInfosFromPresentation } from '../room/roomToUnread';

/**
 * `home`: rooms outside any space (the Home pane's Rooms and their route).
 * `direct`: DMs (the Direct messages section and `/direct/` routes).
 * `merged`: both, the one Home list (NAV-12) and its rail badge, so one
 * attention count is not split between two tabs.
 */
export type NavigationScope = 'home' | 'direct' | 'merged';

type NativeNavigationScope = {
  roomIds: string[];
  unread: Unread | undefined;
  /**
   * Red-count attention (Slack): mentions in rooms plus every unread DM
   * message. Plain unread in a room adds nothing here; it shows as a dot.
   */
  attention: number;
};

const inScope = (
  room: RoomSummary,
  roomToParents: RoomToParents,
  scope: NavigationScope
): boolean => {
  const homeRoom = !room.isDirect && !roomToParents.has(room.roomId);
  if (scope === 'direct') return room.isDirect;
  if (scope === 'home') return homeRoom;
  return room.isDirect || homeRoom;
};

/** Keep the rail scope and counts on the same native revision as room tiles. */
export const nativeNavigationScope = (
  rooms: readonly RoomSummary[],
  presentation: RoomListPresentation,
  roomToParents: RoomToParents,
  scope: NavigationScope
): NativeNavigationScope => {
  const scopedRooms = rooms.filter(
    (room) => room.membership === 'join' && !room.isSpace && inScope(room, roomToParents, scope)
  );
  const directIds = new Set(scopedRooms.filter((room) => room.isDirect).map((r) => r.roomId));
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
  const attention = unreadInfos.reduce(
    (sum, info) =>
      sum + (directIds.has(info.roomId) ? Math.max(info.total, info.highlight) : info.highlight),
    0
  );
  return { roomIds: scopedRooms.map((room) => room.roomId), unread, attention };
};

export const useNativeNavigationScope = (scope: NavigationScope): NativeNavigationScope => {
  const snapshot = useNativeRoomListSnapshot();
  const roomToParents = useAtomValue(roomToParentsAtom);
  return useMemo(
    () => nativeNavigationScope(snapshot.rooms, snapshot.presentation, roomToParents, scope),
    [snapshot.rooms, snapshot.presentation, roomToParents, scope]
  );
};
