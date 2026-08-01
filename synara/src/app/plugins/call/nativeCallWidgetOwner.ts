import type { NativeRoomListSnapshot } from '../../state/room-list/roomList';

export const getKnownRoomsFromNativeSnapshot = (
  desktopAvailable: boolean,
  snapshot: NativeRoomListSnapshot
): string[] => {
  if (!desktopAvailable || snapshot.sessionGeneration <= 0) return [];
  return [...snapshot.orderedRoomIds];
};
