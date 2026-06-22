import {
  MatrixEvent,
  Room,
  RoomEvent,
  RoomEventHandlerMap,
  RoomStateEvent,
  RoomStateEventHandlerMap,
} from 'matrix-js-sdk';
import { useCallback, useEffect, useState } from 'react';
import { StateEvent } from '../../types/matrix/room';
import { getRoomCurrentState } from '../utils/timelineLifecycle';

export type StateKeyToEvents = Map<string, MatrixEvent>;
export type StateTypeToState = Map<string, StateKeyToEvents>;

export const useRoomState = (room: Room): StateTypeToState => {
  const getState = useCallback((): StateTypeToState => {
    const roomState = getRoomCurrentState(room);
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
    const handler: RoomStateEventHandlerMap[RoomStateEvent.Events] = () => {
      setState(getState());
    };
    const handleCurrentStateUpdated: RoomEventHandlerMap[RoomEvent.CurrentStateUpdated] = () => {
      setState(getState());
    };

    room.on(RoomStateEvent.Events, handler);
    room.on(RoomEvent.CurrentStateUpdated, handleCurrentStateUpdated);
    return () => {
      room.removeListener(RoomStateEvent.Events, handler);
      room.removeListener(RoomEvent.CurrentStateUpdated, handleCurrentStateUpdated);
    };
  }, [room, getState]);

  return state;
};
