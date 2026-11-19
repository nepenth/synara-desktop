import { useCallback, useEffect, useState } from 'react';
import { atom, useAtomValue, useSetAtom } from 'jotai';
import {
  createRoomFolderWithNativeOwner,
  deleteRoomFolderWithNativeOwner,
  isRoomFolderReadOnlyError,
  moveRoomFolderWithNativeOwner,
  renameRoomFolderWithNativeOwner,
  roomFolderErrorMessage,
  setRoomFolderWithNativeOwner,
  snapshotRoomFoldersWithNativeOwner,
  type NativeRoomFoldersSnapshot,
} from './nativeRoomFolderOwner';

/**
 * The folder snapshot, for management chrome only (read-only state). The
 * list itself renders from the room-list presentation, which Core refreshes
 * right after every local write.
 */
export const roomFoldersSnapshotAtom = atom<NativeRoomFoldersSnapshot | undefined>(undefined);

/**
 * True while a newer client's format is stored: every management action is
 * hidden. Unknown (not loaded yet, or the load failed) counts as writable; a
 * write then fails with `unsupported-version` and reloads the snapshot.
 */
export const useRoomFoldersReadOnly = (): boolean =>
  useAtomValue(roomFoldersSnapshotAtom)?.readOnly === true;

/**
 * Load the snapshot while a room list is mounted, and again when its folders
 * change (`signature`: ids and names from the presentation), so a version
 * written by another device is picked up.
 */
export const useRoomFoldersSnapshotLoader = (signature: string) => {
  const setSnapshot = useSetAtom(roomFoldersSnapshotAtom);
  useEffect(() => {
    let cancelled = false;
    snapshotRoomFoldersWithNativeOwner().then(
      (snapshot) => {
        if (!cancelled) setSnapshot(snapshot);
      },
      () => undefined
    );
    return () => {
      cancelled = true;
    };
  }, [signature, setSnapshot]);
};

export type RoomFolderCommands = {
  create: (name: string, roomId?: string) => Promise<string>;
  rename: (folderId: string, name: string) => Promise<void>;
  remove: (folderId: string) => Promise<void>;
  move: (folderId: string, toIndex: number) => Promise<void>;
  setRoom: (roomId: string, folderId: string | null) => Promise<void>;
};

/**
 * Folder commands bound to the snapshot atom. Rejections keep the native
 * error, so callers map it with `roomFolderErrorMessage`.
 */
export const useRoomFolderCommands = (): RoomFolderCommands => {
  const setSnapshot = useSetAtom(roomFoldersSnapshotAtom);
  const settle = useCallback(
    async <T>(run: () => Promise<T>, snapshotOf: (value: T) => NativeRoomFoldersSnapshot) => {
      try {
        const value = await run();
        setSnapshot(snapshotOf(value));
        return value;
      } catch (error) {
        if (isRoomFolderReadOnlyError(error)) {
          snapshotRoomFoldersWithNativeOwner().then(setSnapshot, () => undefined);
        }
        throw error;
      }
    },
    [setSnapshot]
  );
  return {
    create: useCallback(
      async (name: string, roomId?: string) =>
        (
          await settle(
            () => createRoomFolderWithNativeOwner(name, roomId),
            (result) => result.snapshot
          )
        ).folderId,
      [settle]
    ),
    rename: useCallback(
      async (folderId: string, name: string) => {
        await settle(
          () => renameRoomFolderWithNativeOwner(folderId, name),
          (s) => s
        );
      },
      [settle]
    ),
    remove: useCallback(
      async (folderId: string) => {
        await settle(
          () => deleteRoomFolderWithNativeOwner(folderId),
          (s) => s
        );
      },
      [settle]
    ),
    move: useCallback(
      async (folderId: string, toIndex: number) => {
        await settle(
          () => moveRoomFolderWithNativeOwner(folderId, toIndex),
          (s) => s
        );
      },
      [settle]
    ),
    setRoom: useCallback(
      async (roomId: string, folderId: string | null) => {
        await settle(
          () => setRoomFolderWithNativeOwner(roomId, folderId),
          (s) => s
        );
      },
      [settle]
    ),
  };
};

/**
 * One command at a time for a control: `busy` disables it while the command
 * runs (the list updates right after a local write, so no optimistic state),
 * `error` is the copy for the last failure.
 */
export const useRoomFolderAction = () => {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const run = useCallback(async (action: () => Promise<unknown>): Promise<boolean> => {
    setBusy(true);
    setError(undefined);
    try {
      await action();
      return true;
    } catch (caught) {
      setError(roomFolderErrorMessage(caught));
      return false;
    } finally {
      setBusy(false);
    }
  }, []);
  const clearError = useCallback(() => setError(undefined), []);
  return { busy, error, run, clearError };
};

/** A dialog request, hosted by the list (`RoomFolderDialogs`) so it outlives menus and virtual rows. */
export type RoomFolderDialogRequest =
  | { kind: 'create'; roomId?: string }
  | { kind: 'rename'; folderId: string; name: string }
  | { kind: 'delete'; folderId: string; name: string };

export const roomFolderDialogAtom = atom<RoomFolderDialogRequest | undefined>(undefined);

export const useOpenRoomFolderDialog = () => useSetAtom(roomFolderDialogAtom);
