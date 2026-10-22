import { createContext, useContext } from 'react';
import { EventedRoomReading } from '../utils/roomEvents';

const RoomContext = createContext<EventedRoomReading | null>(null);

export const RoomProvider = RoomContext.Provider;

export function useRoom(): EventedRoomReading {
  const room = useContext(RoomContext);
  if (!room) throw new Error('Room not provided!');
  return room;
}

/** The room when one is provided; null outside a room view (e.g. a test harness). */
export function useOptionalRoom(): EventedRoomReading | null {
  return useContext(RoomContext);
}

const IsDirectRoomContext = createContext<boolean>(false);

export const IsDirectRoomProvider = IsDirectRoomContext.Provider;

export const useIsDirectRoom = () => {
  const direct = useContext(IsDirectRoomContext);

  return direct;
};
