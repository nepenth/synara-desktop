import { useCallback, useEffect, useState } from 'react';
import { StateEvent } from '../../types/matrix/room';
import type { MatrixEventReading } from '../utils/room';
import { EventedRoomReading, RoomEvent, RoomStateEvent } from '../utils/roomEvents';
import { getRoomCurrentState } from '../utils/timelineLifecycle';

export type StateKeyToEvents = Map<string, MatrixEventReading>;
export type StateTypeToState = Map<string, StateKeyToEvents>;

type RoomStateEventsSource =
  | {
      events?: {
        forEach(
          callback: (
            stateKeyToEvents:
              | {
                  forEach(callback: (event: MatrixEventReading, stateKey: string) => void): void;
                }
              | null
              | undefined,
            eventType: string
          ) => void
        ): void;
      } | null;
    }
  | null
  | undefined;

/**
 * Copy indexed room state for Developer Tools. Native room projections expose
 * getStateEvents but not the js-sdk `events` Map; missing maps must fail
 * closed as empty instead of throwing during render.
 */
export const collectRoomStateEvents = (roomState: RoomStateEventsSource): StateTypeToState => {
  const state: StateTypeToState = new Map();
  const events = roomState?.events;
  if (!events || typeof events.forEach !== 'function') return state;

  events.forEach((stateKeyToEvents, eventType) => {
    if (eventType === StateEvent.RoomMember) return;
    if (!stateKeyToEvents || typeof stateKeyToEvents.forEach !== 'function') return;

    const kToE: StateKeyToEvents = new Map();
    stateKeyToEvents.forEach((mEvent, stateKey) => kToE.set(stateKey, mEvent));
    state.set(eventType, kToE);
  });

  return state;
};

export const useRoomState = (room: EventedRoomReading): StateTypeToState => {
  const getState = useCallback((): StateTypeToState => {
    const roomState = getRoomCurrentState(
      room as unknown as Parameters<typeof getRoomCurrentState>[0]
    );
    return collectRoomStateEvents(roomState);
  }, [room]);

  const [state, setState] = useState(getState);

  useEffect(() => {
    if (typeof room.on !== 'function' || typeof room.removeListener !== 'function') {
      return undefined;
    }
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
