import { useAtomValue } from 'jotai';
import { useCallback, useMemo } from 'react';
import { allRoomsAtom } from '../state/room-list/roomList';
import { useMatrixClient } from './useMatrixClient';
import { EventedRoomReading } from '../utils/roomEvents';

export const useAllJoinedRoomsSet = () => {
  const allRooms = useAtomValue(allRoomsAtom);
  const allJoinedRooms = useMemo(() => new Set(allRooms), [allRooms]);

  return allJoinedRooms;
};

export type GetRoomCallback = (roomId: string) => EventedRoomReading | undefined;
export const useGetRoom = (rooms: Set<string>): GetRoomCallback => {
  const mx = useMatrixClient();

  const getRoom: GetRoomCallback = useCallback(
    (rId: string) => {
      if (rooms.has(rId)) {
        return (mx.getRoom(rId) ?? undefined) as EventedRoomReading | undefined;
      }
      return undefined;
    },
    [mx, rooms]
  );

  return getRoom;
};
