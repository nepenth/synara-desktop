import { useEffect, useMemo, useReducer, useRef, useSyncExternalStore } from 'react';
import { MatrixClient } from 'matrix-js-sdk';
import {
  getRecentRoomExpiryDelay,
  getRoomActivityStore,
  partitionRoomIdsByActivity,
} from '../state/room-list/roomActivity';

export const useRecentRoomPartition = (mx: MatrixClient, roomIds: readonly string[]) => {
  const store = getRoomActivityStore(mx);
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
  const [clockRevision, refreshClock] = useReducer((revision: number) => revision + 1, 0);
  const clockRef = useRef({ clockRevision, roomIds, snapshot, nowMs: Date.now() });
  if (
    clockRef.current.clockRevision !== clockRevision ||
    clockRef.current.roomIds !== roomIds ||
    clockRef.current.snapshot !== snapshot
  ) {
    clockRef.current = { clockRevision, roomIds, snapshot, nowMs: Date.now() };
  }
  const { nowMs } = clockRef.current;

  useEffect(() => {
    const delay = getRecentRoomExpiryDelay(roomIds, snapshot, Date.now());
    if (delay === undefined) return undefined;
    const timeout = window.setTimeout(refreshClock, delay);
    return () => window.clearTimeout(timeout);
  }, [clockRevision, roomIds, snapshot]);

  useEffect(() => {
    const refreshWhenVisible = () => {
      if (document.visibilityState === 'visible') refreshClock();
    };
    window.addEventListener('focus', refreshClock);
    window.addEventListener('pageshow', refreshClock);
    document.addEventListener('visibilitychange', refreshWhenVisible);
    return () => {
      window.removeEventListener('focus', refreshClock);
      window.removeEventListener('pageshow', refreshClock);
      document.removeEventListener('visibilitychange', refreshWhenVisible);
    };
  }, []);

  return useMemo(
    () =>
      partitionRoomIdsByActivity(
        roomIds,
        snapshot,
        nowMs,
        (roomId) => mx.getRoom(roomId)?.name ?? roomId
      ),
    [mx, nowMs, roomIds, snapshot]
  );
};
