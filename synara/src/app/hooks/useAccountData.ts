import { useState, useCallback } from 'react';
import { useMatrixClient } from './useMatrixClient';
import { useAccountDataCallback } from './useAccountDataCallback';

export function useAccountData(eventType: string, enabled = true) {
  const mx = useMatrixClient();
  const [event, setEvent] = useState(() =>
    enabled ? mx.getAccountData(eventType as any) : undefined
  );

  useAccountDataCallback(
    mx,
    useCallback(
      (evt) => {
        if (evt.getType() === eventType) {
          setEvent(evt);
        }
      },
      [eventType, setEvent]
    ),
    enabled
  );

  return enabled ? event : undefined;
}
