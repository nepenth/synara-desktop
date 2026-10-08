import { useMemo } from 'react';
import type { EventedRoomReading } from '../utils/roomEvents';
import { getStateEvent } from '../utils/room';
import { StateEvent } from '../../types/matrix/room';

export const useStateEvent = (
  room: EventedRoomReading,
  eventType: StateEvent,
  stateKey = '',
  enabled = true
) => {
  return useMemo(
    () => (enabled ? getStateEvent(room, eventType, stateKey) : undefined),
    [room, eventType, stateKey, enabled]
  );
};
