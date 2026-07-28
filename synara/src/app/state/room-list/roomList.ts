import { atom, useSetAtom } from 'jotai';
import { ClientEvent, MatrixClient, Room, RoomEvent } from 'matrix-js-sdk';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { RoomsAction } from './utils';

const baseRoomsAtom = atom<string[]>([]);
export const allRoomsAtom = atom<string[], [RoomsAction], undefined>(
  (get) => get(baseRoomsAtom),
  (get, set, action) => {
    if (action.type === 'INITIALIZE') {
      set(baseRoomsAtom, action.rooms);
      return;
    }
    set(baseRoomsAtom, (ids) => {
      const newIds = ids.filter((id) => id !== action.roomId);
      if (action.type === 'PUT') newIds.push(action.roomId);
      return newIds;
    });
  },
);
export const useBindAllRoomsAtom = (mx: MatrixClient, allRooms: typeof allRoomsAtom) => {
  const setRooms = useSetAtom(allRooms);

  useEffect(() => {
    let disposed = false;
    let nativePollInFlight = false;
    let pollId: number | undefined;
    let unbindJsRooms: (() => void) | undefined;

    const bindJsRooms = () => {
      const isJoined = (room: Room) => room.getMyMembership() === 'join';
      setRooms({
        type: 'INITIALIZE',
        rooms: mx
          .getRooms()
          .filter(isJoined)
          .map((room) => room.roomId),
      });

      const handleRoom = (room: Room) => {
        setRooms({ type: isJoined(room) ? 'PUT' : 'DELETE', roomId: room.roomId });
      };
      const handleDeleteRoom = (roomId: string) => {
        setRooms({ type: 'DELETE', roomId });
      };
      mx.on(ClientEvent.Room, handleRoom);
      mx.on(RoomEvent.MyMembership, handleRoom);
      mx.on(ClientEvent.DeleteRoom, handleDeleteRoom);
      unbindJsRooms = () => {
        mx.removeListener(ClientEvent.Room, handleRoom);
        mx.removeListener(RoomEvent.MyMembership, handleRoom);
        mx.removeListener(ClientEvent.DeleteRoom, handleDeleteRoom);
      };
    };

    const pollNativeRooms = async () => {
      if (nativePollInFlight) return;
      nativePollInFlight = true;
      try {
        const result = await invokeDesktopWithAvailability<NativeRoomListSnapshot>(
          'matrix_room_list_snapshot',
        );
        if (disposed || !result.available || !result.value) return;
        setRooms({
          type: 'INITIALIZE',
          rooms: result.value.orderedRoomIds.filter(
            (roomId): roomId is string => typeof roomId === 'string',
          ),
        });
      } finally {
        nativePollInFlight = false;
      }
    };

    const selectRoomListOwner = async () => {
      if (!isSynaraDesktop()) {
        bindJsRooms();
        return;
      }

      const session = await invokeDesktopWithAvailability<NativeSessionSnapshot>(
        'matrix_session_snapshot',
      ).catch(() => undefined);
      if (disposed) return;
      if (!session?.available) {
        bindJsRooms();
        return;
      }
      if (session.value?.status === 'logged_in') {
        void pollNativeRooms().catch(() => undefined);
        pollId = window.setInterval(() => void pollNativeRooms().catch(() => undefined), 1000);
        return;
      }

      bindJsRooms();
    };

    void selectRoomListOwner();
    return () => {
      disposed = true;
      if (pollId !== undefined) window.clearInterval(pollId);
      unbindJsRooms?.();
    };
  }, [allRooms, mx, setRooms]);
};

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};

type NativeRoomListSnapshot = {
  sessionGeneration: number;
  orderedRoomIds: string[];
  rooms: unknown[];
};
