import { produce } from 'immer';
import { atom, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { RoomToParents } from '../../../types/matrix/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { mapParentWithChildren } from '../../utils/room';
import { startRoomListUpdateDrivenPoll } from '../../utils/nativeRoomListUpdates';
import {
  createCoalescedRefresh,
  readNativeSessionSnapshotShared,
} from '../../utils/nativeListRefresh';

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
const sameRoomToParents = (left: RoomToParents, right: RoomToParents): boolean => {
  if (left.size !== right.size) return false;
  for (const [roomId, parents] of left) {
    const other = right.get(roomId);
    if (!other || other.size !== parents.size) return false;
    for (const parentId of parents) if (!other.has(parentId)) return false;
  }
  return true;
};

export const roomToParentsAtom = atom<RoomToParents, [RoomToParentsAction], undefined>(
  (get) => get(baseRoomToParents),
  (get, set, action) => {
    if (action.type === 'INITIALIZE') {
      // An unchanged map keeps its identity, so space and room-list consumers
      // do not re-render on every room-list wake.
      if (sameRoomToParents(get(baseRoomToParents), action.roomToParents)) return;
      set(baseRoomToParents, action.roomToParents);
      return;
    }
    if (action.type === 'PUT') {
      set(
        baseRoomToParents,
        produce(get(baseRoomToParents), (draftRoomToParents) => {
          mapParentWithChildren(draftRoomToParents, action.parent, action.children);
        })
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
        })
      );
    }
  }
);

/** Invert native child→parents entries into the product RoomToParents map. */
export const roomToParentsFromNativeSnapshot = (
  entries: NativeSpaceParentEntry[]
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
  roomToParents: typeof roomToParentsAtom = roomToParentsAtom
) => {
  const setRoomToParents = useSetAtom(roomToParents);

  useEffect(() => {
    let disposed = false;

    const clear = () => {
      setRoomToParents({ type: 'INITIALIZE', roomToParents: new Map() });
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
          const result = await invokeDesktopWithAvailability<NativeSpaceParentsSnapshot>(
            'matrix_space_parents_snapshot'
          );
          if (!disposed && result.available && result.value) {
            setRoomToParents({
              type: 'INITIALIZE',
              roomToParents: roomToParentsFromNativeSnapshot(result.value.entries),
            });
          }
        } catch {
          // Preserve the last known parent map during transient failures.
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
  }, [setRoomToParents]);
};
