export const shouldRetrySyncOnResume = (state: string | null): boolean =>
  state === 'RECONNECTING' || state === 'ERROR';

export const SYNC_WAKE_HIDDEN_MS = 15_000;
export const SYNC_WAKE_RECOVER_COOLDOWN_MS = 8_000;

export type SyncWakeReason = 'visibilitychange' | 'focus' | 'online' | 'pageshow';

export const hiddenDurationMs = (hiddenAtMs: number | null, nowMs: number): number =>
  hiddenAtMs === null ? 0 : Math.max(0, nowMs - hiddenAtMs);

const isLiveSyncState = (state: string | null): boolean =>
  state === 'PREPARED' || state === 'SYNCING' || state === 'CATCHUP';

export const shouldRecoverSyncOnWake = (input: {
  reason: SyncWakeReason;
  syncState: string | null;
  hiddenDurationMs: number;
  persisted?: boolean;
}): boolean => {
  if (shouldRetrySyncOnResume(input.syncState)) {
    return true;
  }
  if (!isLiveSyncState(input.syncState)) {
    return false;
  }
  if (input.reason === 'online') {
    return true;
  }
  if (input.reason === 'pageshow' && input.persisted === true) {
    return true;
  }
  if (
    (input.reason === 'visibilitychange' || input.reason === 'focus') &&
    input.hiddenDurationMs >= SYNC_WAKE_HIDDEN_MS
  ) {
    return true;
  }
  return false;
};
