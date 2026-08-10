import { Atom, useAtomValue } from 'jotai';
import { selectAtom } from 'jotai/utils';
import type { MatrixClientReading } from '../../utils/room';
import { useCallback, useMemo } from 'react';
import { getAllParents, isRoom, isSpace, isUnsupportedRoom } from '../../utils/room';
import { compareRoomsEqual } from '../room-list/utils';
import { useNativeRoomListSnapshot } from '../room-list/roomList';
import { RoomToParents } from '../../../types/matrix/room';

export type RoomsAtom = Atom<string[]>;
export type RoomSelector = (roomId: string) => boolean | undefined;

export const selectedRoomsAtom = (
  roomsAtom: RoomsAtom,
  selector: (roomId: string) => boolean | undefined,
  nativeSnapshot?: unknown
) =>
  selectAtom(
    roomsAtom,
    (rooms) => {
      // Capture the native revision: summary mutations retain room IDs but
      // must still cause callers to read the updated facade wrapper.
      void nativeSnapshot;
      return rooms.filter(selector);
    },
    compareRoomsEqual
  );

export const useSelectedRooms = (roomsAtom: RoomsAtom, selector: RoomSelector) => {
  // Native room wrappers update their summaries in place. Observe the native
  // projection as a revision source so a same-ID list still re-renders names,
  // avatars, membership, and unread state after a live snapshot.
  const nativeSnapshot = useNativeRoomListSnapshot();
  const anAtom = useMemo(
    () => selectedRoomsAtom(roomsAtom, selector, nativeSnapshot),
    [roomsAtom, selector, nativeSnapshot]
  );

  return useAtomValue(anAtom);
};

export type SpaceChildSelectorFactory = (parentId: string) => RoomSelector;

export const useRecursiveChildScopeFactory = (
  mx: MatrixClientReading,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(mx.getRoom(roomId)) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [mx, roomToParents]
  );

export const useChildSpaceScopeFactory = (
  mx: MatrixClientReading,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isSpace(mx.getRoom(roomId)) && roomToParents.get(roomId)?.has(parentId),
    [mx, roomToParents]
  );

export const useRecursiveChildSpaceScopeFactory = (
  mx: MatrixClientReading,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isSpace(mx.getRoom(roomId)) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [mx, roomToParents]
  );

export const useChildRoomScopeFactory = (
  mx: MatrixClientReading,
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(mx.getRoom(roomId)) &&
      !mDirects.has(roomId) &&
      roomToParents.get(roomId)?.has(parentId),
    [mx, mDirects, roomToParents]
  );

export const useRecursiveChildRoomScopeFactory = (
  mx: MatrixClientReading,
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(mx.getRoom(roomId)) &&
      !mDirects.has(roomId) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [mx, mDirects, roomToParents]
  );

export const useChildDirectScopeFactory = (
  mx: MatrixClientReading,
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(mx.getRoom(roomId)) &&
      mDirects.has(roomId) &&
      roomToParents.get(roomId)?.has(parentId),
    [mx, mDirects, roomToParents]
  );

export const useRecursiveChildDirectScopeFactory = (
  mx: MatrixClientReading,
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(mx.getRoom(roomId)) &&
      mDirects.has(roomId) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [mx, mDirects, roomToParents]
  );

export const useSpaceChildren = (
  roomsAtom: RoomsAtom,
  spaceId: string,
  selectorFactory: SpaceChildSelectorFactory
) => {
  const recursiveChildRoomSelector = useMemo(
    () => selectorFactory(spaceId),
    [selectorFactory, spaceId]
  );
  return useSelectedRooms(roomsAtom, recursiveChildRoomSelector);
};

export const useSpaces = (mx: MatrixClientReading, roomsAtom: RoomsAtom) => {
  const selector: RoomSelector = useCallback((roomId) => isSpace(mx.getRoom(roomId)), [mx]);
  return useSelectedRooms(roomsAtom, selector);
};

export const useOrphanSpaces = (
  mx: MatrixClientReading,
  roomsAtom: RoomsAtom,
  roomToParents: RoomToParents
) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isSpace(mx.getRoom(roomId)) && !roomToParents.has(roomId),
    [mx, roomToParents]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useRooms = (mx: MatrixClientReading, roomsAtom: RoomsAtom, mDirects: Set<string>) => {
  const selector: RoomSelector = useCallback(
    (roomId: string) => isRoom(mx.getRoom(roomId)) && !mDirects.has(roomId),
    [mx, mDirects]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useOrphanRooms = (
  mx: MatrixClientReading,
  roomsAtom: RoomsAtom,
  mDirects: Set<string>,
  roomToParents: RoomToParents
) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isRoom(mx.getRoom(roomId)) && !mDirects.has(roomId) && !roomToParents.has(roomId),
    [mx, mDirects, roomToParents]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useDirects = (
  mx: MatrixClientReading,
  roomsAtom: RoomsAtom,
  mDirects: Set<string>
) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isRoom(mx.getRoom(roomId)) && mDirects.has(roomId),
    [mx, mDirects]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useUnsupportedRooms = (mx: MatrixClientReading, roomsAtom: RoomsAtom) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isUnsupportedRoom(mx.getRoom(roomId)),
    [mx]
  );
  return useSelectedRooms(roomsAtom, selector);
};
