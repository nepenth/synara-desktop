import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import type { SynaraRoomNotesContent } from '../../types/matrix/accountData';
import { normalizeRoomNotesContent } from '../utils/roomNotes';

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
    set(baseRoomNotesContentAtom, normalizeRoomNotesContent(action.content));
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
    let inFlight = false;

    const clear = () => {
      setRoomNotesContent({ type: 'INITIALIZE', content: emptyRoomNotesContent() });
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
  }, [setRoomNotesContent]);
};
