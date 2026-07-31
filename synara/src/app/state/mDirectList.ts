import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

export type MDirectAction = {
  type: 'INITIALIZE' | 'PUT';
  rooms: Set<string>;
};

type NativeMDirectSnapshot = {
  sessionGeneration: number;
  roomIds: string[];
};

const baseMDirectAtom = atom(new Set<string>());
export const mDirectAtom = atom<Set<string>, [MDirectAction], undefined>(
  (get) => get(baseMDirectAtom),
  (get, set, action) => {
    set(baseMDirectAtom, action.rooms);
  }
);

export const mDirectRoomsFromNativeSnapshot = (roomIds: string[]): Set<string> => new Set(roomIds);

/**
 * Drive DM room-id set from the native Rust `m.direct` projection.
 * Create/mark-DM writers remain residual where they still use MatrixClient.
 */
export const useBindMDirectAtom = (mDirect: typeof mDirectAtom = mDirectAtom) => {
  const setMDirect = useSetAtom(mDirect);

  useEffect(() => {
    let disposed = false;
    let inFlight = false;

    const clear = () => {
      setMDirect({ type: 'INITIALIZE', rooms: new Set() });
    };

    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const session = await invokeDesktopWithAvailability<{
          status: 'logged_out' | 'logged_in';
        }>('matrix_session_snapshot');
        if (disposed) return;
        if (!session.available || session.value?.status !== 'logged_in') {
          clear();
          return;
        }
        const result = await invokeDesktopWithAvailability<NativeMDirectSnapshot>(
          'matrix_mdirect_snapshot'
        );
        if (!disposed && result.available && result.value) {
          setMDirect({
            type: 'PUT',
            rooms: mDirectRoomsFromNativeSnapshot(result.value.roomIds),
          });
        }
      } catch {
        // Preserve the last known DM set during transient failures.
      } finally {
        inFlight = false;
      }
    };

    if (!isSynaraDesktop()) {
      clear();
      return undefined;
    }

    void refresh();
    const pollId = window.setInterval(() => void refresh(), 1_000);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [setMDirect]);
};
