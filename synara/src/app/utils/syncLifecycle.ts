export const shouldRetrySyncOnResume = (state: string | null): boolean =>
  state === 'RECONNECTING' || state === 'ERROR';

/**
 * A live sync is restarted on show/focus only after a long hide. Linux close
 * now hides to the tray, so every reopen is a visibility change; restarting a
 * healthy SyncService each time flashes "connecting" for nothing. OS sleep is
 * caught by the native suspend watchdog, and RECONNECTING/ERROR still recover
 * immediately.
 */
export const SYNC_WAKE_HIDDEN_MS = 5 * 60_000;
export const SYNC_WAKE_RECOVER_COOLDOWN_MS = 8_000;

export type SyncWakeReason = 'visibilitychange' | 'focus' | 'online' | 'pageshow';

export const hiddenDurationMs = (hiddenAtMs: number | null, nowMs: number): number =>
  hiddenAtMs === null ? 0 : Math.max(0, nowMs - hiddenAtMs);

/** Take this hide's duration and clear the stamp so a later focus cannot reuse it. */
export const consumeHiddenDurationMs = (
  hiddenAtMs: number | null,
  nowMs: number
): { hiddenDurationMs: number; hiddenAtMs: null } => ({
  hiddenDurationMs: hiddenDurationMs(hiddenAtMs, nowMs),
  hiddenAtMs: null,
});

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
