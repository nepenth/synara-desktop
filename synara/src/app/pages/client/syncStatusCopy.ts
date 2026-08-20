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

/**
 * A ready sync service is a steady state, not a permanent success alert. The
 * component owns the short transition window and asks this helper whether the
 * otherwise persistent PREPARED copy may be shown.
 *
 * Native SyncService reports `Offline` during brief sliding-sync gaps. The
 * banner holds RECONNECTING until that state lasts, and only flashes Connected
 * after a Lost banner the user actually saw.
 */
export const getTransientSyncStatusBannerCopy = (
  state: SyncState | null,
  connectedTransitionVisible: boolean,
  reconnectingBannerVisible = true
): string | null => {
  if (state === SyncState.Prepared && !connectedTransitionVisible) return null;
  if (state === SyncState.Reconnecting && !reconnectingBannerVisible) return null;
  return getSyncStatusBannerCopy(state);
};

export const CONNECTED_STATUS_BANNER_DURATION_MS = 4_000;

/** Ignore SDK Offline blips shorter than this before showing Connection Lost. */
export const RECONNECTING_BANNER_HOLD_MS = 4_000;

export const shouldShowConnectedTransition = (
  state: SyncState | null,
  recoveredFromVisibleDisconnect: boolean
): boolean => state === SyncState.Prepared && recoveredFromVisibleDisconnect;

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
