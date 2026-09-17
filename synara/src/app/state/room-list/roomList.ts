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

export type NativeSessionSnapshot =
  | { status: 'logged_out' }
  | {
      status: 'logged_in';
      userId: string;
      deviceId: string;
      homeserverUrl: string;
      sessionGeneration: number;
    };

const emptyRoomListSnapshot: NativeRoomListSnapshot = {
  sessionGeneration: 0,
  orderedRoomIds: [],
  rooms: [],
};

export const sameStringList = (left: readonly string[], right: readonly string[]): boolean =>
  left.length === right.length && left.every((value, index) => value === right[index]);

const sameRoomSummary = (left: RoomSummary, right: RoomSummary): boolean =>
  left.roomId === right.roomId &&
  left.unreadCount === right.unreadCount &&
  left.highlightCount === right.highlightCount &&
  left.markedUnread === right.markedUnread &&
  left.name === right.name &&
  left.lastActivityTs === right.lastActivityTs &&
  left.lastMessagePreview === right.lastMessagePreview &&
  left.hasActiveCall === right.hasActiveCall &&
  left.activeCallParticipantCount === right.activeCallParticipantCount &&
  left.membership === right.membership &&
  left.isFavorite === right.isFavorite &&
  left.notificationMode === right.notificationMode &&
  left.avatarUrl === right.avatarUrl &&
  left.directUserId === right.directUserId &&
  left.isDirect === right.isDirect &&
  left.isSpace === right.isSpace &&
  left.encryptionStatus === right.encryptionStatus &&
  left.joinRule === right.joinRule &&
  sameHeroes(left.heroes, right.heroes);

const sameHeroes = (left: RoomSummary['heroes'], right: RoomSummary['heroes']): boolean => {
  const leftHeroes = left ?? [];
  const rightHeroes = right ?? [];
  return (
    leftHeroes.length === rightHeroes.length &&
    leftHeroes.every(
      (hero, index) =>
        hero.userId === rightHeroes[index]?.userId &&
        hero.inCall === rightHeroes[index]?.inCall &&
        hero.displayName === rightHeroes[index]?.displayName
    )
  );
};

export const sameNativeRoomListSnapshot = (
  left: NativeRoomListSnapshot,
  right: NativeRoomListSnapshot
): boolean => {
  if (left.sessionGeneration !== right.sessionGeneration) return false;
  if (!sameStringList(left.orderedRoomIds, right.orderedRoomIds)) return false;
  if (left.rooms.length !== right.rooms.length) return false;
  return left.rooms.every((room, index) => sameRoomSummary(room, right.rooms[index]));
};

const nativeRoomListSnapshotAtom = atom<NativeRoomListSnapshot>(emptyRoomListSnapshot);
let latestNativeRoomListSnapshot = emptyRoomListSnapshot;

const baseRoomsAtom = atom<string[]>([]);
export const allRoomsAtom = atom<string[], [RoomsAction], undefined>(
  (get) => get(baseRoomsAtom),
  (get, set, action) => {
    if (action.type === 'INITIALIZE') {
      const current = get(baseRoomsAtom);
      if (sameStringList(current, action.rooms)) return;
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

/**
 * Read the latest native snapshot outside React, for synchronous widget APIs.
 * An empty snapshot means that no native room-list readback is available yet.
 */
export const getNativeRoomListSnapshot = (): NativeRoomListSnapshot => latestNativeRoomListSnapshot;

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

const parseNativeSessionSnapshot = (value: unknown): NativeSessionSnapshot | null => {
  if (!value || typeof value !== 'object') return null;
  const record = value as Record<string, unknown>;
  if (record.status === 'logged_out') return { status: 'logged_out' };
  if (record.status !== 'logged_in') return null;
  const { user_id: userId, device_id: deviceId, homeserver_url: homeserverUrl } = record;
  const sessionGeneration = record.sessionGeneration;
  if (
    typeof userId !== 'string' ||
    typeof deviceId !== 'string' ||
    typeof homeserverUrl !== 'string' ||
    typeof sessionGeneration !== 'number' ||
    !Number.isSafeInteger(sessionGeneration) ||
    sessionGeneration < 0
  ) {
    return null;
  }
  return { status: 'logged_in', userId, deviceId, homeserverUrl, sessionGeneration };
};

/**
 * Sole desktop owner for joined-room ids and unread-bearing room summaries.
 * Dual-backend / JS MatrixClient room-list fallback is forbidden.
 */
export const useBindAllRoomsAtom = (
  onSnapshot?: (snapshot: NativeRoomListSnapshot) => void,
  onSessionSnapshot?: (snapshot: NativeSessionSnapshot) => void
) => {
  const setRooms = useSetAtom(allRoomsAtom);
  const setSnapshot = useSetAtom(nativeRoomListSnapshotAtom);

  useEffect(() => {
    let disposed = false;
    let inFlight = false;

    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const sessionResult = await invokeDesktopWithAvailability<unknown>(
          'matrix_session_snapshot'
        );
        if (disposed || !sessionResult.available) return;
        const session = parseNativeSessionSnapshot(sessionResult.value);
        if (!session) return;
        onSessionSnapshot?.(session);
        if (session.status !== 'logged_in') {
          latestNativeRoomListSnapshot = emptyRoomListSnapshot;
          setSnapshot(emptyRoomListSnapshot);
          setRooms({ type: 'INITIALIZE', rooms: [] });
          return;
        }
        const result = await invokeDesktopWithAvailability<unknown>('matrix_room_list_snapshot');
        if (disposed || !result.available || !result.value) return;
        const snapshot = parseNativeRoomListSnapshot(result.value);
        if (!snapshot || snapshot.sessionGeneration !== session.sessionGeneration) return;
        if (sameNativeRoomListSnapshot(latestNativeRoomListSnapshot, snapshot)) return;
        latestNativeRoomListSnapshot = snapshot;
        // Hydrate the synchronous facade before either atom setter can schedule
        // selectors that combine a fresh room id with mx.getRoom().
        onSnapshot?.(snapshot);
        setSnapshot(snapshot);
        setRooms({ type: 'INITIALIZE', rooms: snapshot.orderedRoomIds });
      } catch {
        // Preserve the last known snapshot during a transient sync/protocol failure.
      } finally {
        inFlight = false;
      }
    };

    if (!isSynaraDesktop()) {
      latestNativeRoomListSnapshot = emptyRoomListSnapshot;
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
  }, [onSessionSnapshot, onSnapshot, setRooms, setSnapshot]);
};
