import type { EventedRoomReading } from '../utils/roomEvents';
import { RoomEvent } from '../utils/roomEvents';
import { useCallback, useEffect, useState } from 'react';

export const useRoomAccountData = (room: EventedRoomReading): Map<string, object> => {
  const getAccountData = useCallback((): Map<string, object> => {
    const accountData = new Map<string, object>();
    const roomAccountData = (
      room as unknown as {
        accountData: Map<string, { getContent(): object }>;
      }
    ).accountData;

    Array.from(roomAccountData.entries()).forEach(([type, mEvent]) => {
      const content = mEvent.getContent();
      accountData.set(type, content);
    });

    return accountData;
  }, [room]);

  const [accountData, setAccountData] = useState<Map<string, object>>(getAccountData);

  useEffect(() => {
    const handleEvent: (...args: unknown[]) => void = () => {
      setAccountData(getAccountData());
    };
    room.on(RoomEvent.AccountData, handleEvent);
    return () => {
      room.removeListener(RoomEvent.AccountData, handleEvent);
    };
  }, [room, getAccountData]);

  return accountData;
};
