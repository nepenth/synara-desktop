import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import type { RoomListPresentation } from '../../features/matrix-dto/generated';
import { attentionToUnread, roomAttentionRows } from '../room-list/roomListPresentation';
import { RoomToUnread, UnreadInfo, Unread } from '../../../types/matrix/room';
import { useNativeRoomListSnapshot } from '../room-list/roomList';

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

/** Core's per-room attention rows (no space rollups) as unread infos. */
export const unreadInfosFromPresentation = (
  presentation: RoomListPresentation,
  roomIds?: ReadonlySet<string>
): UnreadInfo[] =>
  roomAttentionRows(presentation, roomIds).map((row) => ({
    roomId: row.roomId,
    highlight: row.highlight,
    total: row.total,
  }));

const baseRoomToUnread = atom<RoomToUnread>(new Map());

/** Room and space unread from Core's presentation; written only by the binder below. */
export const roomToUnreadAtom = atom<RoomToUnread>((get) => get(baseRoomToUnread));

/** Map Core's attention rows, space rollups included, to the unread atom shape. */
export const roomToUnreadFromPresentation = (presentation: RoomListPresentation): RoomToUnread =>
  new Map(presentation.unread.map((row) => [row.roomId, attentionToUnread(row)]));

/**
 * Drive list/nav/platform unread badges from the native room-list snapshot.
 * Core computes per-room attention and the space parent rollup.
 */
export const useBindRoomToUnreadAtom = () => {
  const setRoomToUnread = useSetAtom(baseRoomToUnread);
  const snapshot = useNativeRoomListSnapshot();

  useEffect(() => {
    setRoomToUnread(roomToUnreadFromPresentation(snapshot.presentation));
  }, [setRoomToUnread, snapshot.presentation]);
};
