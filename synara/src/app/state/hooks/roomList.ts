import { Atom, useAtomValue } from 'jotai';
import { selectAtom } from 'jotai/utils';
import { useCallback, useMemo } from 'react';
import { getAllParents, isRoom, isSpace, isUnsupportedRoom } from '../../utils/room';
import { compareRoomsEqual } from '../room-list/utils';
import { useNativeRoomListSnapshot } from '../room-list/roomList';
import { RoomToParents } from '../../../types/matrix/room';

import { getNativeRoom } from '../../native/nativeSession';
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
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(getNativeRoom(roomId)) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [roomToParents]
  );

export const useChildSpaceScopeFactory = (
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isSpace(getNativeRoom(roomId)) && roomToParents.get(roomId)?.has(parentId),
    [roomToParents]
  );

export const useRecursiveChildSpaceScopeFactory = (
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isSpace(getNativeRoom(roomId)) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [roomToParents]
  );

export const useChildRoomScopeFactory = (
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(getNativeRoom(roomId)) &&
      !mDirects.has(roomId) &&
      roomToParents.get(roomId)?.has(parentId),
    [mDirects, roomToParents]
  );

export const useRecursiveChildRoomScopeFactory = (
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(getNativeRoom(roomId)) &&
      !mDirects.has(roomId) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [mDirects, roomToParents]
  );

export const useChildDirectScopeFactory = (
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(getNativeRoom(roomId)) &&
      mDirects.has(roomId) &&
      roomToParents.get(roomId)?.has(parentId),
    [mDirects, roomToParents]
  );

export const useRecursiveChildDirectScopeFactory = (
  mDirects: Set<string>,
  roomToParents: RoomToParents
): SpaceChildSelectorFactory =>
  useCallback(
    (parentId: string) => (roomId) =>
      isRoom(getNativeRoom(roomId)) &&
      mDirects.has(roomId) &&
      roomToParents.has(roomId) &&
      getAllParents(roomToParents, roomId).has(parentId),
    [mDirects, roomToParents]
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

export const useSpaces = (roomsAtom: RoomsAtom) => {
  const selector: RoomSelector = useCallback((roomId) => isSpace(getNativeRoom(roomId)), []);
  return useSelectedRooms(roomsAtom, selector);
};

export const useOrphanSpaces = (roomsAtom: RoomsAtom, roomToParents: RoomToParents) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isSpace(getNativeRoom(roomId)) && !roomToParents.has(roomId),
    [roomToParents]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useRooms = (roomsAtom: RoomsAtom, mDirects: Set<string>) => {
  const selector: RoomSelector = useCallback(
    (roomId: string) => isRoom(getNativeRoom(roomId)) && !mDirects.has(roomId),
    [mDirects]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useOrphanRooms = (
  roomsAtom: RoomsAtom,
  mDirects: Set<string>,
  roomToParents: RoomToParents
) => {
  const selector: RoomSelector = useCallback(
    (roomId) =>
      isRoom(getNativeRoom(roomId)) && !mDirects.has(roomId) && !roomToParents.has(roomId),
    [mDirects, roomToParents]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useDirects = (roomsAtom: RoomsAtom, mDirects: Set<string>) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isRoom(getNativeRoom(roomId)) && mDirects.has(roomId),
    [mDirects]
  );
  return useSelectedRooms(roomsAtom, selector);
};

export const useUnsupportedRooms = (roomsAtom: RoomsAtom) => {
  const selector: RoomSelector = useCallback(
    (roomId) => isUnsupportedRoom(getNativeRoom(roomId)),
    []
  );
  return useSelectedRooms(roomsAtom, selector);
};
