import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import type { SynaraRoomNotesContent } from '../../types/matrix/accountData';
import { normalizeRoomNotesContent } from '../utils/roomNotes';
import { startRoomListUpdateDrivenPoll } from '../utils/nativeRoomListUpdates';
import {
  createCoalescedRefresh,
  keepIfSameJson,
  readNativeSessionSnapshotShared,
} from '../utils/nativeListRefresh';

export type RoomNotesContentAction = {
  type: 'INITIALIZE' | 'PUT';
  content: SynaraRoomNotesContent;
};

type NativeRoomNotesSnapshot = {
  sessionGeneration: number;
  content: SynaraRoomNotesContent;
};

const emptyRoomNotesContent = (): SynaraRoomNotesContent => ({
  version: 1,
  rooms: {},
});

const baseRoomNotesContentAtom = atom<SynaraRoomNotesContent>(emptyRoomNotesContent());
export const roomNotesContentAtom = atom<
  SynaraRoomNotesContent,
  [RoomNotesContentAction],
  undefined
>(
  (get) => get(baseRoomNotesContentAtom),
  (_get, set, action) => {
    const next = normalizeRoomNotesContent(action.content);
    set(baseRoomNotesContentAtom, (current) => keepIfSameJson(current, next));
  }
);

/**
 * Drive room notes panel/header state from the native Rust `in.synara.room_notes` projection.
 */
export const useBindRoomNotesContentAtom = (
  roomNotesContent: typeof roomNotesContentAtom = roomNotesContentAtom
) => {
  const setRoomNotesContent = useSetAtom(roomNotesContent);

  useEffect(() => {
    let disposed = false;

    const clear = () => {
      setRoomNotesContent({ type: 'INITIALIZE', content: emptyRoomNotesContent() });
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
          const result = await invokeDesktopWithAvailability<NativeRoomNotesSnapshot>(
            'matrix_room_notes_snapshot'
          );
          if (!disposed && result.available && result.value) {
            setRoomNotesContent({
              type: 'PUT',
              content: result.value.content,
            });
          }
        } catch {
          // Preserve last known notes content during transient failures.
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
  }, [setRoomNotesContent]);
};
