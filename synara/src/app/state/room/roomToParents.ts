import produce from 'immer';
import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { RoomToParents } from '../../../types/matrix/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { mapParentWithChildren } from '../../utils/room';

export type RoomToParentsAction =
  | {
      type: 'INITIALIZE';
      roomToParents: RoomToParents;
    }
  | {
      type: 'PUT';
      parent: string;
      children: string[];
    }
  | {
      type: 'DELETE';
      roomId: string;
    };

type NativeSpaceParentEntry = {
  roomId: string;
  parentIds: string[];
};

type NativeSpaceParentsSnapshot = {
  sessionGeneration: number;
  entries: NativeSpaceParentEntry[];
};

const baseRoomToParents = atom<RoomToParents>(new Map());
export const roomToParentsAtom = atom<RoomToParents, [RoomToParentsAction], undefined>(
  (get) => get(baseRoomToParents),
  (get, set, action) => {
    if (action.type === 'INITIALIZE') {
      set(baseRoomToParents, action.roomToParents);
      return;
    }
    if (action.type === 'PUT') {
      set(
        baseRoomToParents,
        produce(get(baseRoomToParents), (draftRoomToParents) => {
          mapParentWithChildren(draftRoomToParents, action.parent, action.children);
        }),
      );
      return;
    }
    if (action.type === 'DELETE') {
      set(
        baseRoomToParents,
        produce(get(baseRoomToParents), (draftRoomToParents) => {
          const noParentRooms: string[] = [];
          draftRoomToParents.delete(action.roomId);
          draftRoomToParents.forEach((parents, child) => {
            parents.delete(action.roomId);
            if (parents.size === 0) noParentRooms.push(child);
          });
          noParentRooms.forEach((room) => draftRoomToParents.delete(room));
        }),
      );
    }
  },
);

/** Invert native child→parents entries into the product RoomToParents map. */
export const roomToParentsFromNativeSnapshot = (
  entries: NativeSpaceParentEntry[],
): RoomToParents => {
  const map: RoomToParents = new Map();
  for (const entry of entries) {
    if (!entry.parentIds.length) continue;
    map.set(entry.roomId, new Set(entry.parentIds));
  }
  return map;
};

/**
 * Drive space parent map from the native Rust projection.
 * Lobby hierarchy mutations are owned by V-ROOMS.2c native commands.
 */
export const useBindRoomToParentsAtom = (
  roomToParents: typeof roomToParentsAtom = roomToParentsAtom,
) => {
  const setRoomToParents = useSetAtom(roomToParents);

  useEffect(() => {
    let disposed = false;
    let inFlight = false;

    const clear = () => {
      setRoomToParents({ type: 'INITIALIZE', roomToParents: new Map() });
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
        const result = await invokeDesktopWithAvailability<NativeSpaceParentsSnapshot>(
          'matrix_space_parents_snapshot',
        );
        if (!disposed && result.available && result.value) {
          setRoomToParents({
            type: 'INITIALIZE',
            roomToParents: roomToParentsFromNativeSnapshot(result.value.entries),
          });
        }
      } catch {
        // Preserve the last known parent map during transient failures.
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
  }, [setRoomToParents]);
};
