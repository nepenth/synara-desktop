import { useAtomValue } from 'jotai';
import { useCallback, useMemo } from 'react';
import { allRoomsAtom } from '../state/room-list/roomList';
import { EventedRoomReading } from '../utils/roomEvents';

import { getNativeRoom } from '../native/nativeSession';
export const useAllJoinedRoomsSet = () => {
  const allRooms = useAtomValue(allRoomsAtom);
  const allJoinedRooms = useMemo(() => new Set(allRooms), [allRooms]);

  return allJoinedRooms;
};

export type GetRoomCallback = (roomId: string) => EventedRoomReading | undefined;
export const useGetRoom = (rooms: Set<string>): GetRoomCallback => {
  const getRoom: GetRoomCallback = useCallback(
    (rId: string) => {
      if (rooms.has(rId)) {
        return (getNativeRoom(rId) ?? undefined) as EventedRoomReading | undefined;
      }
      return undefined;
    },
    [rooms]
  );

  return getRoom;
};
