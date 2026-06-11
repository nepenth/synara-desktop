import { SyncState } from 'matrix-js-sdk';

export const getSyncStatusBannerCopy = (state: SyncState | null): string | null => {
  if (state === SyncState.Catchup) {
    return 'Syncing history…';
  }
  if (state === SyncState.Prepared) {
    return 'Connected';
  }
  if (state === SyncState.Reconnecting) {
    return 'Connection Lost! Reconnecting...';
  }
  if (state === SyncState.Error) {
    return 'Connection Lost!';
  }
  return null;
};

export const getSyncStatusBannerVariant = (
  state: SyncState | null
): 'Success' | 'Warning' | 'Critical' | null => {
  if (state === SyncState.Catchup || state === SyncState.Prepared) {
    return 'Success';
  }
  if (state === SyncState.Reconnecting) {
    return 'Warning';
  }
  if (state === SyncState.Error) {
    return 'Critical';
  }
  return null;
};
