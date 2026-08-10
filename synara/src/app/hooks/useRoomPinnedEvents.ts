import { useMemo } from 'react';
import { StateEvent } from '../../types/matrix/room';
import type { EventedRoomReading } from '../utils/roomEvents';
import { useStateEvent } from './useStateEvent';

type RoomPinnedEventsEventContent = {
  pinned: string[];
};

export const useRoomPinnedEvents = (room: EventedRoomReading): string[] => {
  const pinEvent = useStateEvent(room, StateEvent.RoomPinnedEvents);
  const events = useMemo(() => {
    const content = pinEvent?.getContent<RoomPinnedEventsEventContent>();
    return content?.pinned ?? [];
  }, [pinEvent]);

  return events;
};
