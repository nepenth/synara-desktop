import { useEffect } from 'react';
import type { NativeSession } from '../native/nativeSession';
import type { NativeSyncState, NativeSyncStateData } from '../native/nativeWire';

/** Receives each connection-state transition and the state before it. */
export type SyncStateHandler = (
  syncState: NativeSyncState,
  prevState: NativeSyncState | null,
  data?: NativeSyncStateData
) => void;

/**
 * Report the session's connection state now and on every transition. A
 * missing session (still loading) reports nothing.
 */
export const useSyncState = (
  session: NativeSession | undefined,
  onChange: SyncStateHandler
): void => {
  useEffect(() => {
    if (!session) return undefined;

    let previous = session.getSyncState();
    const unsubscribe = session.subscribe('sync', (next) => {
      onChange(next, previous, session.getSyncStateData() ?? undefined);
      previous = next;
    });

    if (previous !== null) {
      onChange(previous, null, session.getSyncStateData() ?? undefined);
    }

    return unsubscribe;
  }, [session, onChange]);
};
