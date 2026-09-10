import type { EventedRoomReading } from '../utils/roomEvents';
import { RoomEvent } from '../utils/roomEvents';
import { useCallback, useEffect, useState } from 'react';

type AccountDataEventReading = {
  getContent(): object;
};

/**
 * Copy room account data for Developer Tools. Native rooms may expose a
 * get-only stub instead of a js-sdk Map; missing iterators fail closed empty.
 */
export const collectRoomAccountData = (accountDataSource: unknown): Map<string, object> => {
  const accountData = new Map<string, object>();
  if (
    !accountDataSource ||
    typeof accountDataSource !== 'object' ||
    typeof (accountDataSource as { entries?: unknown }).entries !== 'function'
  ) {
    return accountData;
  }

  for (const [type, mEvent] of (
    accountDataSource as Map<string, AccountDataEventReading>
  ).entries()) {
    if (!mEvent || typeof mEvent.getContent !== 'function') continue;
    accountData.set(type, mEvent.getContent());
  }

  return accountData;
};

export const useRoomAccountData = (room: EventedRoomReading): Map<string, object> => {
  const getAccountData = useCallback(
    (): Map<string, object> => collectRoomAccountData(room.accountData),
    [room]
  );

  const [accountData, setAccountData] = useState<Map<string, object>>(getAccountData);

  useEffect(() => {
    if (typeof room.on !== 'function' || typeof room.removeListener !== 'function') {
      return undefined;
    }
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
