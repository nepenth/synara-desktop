import { useEffect, useState } from 'react';
import type { RoomReading } from '../utils/room';
import { loadAllNativeAccountData, subscribeNativeAccountData } from '../native/nativeAccountData';

/** Room account data for Developer Tools, read through Core and kept current. */
export const useRoomAccountData = (room: Pick<RoomReading, 'roomId'>): Map<string, object> => {
  const [accountData, setAccountData] = useState<Map<string, object>>(() => new Map());

  useEffect(() => {
    let cancelled = false;
    const reload = () => {
      void loadAllNativeAccountData(room.roomId)
        .then((next) => {
          if (!cancelled) setAccountData(next);
        })
        .catch(() => undefined);
    };
    reload();
    const unsubscribe = subscribeNativeAccountData(reload);
    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [room.roomId]);

  return accountData;
};
