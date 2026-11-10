import { useCallback, useRef, useState } from 'react';
import { useAtomValue } from 'jotai';
import { useOrphanSpaces } from '../../state/hooks/roomList';
import { roomToParentsAtom } from '../../state/room/roomToParents';
import { allRoomsAtom } from '../../state/room-list/roomList';
import {
  SidebarItems,
  makeSynaraSpacesContent,
  parseSidebar,
  saveSynaraSpacesContent,
  sidebarItemWithout,
  useSidebarItems,
} from '../../hooks/useSidebarItems';
import { getNativeRoom } from '../../native/nativeSession';

export type SpaceOrder = {
  /** Top-level spaces and folders, in the saved order. */
  items: SidebarItems;
  /** Spaces with no parent space: always listed, so never unpinnable. */
  orphanSpaces: string[];
  /** Show `next` at once and save it; on failure put `previous` back and say `failure`. */
  save: (previous: SidebarItems, next: SidebarItems, failure: string) => void;
  /** Drop a pinned (non-orphan) space from the list. */
  unpin: (roomId: string) => void;
  saveError?: string;
  clearSaveError: () => void;
};

/**
 * The space switcher's list and its writes to `in.synara.spaces`. Shared by
 * the switcher menu and the Manage spaces dialog, which outlives the menu.
 */
export const useSpaceOrder = (): SpaceOrder => {
  const roomToParents = useAtomValue(roomToParentsAtom);
  const orphanSpaces = useOrphanSpaces(allRoomsAtom, roomToParents);
  const [items, localEchoItems] = useSidebarItems(orphanSpaces);
  const [saveError, setSaveError] = useState<string>();
  const saveSequence = useRef(0);

  /**
   * Show the new layout at once and persist it. If the write fails, put the
   * previous layout back right away and say so, instead of showing success
   * until the next Core refresh silently reverts it.
   */
  const save = useCallback(
    (previous: SidebarItems, next: SidebarItems, failure: string) => {
      const newSpacesContent = makeSynaraSpacesContent(next);
      localEchoItems(parseSidebar(orphanSpaces, newSpacesContent));
      setSaveError(undefined);
      saveSequence.current += 1;
      const sequence = saveSequence.current;
      saveSynaraSpacesContent(newSpacesContent).catch(() => {
        // A newer layout supersedes this one; never revert over it.
        if (sequence !== saveSequence.current) return;
        localEchoItems(previous);
        setSaveError(failure);
      });
    },
    [localEchoItems, orphanSpaces]
  );

  const unpin = useCallback(
    (roomId: string) => {
      if (orphanSpaces.includes(roomId)) return;
      const name = getNativeRoom(roomId)?.name ?? 'this space';
      save(items, sidebarItemWithout(items, roomId), `Couldn't unpin ${name}. Try again.`);
    },
    [items, orphanSpaces, save]
  );

  const clearSaveError = useCallback(() => setSaveError(undefined), []);

  return { items, orphanSpaces, save, unpin, saveError, clearSaveError };
};
