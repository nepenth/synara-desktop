import { useEffect } from 'react';
import type { MatrixClientReading } from '../utils/room';
import { ClientEvent } from '../utils/roomEvents';

/**
 * Structural listener for the js-sdk 'sync' client event.
 * Params stay permissive (`any`) on purpose: callers keep threading the
 * js-sdk SyncState values into their own state (typed from the js-sdk enum),
 * while the runtime values are plain strings ('SYNCING', 'PREPARED', ...).
 */
export type SyncStateHandler = (syncState: any, prevState: any, data?: any) => void;

type ClientEventedReading = MatrixClientReading & {
  getSyncState(): unknown;
  getSyncStateData(): unknown;
  on(event: string, listener: (...args: any[]) => unknown): unknown;
  removeListener(event: string, listener: (...args: any[]) => unknown): unknown;
};

export const useSyncState = (
  mx: ClientEventedReading | undefined,
  onChange: SyncStateHandler
): void => {
  useEffect(() => {
    if (!mx) return undefined;

    mx.on(ClientEvent.Sync, onChange);

    const currentState = mx.getSyncState();
    if (currentState !== null) {
      onChange(currentState, null, mx.getSyncStateData() ?? undefined);
    }

    return () => {
      mx.removeListener(ClientEvent.Sync, onChange);
    };
  }, [mx, onChange]);
};
