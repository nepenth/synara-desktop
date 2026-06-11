import { SyncState } from 'matrix-js-sdk';

export const SYNC_PREPARED_TIMEOUT_MS = 90_000;

export const shouldShowSyncRecoveryUI = (loading: boolean, syncTimedOut: boolean): boolean =>
  loading && syncTimedOut;

export const formatSyncStateTransition = (
  current: SyncState | null,
  previous: SyncState | null | undefined
): string => `sync ${String(previous)} -> ${String(current)}`;

export const logSyncStateTransition = (
  current: SyncState | null,
  previous: SyncState | null | undefined
): void => {
  // eslint-disable-next-line no-console
  console.info(`[synara:sync] ${formatSyncStateTransition(current, previous)}`);
};