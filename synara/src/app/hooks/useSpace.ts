import { createContext, useContext } from 'react';
import { EventedRoomReading } from '../utils/roomEvents';

const SpaceContext = createContext<EventedRoomReading | null>(null);

export const SpaceProvider = SpaceContext.Provider;

export function useSpace(): EventedRoomReading {
  const space = useContext(SpaceContext);
  if (!space) throw new Error('Space not provided!');
  return space;
}

export function useSpaceOptionally(): EventedRoomReading | null {
  const space = useContext(SpaceContext);
  return space;
}
