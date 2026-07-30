import { atom, useAtomValue, useSetAtom } from 'jotai';
import { useEffect } from 'react';
import { parseRoomSummary, type RoomSummary } from '../../features/matrix-dto/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { RoomsAction } from './utils';

export type NativeRoomListSnapshot = {
  sessionGeneration: number;
  orderedRoomIds: string[];
  rooms: RoomSummary[];
};

const emptyRoomListSnapshot: NativeRoomListSnapshot = {
  sessionGeneration: 0,
  orderedRoomIds: [],
  rooms: [],
};

const nativeRoomListSnapshotAtom = atom<NativeRoomListSnapshot>(emptyRoomListSnapshot);

const baseRoomsAtom = atom<string[]>([]);
export const allRoomsAtom = atom<string[], [RoomsAction], undefined>(
  (get) => get(baseRoomsAtom),
  (get, set, action) => {
    if (action.type === 'INITIALIZE') {
      set(baseRoomsAtom, action.rooms);
      return;
    }
    set(baseRoomsAtom, (ids) => {
      const newIds = ids.filter((id) => id !== action.roomId);
      if (action.type === 'PUT') newIds.push(action.roomId);
      return newIds;
    });
  }
);

export const useNativeRoomListSnapshot = (): NativeRoomListSnapshot =>
  useAtomValue(nativeRoomListSnapshotAtom);

const parseNativeRoomListSnapshot = (value: unknown): NativeRoomListSnapshot | null => {
  if (!value || typeof value !== 'object') return null;
  const record = value as Record<string, unknown>;
  if (typeof record.sessionGeneration !== 'number' || !Number.isFinite(record.sessionGeneration)) {
    return null;
  }
  if (!Array.isArray(record.orderedRoomIds) || !Array.isArray(record.rooms)) {
    return null;
  }
  const orderedRoomIds = record.orderedRoomIds.filter(
    (roomId): roomId is string => typeof roomId === 'string'
  );
  const rooms: RoomSummary[] = [];
  for (const room of record.rooms) {
    const parsed = parseRoomSummary(room);
    if (!parsed) return null;
    rooms.push(parsed);
  }
  return {
    sessionGeneration: record.sessionGeneration,
    orderedRoomIds,
    rooms,
  };
};

/**
 * Sole desktop owner for joined-room ids and unread-bearing room summaries.
 * Dual-backend / JS MatrixClient room-list fallback is forbidden.
 */
export const useBindAllRoomsAtom = () => {
  const setRooms = useSetAtom(allRoomsAtom);
  const setSnapshot = useSetAtom(nativeRoomListSnapshotAtom);

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
          setSnapshot(emptyRoomListSnapshot);
          setRooms({ type: 'INITIALIZE', rooms: [] });
          return;
        }
        const result = await invokeDesktopWithAvailability<unknown>('matrix_room_list_snapshot');
        if (disposed || !result.available || !result.value) return;
        const snapshot = parseNativeRoomListSnapshot(result.value);
        if (!snapshot) return;
        setSnapshot(snapshot);
        setRooms({ type: 'INITIALIZE', rooms: snapshot.orderedRoomIds });
      } catch {
        // Preserve the last known snapshot during a transient sync/protocol failure.
      } finally {
        inFlight = false;
      }
    };

    if (!isSynaraDesktop()) {
      setSnapshot(emptyRoomListSnapshot);
      setRooms({ type: 'INITIALIZE', rooms: [] });
      return undefined;
    }

    void refresh();
    const pollId = window.setInterval(() => void refresh(), 1_000);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, [setRooms, setSnapshot]);
};

type NativeSessionSnapshot = {
  status: 'logged_out' | 'logged_in';
};
