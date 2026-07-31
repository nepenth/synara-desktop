import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

export type MDirectAction = {
  type: 'INITIALIZE' | 'PUT';
  rooms: Set<string>;
};

export type MDirectUsersAction = {
  type: 'INITIALIZE' | 'PUT';
  users: string[];
};

type NativeMDirectSnapshot = {
  sessionGeneration: number;
  roomIds: string[];
  userIds: string[];
};

const baseMDirectAtom = atom(new Set<string>());
export const mDirectAtom = atom<Set<string>, [MDirectAction], undefined>(
  (get) => get(baseMDirectAtom),
  (get, set, action) => {
    set(baseMDirectAtom, action.rooms);
  }
);

const baseMDirectUsersAtom = atom<string[]>([]);
export const mDirectUsersAtom = atom<string[], [MDirectUsersAction], undefined>(
  (get) => get(baseMDirectUsersAtom),
  (get, set, action) => {
    set(baseMDirectUsersAtom, action.users);
  }
);

export const mDirectRoomsFromNativeSnapshot = (roomIds: string[]): Set<string> => new Set(roomIds);

export const mDirectUsersFromNativeSnapshot = (userIds: string[]): string[] => [...userIds];

/**
 * Drive DM room-id set and DM user-id list from the native Rust `m.direct` projection.
 */
export const useBindMDirectAtom = (mDirect: typeof mDirectAtom = mDirectAtom) => {
  const setMDirect = useSetAtom(mDirect);
  const setMDirectUsers = useSetAtom(mDirectUsersAtom);

  useEffect(() => {
    let disposed = false;
    let inFlight = false;

    const clear = () => {
      setMDirect({ type: 'INITIALIZE', rooms: new Set() });
      setMDirectUsers({ type: 'INITIALIZE', users: [] });
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
          setMDirectUsers({
            type: 'PUT',
            users: mDirectUsersFromNativeSnapshot(result.value.userIds ?? []),
          });
        }
      } catch {
        // Preserve the last known DM projection during transient failures.
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
  }, [setMDirect, setMDirectUsers]);
};

/** Product hook: DM user keys with at least one joined room (native-owned). */
export const useDirectUsers = (): string[] => useAtomValue(mDirectUsersAtom);
