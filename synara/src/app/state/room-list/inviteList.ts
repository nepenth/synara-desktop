import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useCallback, useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { startRoomListUpdateDrivenPoll } from '../../utils/nativeRoomListUpdates';
import {
  createCoalescedRefresh,
  keepIfSameJson,
  readNativeSessionSnapshotShared,
  readNativeSyncStatusShared,
} from '../../utils/nativeListRefresh';
import type {
  NativeInvite,
  NativeInviteSnapshot,
  NativeInviteTriage,
  SyncReadinessSnapshot,
} from '../../features/matrix-dto/generated';

type NativeSyncReadiness = Pick<SyncReadinessSnapshot, 'readiness'>;

const emptyInviteSnapshot: NativeInviteSnapshot = { sessionGeneration: 0, invites: [] };
const nativeInviteSnapshotAtom = atom<NativeInviteSnapshot>(emptyInviteSnapshot);
const nativeInviteSyncingAtom = atom(false);

// Counts and badges derive from Rust-owned invite data. Keeping the legacy
// string-shaped read atom avoids spreading the native DTO through unrelated UI.
export const allInvitesAtom = atom((get) =>
  get(nativeInviteSnapshotAtom).invites.map((invite) => invite.roomId)
);

export const useNativeInvites = (): NativeInviteSnapshot => useAtomValue(nativeInviteSnapshotAtom);
export const useNativeInviteSyncing = (): boolean => useAtomValue(nativeInviteSyncingAtom);

export const useBindAllInvitesAtom = () => {
  const setSnapshot = useSetAtom(nativeInviteSnapshotAtom);
  const setSyncing = useSetAtom(nativeInviteSyncingAtom);

  useEffect(() => {
    let disposed = false;
    // Unchanged readbacks keep the atom value, so invite consumers do not
    // re-render on every room-list wake.
    const publish = (snapshot: NativeInviteSnapshot) =>
      setSnapshot((current) => keepIfSameJson(current, snapshot));

    const refresh = createCoalescedRefresh(
      async () => {
        try {
          const session = await readNativeSessionSnapshotShared<NativeSessionSnapshot>();
          if (disposed || !session.available) return;
          if (session.value?.status !== 'logged_in') {
            publish(emptyInviteSnapshot);
            setSyncing(false);
            return;
          }
          const syncStatus = await readNativeSyncStatusShared<NativeSyncReadiness>();
          if (!disposed && syncStatus.available && syncStatus.value) {
            setSyncing(syncStatus.value.readiness === 'running');
          }
          const result =
            await invokeDesktopWithAvailability<NativeInviteSnapshot>('matrix_invites_snapshot');
          if (!disposed && result.available && result.value) publish(result.value);
        } catch {
          // The native command records a privacy-safe diagnostic. Preserve the last
          // known snapshot during a transient sync or protocol failure.
        }
      },
      () => disposed
    );

    if (!isSynaraDesktop()) {
      setSnapshot(emptyInviteSnapshot);
      setSyncing(false);
      return undefined;
    }

    void refresh();
    const stopPolling = startRoomListUpdateDrivenPoll(() => void refresh());
    return () => {
      disposed = true;
      stopPolling();
    };
  }, [setSnapshot, setSyncing]);
};

type NativeSessionSnapshot = { status: 'logged_out' | 'logged_in' };

export type NativeInviteCommand =
  | 'matrix_invites_accept'
  | 'matrix_invites_decline'
  | 'matrix_invites_report_spam'
  | 'matrix_invites_block_sender';

export const useNativeInviteCommand = () => {
  const setSnapshot = useSetAtom(nativeInviteSnapshotAtom);

  return useCallback(
    async (command: NativeInviteCommand, roomId: string): Promise<NativeInviteSnapshot> => {
      const result = await invokeDesktopWithAvailability<NativeInviteSnapshot>(command, { roomId });
      if (!result.available || !result.value) {
        throw new Error('Native invite actions are unavailable in this client.');
      }
      setSnapshot(result.value);
      return result.value;
    },
    [setSnapshot]
  );
};

export type { NativeInvite, NativeInviteSnapshot, NativeInviteTriage };
