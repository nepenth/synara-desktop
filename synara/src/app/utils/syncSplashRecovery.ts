/** SDK-neutral mirror of the js-sdk SyncState string values. */
const SyncState = {
  Error: 'ERROR',
  Prepared: 'PREPARED',
  Stopped: 'STOPPED',
  Syncing: 'SYNCING',
  Catchup: 'CATCHUP',
  Reconnecting: 'RECONNECTING',
} as const;

type SyncState = typeof SyncState[keyof typeof SyncState];

export const SYNC_PREPARED_TIMEOUT_MS = 30_000;

export type SyncSplashView = 'error' | 'recovery' | 'loading' | 'client';

export const shouldShowSyncRecoveryUI = (loading: boolean, syncTimedOut: boolean): boolean =>
  loading && syncTimedOut;

export type SelectSyncSplashViewOptions = {
  hasError: boolean;
  hasClient: boolean;
  loading: boolean;
  syncTimedOut: boolean;
};

export const selectSyncSplashView = ({
  hasError,
  hasClient,
  loading,
  syncTimedOut,
}: SelectSyncSplashViewOptions): SyncSplashView => {
  if (hasError) return 'error';
  if (shouldShowSyncRecoveryUI(loading, syncTimedOut)) return 'recovery';
  if (loading || !hasClient) return 'loading';
  return 'client';
};

export const formatSyncSplashStatus = (
  current: SyncState | null | undefined,
  hasClient: boolean
): string => {
  if (!hasClient) return 'Restoring session';

  switch (current) {
    case SyncState.Prepared:
      return 'Opening conversations';
    case SyncState.Syncing:
      return 'Syncing messages';
    case SyncState.Catchup:
      return 'Catching up';
    case SyncState.Reconnecting:
      return 'Reconnecting';
    case SyncState.Error:
      return 'Sync is retrying';
    case SyncState.Stopped:
      return 'Sync is stopped';
    default:
      return 'Starting Matrix sync';
  }
};

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
