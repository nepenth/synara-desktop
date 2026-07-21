import { useEffect, useMemo, useReducer, useRef, useSyncExternalStore } from 'react';
import { MatrixClient } from 'matrix-js-sdk';
import {
  getLegacyRoomActivitySnapshot,
  getRecentRoomExpiryDelay,
  getRoomActivityStore,
  partitionRoomIdsByActivity,
} from '../state/room-list/roomActivity';
import { isFoundationFeatureEnabled } from '../config/foundationFeatures';
import { recordFoundationDiagnostic } from '../utils/foundationDiagnostics';

const subscribeDisabledRoomActivity: (listener: () => void) => () => void = () => () => undefined;

export const useRecentRoomPartition = (mx: MatrixClient, roomIds: readonly string[]) => {
  const reactiveEnabled = isFoundationFeatureEnabled('reactiveRoomActivity');
  const legacySnapshot = useMemo(
    () => (reactiveEnabled ? undefined : getLegacyRoomActivitySnapshot(mx, roomIds)),
    [mx, reactiveEnabled, roomIds]
  );
  const snapshotSource = useMemo(() => {
    if (!reactiveEnabled) {
      const snapshot = legacySnapshot ?? getLegacyRoomActivitySnapshot(mx, roomIds);
      return {
        subscribe: subscribeDisabledRoomActivity,
        getSnapshot: () => snapshot,
      };
    }
    const store = getRoomActivityStore(mx);
    return { subscribe: store.subscribe, getSnapshot: store.getSnapshot };
  }, [legacySnapshot, mx, reactiveEnabled, roomIds]);
  const snapshot = useSyncExternalStore(
    snapshotSource.subscribe,
    snapshotSource.getSnapshot,
    snapshotSource.getSnapshot
  );
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
    recordFoundationDiagnostic('activity', 'room-activity.rollout-mode', {
      fields: { feature: 'reactiveRoomActivity', enabled: reactiveEnabled },
    });
  }, [reactiveEnabled]);

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
