import { useEffect } from 'react';
import type { MatrixClientReading, MatrixEventReading } from '../utils/room';
import { RoomStateEvent } from '../utils/roomEvents';

export type StateEventCallback = (
  event: MatrixEventReading,
  state: unknown,
  lastStateEvent: MatrixEventReading | null
) => void;

export const useStateEventCallback = (
  mx: MatrixClientReading,
  onStateEvent: StateEventCallback
) => {
  useEffect(() => {
    (
      mx as unknown as {
        on(event: string, listener: StateEventCallback): void;
        removeListener(event: string, listener: StateEventCallback): void;
      }
    ).on(RoomStateEvent.Events, onStateEvent);
    return () => {
      (
        mx as unknown as {
          on(event: string, listener: StateEventCallback): void;
          removeListener(event: string, listener: StateEventCallback): void;
        }
      ).removeListener(RoomStateEvent.Events, onStateEvent);
    };
  }, [mx, onStateEvent]);
};
