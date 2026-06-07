import { SyncState } from 'matrix-js-sdk';

export const shouldRetrySyncOnResume = (state: SyncState | null): boolean =>
  state === SyncState.Reconnecting || state === SyncState.Error;
