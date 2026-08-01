import { useMemo } from 'react';
import { getAllParents } from '../utils/room';
import { RoomToParents } from '../../types/matrix/room';

/**
 * V-SEND.R-PACK-READ: candidate pack room IDs = room + parent spaces.
 * Pure graph resolution from roomToParents — no live `mx.getRoom` (JS client).
 * Consumers load packs via native `matrix_get_room_image_packs` by room id.
 */
export const useImagePackRooms = (roomId: string, roomToParents: RoomToParents): string[] => {
  return useMemo(() => {
    const parents = Array.from(getAllParents(roomToParents, roomId));
    return [roomId].concat(parents);
  }, [roomId, roomToParents]);
};
