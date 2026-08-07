import { useCallback, useEffect, useState } from 'react';
import { StateEvent } from '../../types/matrix/room';
import type { MatrixEventReading } from '../utils/room';
import { EventedRoomReading, RoomEvent, RoomStateEvent } from '../utils/roomEvents';
import { getRoomCurrentState } from '../utils/timelineLifecycle';

export type StateKeyToEvents = Map<string, MatrixEventReading>;
export type StateTypeToState = Map<string, StateKeyToEvents>;

export const useRoomState = (room: EventedRoomReading): StateTypeToState => {
  const getState = useCallback((): StateTypeToState => {
    const roomState = getRoomCurrentState(
      room as unknown as Parameters<typeof getRoomCurrentState>[0]
    );
    const state: StateTypeToState = new Map();

    if (!roomState) return state;

    roomState.events.forEach((stateKeyToEvents, eventType) => {
      if (eventType === StateEvent.RoomMember) {
        // Ignore room members from state on purpose;
        return;
      }
      const kToE: StateKeyToEvents = new Map();
      stateKeyToEvents.forEach((mEvent, stateKey) => kToE.set(stateKey, mEvent));

      state.set(eventType, kToE);
    });

    return state;
  }, [room]);

  const [state, setState] = useState(getState);

  useEffect(() => {
    const handler: () => void = () => {
      setState(getState());
    };
    room.on(RoomStateEvent.Events, handler);
    room.on(RoomEvent.CurrentStateUpdated, handler);
    return () => {
      room.removeListener(RoomStateEvent.Events, handler);
      room.removeListener(RoomEvent.CurrentStateUpdated, handler);
    };
  }, [room, getState]);

  return state;
};
