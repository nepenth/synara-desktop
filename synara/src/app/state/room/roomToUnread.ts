import produce from 'immer';
import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import type { RoomSummary } from '../../features/matrix-dto/room';
import { RoomToUnread, UnreadInfo, Unread } from '../../../types/matrix/room';
import { getAllParents } from '../../utils/room';
import { useNativeRoomListSnapshot } from '../room-list/roomList';
import { roomToParentsAtom } from './roomToParents';

export type RoomToUnreadAction =
  | {
      type: 'RESET';
      unreadInfos: UnreadInfo[];
    }
  | {
      type: 'PUT';
      unreadInfo: UnreadInfo;
    }
  | {
      type: 'DELETE';
      roomId: string;
    };

export const unreadInfoToUnread = (unreadInfo: UnreadInfo): Unread => ({
  highlight: unreadInfo.highlight,
  total: unreadInfo.total,
  from: null,
});

const putUnreadInfo = (
  roomToUnread: RoomToUnread,
  allParents: Set<string>,
  unreadInfo: UnreadInfo
) => {
  const oldUnread = roomToUnread.get(unreadInfo.roomId) ?? { highlight: 0, total: 0, from: null };
  roomToUnread.set(unreadInfo.roomId, unreadInfoToUnread(unreadInfo));

  const newH = unreadInfo.highlight - oldUnread.highlight;
  const newT = unreadInfo.total - oldUnread.total;

  allParents.forEach((parentId) => {
    const oldParentUnread = roomToUnread.get(parentId) ?? { highlight: 0, total: 0, from: null };
    roomToUnread.set(parentId, {
      highlight: (oldParentUnread.highlight += newH),
      total: (oldParentUnread.total += newT),
      from: new Set([...(oldParentUnread.from ?? []), unreadInfo.roomId]),
    });
  });
};

const deleteUnreadInfo = (roomToUnread: RoomToUnread, allParents: Set<string>, roomId: string) => {
  const oldUnread = roomToUnread.get(roomId);
  if (!oldUnread) return;
  roomToUnread.delete(roomId);

  allParents.forEach((parentId) => {
    const oldParentUnread = roomToUnread.get(parentId);
    if (!oldParentUnread) return;
    const newFrom = new Set([...(oldParentUnread.from ?? roomId)]);
    newFrom.delete(roomId);
    if (newFrom.size === 0) {
      roomToUnread.delete(parentId);
      return;
    }
    roomToUnread.set(parentId, {
      highlight: oldParentUnread.highlight - oldUnread.highlight,
      total: oldParentUnread.total - oldUnread.total,
      from: newFrom,
    });
  });
};

export const unreadEqual = (u1: Unread, u2: Unread): boolean => {
  const countEqual = u1.highlight === u2.highlight && u1.total === u2.total;

  if (!countEqual) return false;

  const f1 = u1.from;
  const f2 = u2.from;
  if (f1 === null && f2 === null) return true;
  if (f1 === null || f2 === null) return false;

  if (f1.size !== f2.size) return false;

  let fromEqual = true;
  f1?.forEach((item) => {
    if (!f2?.has(item)) {
      fromEqual = false;
    }
  });

  return fromEqual;
};

/** Project native room-list summaries into the retained unread-info shape. */
export const unreadInfosFromNativeRooms = (rooms: readonly RoomSummary[]): UnreadInfo[] => {
  const unreadInfos: UnreadInfo[] = [];
  for (const room of rooms) {
    if (room.membership !== 'join' || room.isSpace) continue;
    if (room.notificationMode === 'mute') continue;
    if (!(room.markedUnread || room.unreadCount > 0 || room.highlightCount > 0)) continue;
    unreadInfos.push({
      roomId: room.roomId,
      highlight: room.highlightCount,
      total: room.highlightCount > room.unreadCount ? room.highlightCount : room.unreadCount,
    });
  }
  return unreadInfos;
};

/** Nav-item unread from the native room-list snapshot, not a JS-sdk receipt cache. */
export const unreadFromNativeRoom = (room: RoomSummary | undefined): Unread | undefined => {
  if (!room) return undefined;
  const [info] = unreadInfosFromNativeRooms([room]);
  return info ? unreadInfoToUnread(info) : undefined;
};

const baseRoomToUnread = atom<RoomToUnread>(new Map());
export const roomToUnreadAtom = atom<RoomToUnread, [RoomToUnreadAction], undefined>(
  (get) => get(baseRoomToUnread),
  (get, set, action) => {
    if (action.type === 'RESET') {
      const draftRoomToUnread: RoomToUnread = new Map();
      action.unreadInfos.forEach((unreadInfo) => {
        putUnreadInfo(
          draftRoomToUnread,
          getAllParents(get(roomToParentsAtom), unreadInfo.roomId),
          unreadInfo
        );
      });
      set(baseRoomToUnread, draftRoomToUnread);
      return;
    }
    if (action.type === 'PUT') {
      const { unreadInfo } = action;
      const currentUnread = get(baseRoomToUnread).get(unreadInfo.roomId);
      if (currentUnread && unreadEqual(currentUnread, unreadInfoToUnread(unreadInfo))) {
        // Do not update if unread data has not changes
        // like total & highlight
        return;
      }
      set(
        baseRoomToUnread,
        produce(get(baseRoomToUnread), (draftRoomToUnread) =>
          putUnreadInfo(
            draftRoomToUnread,
            getAllParents(get(roomToParentsAtom), unreadInfo.roomId),
            unreadInfo
          )
        )
      );
      return;
    }
    if (action.type === 'DELETE' && get(baseRoomToUnread).has(action.roomId)) {
      set(
        baseRoomToUnread,
        produce(get(baseRoomToUnread), (draftRoomToUnread) =>
          deleteUnreadInfo(
            draftRoomToUnread,
            getAllParents(get(roomToParentsAtom), action.roomId),
            action.roomId
          )
        )
      );
    }
  }
);

/**
 * Drive list/nav/platform unread badges from the native room-list snapshot.
 * Space parent rollup reads `roomToParentsAtom` (native V-ROOMS.2a owner).
 */
export const useBindRoomToUnreadAtom = () => {
  const setUnreadAtom = useSetAtom(roomToUnreadAtom);
  const snapshot = useNativeRoomListSnapshot();
  const roomToParents = useAtomValue(roomToParentsAtom);

  useEffect(() => {
    setUnreadAtom({
      type: 'RESET',
      unreadInfos: unreadInfosFromNativeRooms(snapshot.rooms),
    });
  }, [roomToParents, setUnreadAtom, snapshot.rooms]);
};
