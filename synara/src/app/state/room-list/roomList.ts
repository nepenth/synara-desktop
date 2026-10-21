import { atom, useAtomValue, useSetAtom, type Atom } from 'jotai';
import { selectAtom } from 'jotai/utils';
import { useEffect } from 'react';
import type { Unread } from '../../../types/matrix/room';
import { parseRoomSummary, type RoomSummary } from '../../features/matrix-dto/room';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { RoomsAction } from './utils';
import { startRoomListUpdateDrivenPoll } from '../../utils/nativeRoomListUpdates';
import {
  createCoalescedRefresh,
  readNativeSessionSnapshotShared,
} from '../../utils/nativeListRefresh';
import type { RoomListPresentation } from '../../features/matrix-dto/generated';
import {
  EMPTY_ROOM_LIST_PRESENTATION,
  parseRoomListPresentation,
  unreadFromPresentation,
} from './roomListPresentation';

export type NativeRoomListSnapshot = {
  sessionGeneration: number;
  orderedRoomIds: string[];
  rooms: RoomSummary[];
  /** Section orders, favorites and unread attention, derived by Core. */
  presentation: RoomListPresentation;
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
  presentation: EMPTY_ROOM_LIST_PRESENTATION,
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
  left.stateEncrypted === right.stateEncrypted &&
  left.isEncrypted === right.isEncrypted &&
  left.isCall === right.isCall &&
  left.canonicalAlias === right.canonicalAlias &&
  left.lastMessageIsAgentApproval === right.lastMessageIsAgentApproval &&
  left.tombstoneSuccessorRoomId === right.tombstoneSuccessorRoomId &&
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
        hero.displayName === rightHeroes[index]?.displayName &&
        hero.statusEmoji === rightHeroes[index]?.statusEmoji
    )
  );
};

export const sameOptionalRoomSummary = (
  left: RoomSummary | undefined,
  right: RoomSummary | undefined
): boolean => left === right || (!!left && !!right && sameRoomSummary(left, right));

const sameUnread = (left: Unread | undefined, right: Unread | undefined): boolean => {
  if (left === right) return true;
  if (!left || !right) return false;
  if (left.highlight !== right.highlight || left.total !== right.total) return false;
  if (left.from === right.from) return true;
  if (!left.from || !right.from || left.from.size !== right.from.size) return false;
  return [...left.from].every((roomId) => right.from?.has(roomId));
};

export const sameNativeRoomListSnapshot = (
  left: NativeRoomListSnapshot,
  right: NativeRoomListSnapshot
): boolean => {
  if (left.sessionGeneration !== right.sessionGeneration) return false;
  if (!sameStringList(left.orderedRoomIds, right.orderedRoomIds)) return false;
  if (left.rooms.length !== right.rooms.length) return false;
  if (JSON.stringify(left.presentation) !== JSON.stringify(right.presentation)) return false;
  return left.rooms.every((room, index) => sameRoomSummary(room, right.rooms[index]));
};

/** Written only by `useBindAllRoomsAtom`; exported for tests. */
export const nativeRoomListSnapshotAtom = atom<NativeRoomListSnapshot>(emptyRoomListSnapshot);
let latestNativeRoomListSnapshot = emptyRoomListSnapshot;
/**
 * False until the first room-list readback (or a signed-out session) since
 * the app started, so an empty list before then reads as loading, not as
 * "no rooms".
 */
const nativeRoomListLoadedAtom = atom(false);
let loadedSessionGeneration: number | undefined;

export const useNativeRoomListLoaded = (): boolean => useAtomValue(nativeRoomListLoadedAtom);

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

/**
 * The whole projection. Every room's activity changes it, so a component that
 * needs one room should use `useNativeRoomSummary` / `useNativeRoomUnread`.
 */
export const useNativeRoomListSnapshot = (): NativeRoomListSnapshot =>
  useAtomValue(nativeRoomListSnapshotAtom);

/** Summaries by room id, built once per published snapshot. */
const nativeRoomSummaryByIdAtom = atom(
  (get) => new Map(get(nativeRoomListSnapshotAtom).rooms.map((room) => [room.roomId, room]))
);

/** Per-room atoms, so every row of one room shares a single subscription. */
const roomSummaryAtoms = new Map<string, Atom<RoomSummary | undefined>>();
const roomUnreadAtoms = new Map<string, Atom<Unread | undefined>>();

