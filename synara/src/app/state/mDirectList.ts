import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import { startRoomListUpdateDrivenPoll } from '../utils/nativeRoomListUpdates';
import {
  createCoalescedRefresh,
  readNativeSessionSnapshotShared,
} from '../utils/nativeListRefresh';

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
    // An unchanged set keeps its identity, so DM consumers do not re-render
    // on every room-list wake.
    const current = get(baseMDirectAtom);
    if (current.size === action.rooms.size && [...current].every((id) => action.rooms.has(id))) {
      return;
    }
    set(baseMDirectAtom, action.rooms);
  }
);

const baseMDirectUsersAtom = atom<string[]>([]);
export const mDirectUsersAtom = atom<string[], [MDirectUsersAction], undefined>(
  (get) => get(baseMDirectUsersAtom),
  (get, set, action) => {
    const current = get(baseMDirectUsersAtom);
    if (
      current.length === action.users.length &&
      current.every((userId, index) => userId === action.users[index])
    ) {
      return;
    }
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

    const clear = () => {
      setMDirect({ type: 'INITIALIZE', rooms: new Set() });
      setMDirectUsers({ type: 'INITIALIZE', users: [] });
    };

    const refresh = createCoalescedRefresh(
      async () => {
        try {
          const session = await readNativeSessionSnapshotShared<{
            status: 'logged_out' | 'logged_in';
          }>();
          if (disposed) return;
          if (!session.available || session.value?.status !== 'logged_in') {
            clear();
            return;
          }
          const result =
            await invokeDesktopWithAvailability<NativeMDirectSnapshot>('matrix_mdirect_snapshot');
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
        }
      },
      () => disposed
    );

    if (!isSynaraDesktop()) {
      clear();
      return undefined;
    }

    void refresh();
    const stopPolling = startRoomListUpdateDrivenPoll(() => void refresh());
    return () => {
      disposed = true;
      stopPolling();
    };
  }, [setMDirect, setMDirectUsers]);
};

/** Product hook: DM user keys with at least one joined room (native-owned). */
export const useDirectUsers = (): string[] => useAtomValue(mDirectUsersAtom);
