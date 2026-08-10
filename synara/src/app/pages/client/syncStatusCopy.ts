/** SDK-neutral mirror of the js-sdk SyncState string values. */
const SyncState = {
  Error: 'ERROR',
  Prepared: 'PREPARED',
  Stopped: 'STOPPED',
  Syncing: 'SYNCING',
  Catchup: 'CATCHUP',
  Reconnecting: 'RECONNECTING',
} as const;

export type SyncState = typeof SyncState[keyof typeof SyncState];

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

export const getSlidingSyncCapabilityBannerCopy = (): string =>
  'This homeserver does not advertise sliding-sync (MSC4186) support, so sync may not start. Contact your server administrator.';

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