export const nativeRoomSummaryAtom = (roomId: string): Atom<RoomSummary | undefined> => {
  let summaryAtom = roomSummaryAtoms.get(roomId);
  if (!summaryAtom) {
    summaryAtom = selectAtom(
      nativeRoomSummaryByIdAtom,
      (rooms) => rooms.get(roomId),
      sameOptionalRoomSummary
    );
    roomSummaryAtoms.set(roomId, summaryAtom);
  }
  return summaryAtom;
};

export const nativeRoomUnreadAtom = (roomId: string): Atom<Unread | undefined> => {
  let unreadAtom = roomUnreadAtoms.get(roomId);
  if (!unreadAtom) {
    unreadAtom = selectAtom(
      nativeRoomListSnapshotAtom,
      (snapshot) => unreadFromPresentation(snapshot.presentation, roomId),
      sameUnread
    );
    roomUnreadAtoms.set(roomId, unreadAtom);
  }
  return unreadAtom;
};

const NO_ROOM_ATOM = atom<undefined>(undefined);

/**
 * One room's summary. Re-renders only when that room's summary changes, not on
 * every other room's activity (D10).
 */
export const useNativeRoomSummary = (roomId: string | undefined): RoomSummary | undefined =>
  useAtomValue(roomId ? nativeRoomSummaryAtom(roomId) : NO_ROOM_ATOM);

/** One room's (or space's) unread attention from Core's presentation. */
export const useNativeRoomUnread = (roomId: string | undefined): Unread | undefined =>
  useAtomValue(roomId ? nativeRoomUnreadAtom(roomId) : NO_ROOM_ATOM);

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
  const presentation = parseRoomListPresentation(record.presentation);
  if (!presentation) return null;
  return {
    sessionGeneration: record.sessionGeneration,
    orderedRoomIds,
    rooms,
    presentation,
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
  const setLoaded = useSetAtom(nativeRoomListLoadedAtom);

  useEffect(() => {
    let disposed = false;

    // A wake during a refresh re-runs it once it finishes (D8), so the last
    // change of a burst is never left for the safety poll.
    const refresh = createCoalescedRefresh(
      async () => {
        try {
          const sessionResult = await readNativeSessionSnapshotShared<unknown>();
          if (disposed || !sessionResult.available) return;
          const session = parseNativeSessionSnapshot(sessionResult.value);
          if (!session) return;
          onSessionSnapshot?.(session);
          if (session.status !== 'logged_in') {
            latestNativeRoomListSnapshot = emptyRoomListSnapshot;
            setSnapshot(emptyRoomListSnapshot);
            setRooms({ type: 'INITIALIZE', rooms: [] });
            loadedSessionGeneration = undefined;
            setLoaded(true);
            return;
          }
          // A new sign-in has not listed its rooms yet.
          if (loadedSessionGeneration !== session.sessionGeneration) setLoaded(false);
          const result = await invokeDesktopWithAvailability<unknown>('matrix_room_list_snapshot');
          if (disposed || !result.available || !result.value) return;
          const snapshot = parseNativeRoomListSnapshot(result.value);
          if (!snapshot || snapshot.sessionGeneration !== session.sessionGeneration) return;
          loadedSessionGeneration = snapshot.sessionGeneration;
          setLoaded(true);
          if (sameNativeRoomListSnapshot(latestNativeRoomListSnapshot, snapshot)) return;
          latestNativeRoomListSnapshot = snapshot;
          // Hydrate the synchronous facade before either atom setter can schedule
          // selectors that combine a fresh room id with getNativeRoom().
          onSnapshot?.(snapshot);
          setSnapshot(snapshot);
          setRooms({ type: 'INITIALIZE', rooms: snapshot.orderedRoomIds });
        } catch {
          // Preserve the last known snapshot during a transient sync/protocol failure.
        }
      },
      () => disposed
    );

    if (!isSynaraDesktop()) {
      latestNativeRoomListSnapshot = emptyRoomListSnapshot;
      setSnapshot(emptyRoomListSnapshot);
      setRooms({ type: 'INITIALIZE', rooms: [] });
      setLoaded(true);
      return undefined;
    }

    void refresh();
    const stopPolling = startRoomListUpdateDrivenPoll(() => void refresh());
    return () => {
      disposed = true;
      stopPolling();
    };
  }, [onSessionSnapshot, onSnapshot, setLoaded, setRooms, setSnapshot]);
};
