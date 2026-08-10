import { useCallback, useMemo } from 'react';
import type { EventedRoomReading } from '../utils/roomEvents';
import { useStateEventCallback } from './useStateEventCallback';
import { useForceUpdate } from './useForceUpdate';
import { getStateEvent } from '../utils/room';
import { StateEvent } from '../../types/matrix/room';

export const useStateEvent = (
  room: EventedRoomReading,
  eventType: StateEvent,
  stateKey = '',
  enabled = true
) => {
  const [updateCount, forceUpdate] = useForceUpdate();

  useStateEventCallback(
    room.client,
    useCallback(
      (event) => {
        if (
          enabled &&
          event.getRoomId() === room.roomId &&
          event.getType() === eventType &&
          event.getStateKey() === stateKey
        ) {
          forceUpdate();
        }
      },
      [room, eventType, stateKey, enabled, forceUpdate]
    )
  );

  return useMemo(
    () => (enabled ? getStateEvent(room, eventType, stateKey) : undefined),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [room, eventType, stateKey, enabled, updateCount]
  );
};
