import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useCallback, useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';

export type NativeInviteTriage = 'known' | 'public' | 'spam';

export type NativeInvite = {
  roomId: string;
  roomName: string;
  avatarHandleId?: string;
  roomTopic?: string;
  roomAlias?: string;
  senderId: string;
  senderName: string;
  senderIgnored: boolean;
  inviteTs?: number;
  reason?: string;
  isSpace: boolean;
  isDirect: boolean;
  isEncrypted: boolean;
  triage: NativeInviteTriage;
};

export type NativeInviteSnapshot = {
  sessionGeneration: number;
  invites: NativeInvite[];
};

type NativeSyncReadiness = {
  readiness: 'unconfigured' | 'idle' | 'running' | 'offline' | 'terminated' | 'failed';
};

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
    let inFlight = false;

    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const session = await invokeDesktopWithAvailability<NativeSessionSnapshot>(
          'matrix_session_snapshot'
        );
        if (disposed || !session.available) return;
        if (session.value?.status !== 'logged_in') {
          setSnapshot(emptyInviteSnapshot);
          setSyncing(false);
          return;
        }
        const syncStatus = await invokeDesktopWithAvailability<NativeSyncReadiness>(
          'matrix_sync_status'
        );
        if (!disposed && syncStatus.available && syncStatus.value) {
          setSyncing(syncStatus.value.readiness === 'running');
        }
        const result = await invokeDesktopWithAvailability<NativeInviteSnapshot>(
          'matrix_invites_snapshot'
        );
        if (!disposed && result.available && result.value) setSnapshot(result.value);
      } catch {
        // The native command records a privacy-safe diagnostic. Preserve the last
        // known snapshot during a transient sync or protocol failure.
      } finally {
        inFlight = false;
      }
    };

    if (!isSynaraDesktop()) {
      setSnapshot(emptyInviteSnapshot);
      setSyncing(false);
      return undefined;
    }

    void refresh();
    const pollId = window.setInterval(() => void refresh(), 1_000);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
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
