import { createContext, useContext } from 'react';
import { EventedRoomReading } from '../utils/roomEvents';

const RoomContext = createContext<EventedRoomReading | null>(null);

export const RoomProvider = RoomContext.Provider;

export function useRoom(): EventedRoomReading {
  const room = useContext(RoomContext);
  if (!room) throw new Error('Room not provided!');
  return room;
}

const IsDirectRoomContext = createContext<boolean>(false);

export const IsDirectRoomProvider = IsDirectRoomContext.Provider;

export const useIsDirectRoom = () => {
  const direct = useContext(IsDirectRoomContext);

  return direct;
};
