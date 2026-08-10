import { useSetAtom, WritableAtom } from 'jotai';
import type { MatrixClientReading, RoomReading } from '../../utils/room';
import { ClientEvent, RoomEvent } from '../../utils/roomEvents';
import { useEffect } from 'react';
import { Membership } from '../../../types/matrix/room';

export type RoomsAction =
  | {
      type: 'INITIALIZE';
      rooms: string[];
    }
  | {
      type: 'PUT' | 'DELETE';
      roomId: string;
    };

type ClientEventedReading = MatrixClientReading & {
  on(event: string, listener: (...args: any[]) => unknown): unknown;
  removeListener(event: string, listener: (...args: any[]) => unknown): unknown;
};

export const useBindRoomsWithMembershipsAtom = (
  mx: ClientEventedReading,
  roomsAtom: WritableAtom<string[], [RoomsAction], undefined>,
  memberships: Membership[]
) => {
  const setRoomsAtom = useSetAtom(roomsAtom);

  useEffect(() => {
    const satisfyMembership = (room: RoomReading): boolean =>
      !!memberships.find((membership) => membership === room.getMyMembership());
    setRoomsAtom({
      type: 'INITIALIZE',
      rooms: mx
        .getRooms()
        .filter(satisfyMembership)
        .map((room) => room.roomId),
    });

    const handleAddRoom = (room: RoomReading) => {
      if (satisfyMembership(room)) {
        setRoomsAtom({ type: 'PUT', roomId: room.roomId });
      }
    };

    const handleMembershipChange = (room: RoomReading) => {
      if (satisfyMembership(room)) {
        setRoomsAtom({ type: 'PUT', roomId: room.roomId });
      } else {
        setRoomsAtom({ type: 'DELETE', roomId: room.roomId });
      }
    };

    const handleDeleteRoom = (roomId: string) => {
      setRoomsAtom({ type: 'DELETE', roomId });
    };

    mx.on(ClientEvent.Room, handleAddRoom);
    mx.on(RoomEvent.MyMembership, handleMembershipChange);
    mx.on(ClientEvent.DeleteRoom, handleDeleteRoom);
    return () => {
      mx.removeListener(ClientEvent.Room, handleAddRoom);
      mx.removeListener(RoomEvent.MyMembership, handleMembershipChange);
      mx.removeListener(ClientEvent.DeleteRoom, handleDeleteRoom);
    };
  }, [mx, memberships, setRoomsAtom]);
};

export const compareRoomsEqual = (a: string[], b: string[]) => {
  if (a.length !== b.length) return false;
  return a.every((roomId, roomIdIndex) => roomId === b[roomIdIndex]);
};
