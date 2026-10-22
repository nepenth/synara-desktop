import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import type { SynaraLaterContent } from '../../types/matrix/accountData';
import { emptyLaterContent, normalizeLaterContent } from '../utils/later';
import { startRoomListUpdateDrivenPoll } from '../utils/nativeRoomListUpdates';
import {
  createCoalescedRefresh,
  keepIfSameJson,
  readNativeSessionSnapshotShared,
} from '../utils/nativeListRefresh';

export type LaterContentAction = {
  type: 'INITIALIZE' | 'PUT';
  content: SynaraLaterContent;
};

type NativeLaterSnapshot = {
  sessionGeneration: number;
  content: SynaraLaterContent;
};

const baseLaterContentAtom = atom<SynaraLaterContent>(emptyLaterContent());
export const laterContentAtom = atom<SynaraLaterContent, [LaterContentAction], undefined>(
  (get) => get(baseLaterContentAtom),
  (_get, set, action) => {
    const next = normalizeLaterContent(action.content);
    set(baseLaterContentAtom, (current) => keepIfSameJson(current, next));
  }
);

/**
 * Drive Later inbox/badge state from the native Rust `in.synara.later` projection.
 */
export const useBindLaterContentAtom = (
  laterContent: typeof laterContentAtom = laterContentAtom
) => {
  const setLaterContent = useSetAtom(laterContent);

  useEffect(() => {
    let disposed = false;

    const clear = () => {
      setLaterContent({ type: 'INITIALIZE', content: emptyLaterContent() });
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
            await invokeDesktopWithAvailability<NativeLaterSnapshot>('matrix_later_snapshot');
          if (!disposed && result.available && result.value) {
            setLaterContent({
              type: 'PUT',
              content: result.value.content,
            });
          }
        } catch {
          // Preserve last known later content during transient failures.
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
  }, [setLaterContent]);
};
