import { useMemo } from 'react';

import { getCanonicalAliasRoomId, isRoomAlias } from '../utils/matrix';

export const useJoinedRoomId = (allRooms: string[], roomIdOrAlias: string): string | undefined => {
  const joinedRoomId = useMemo(() => {
    const roomId = isRoomAlias(roomIdOrAlias)
      ? getCanonicalAliasRoomId(roomIdOrAlias)
      : roomIdOrAlias;

    if (roomId && allRooms.includes(roomId)) return roomId;
    return undefined;
  }, [allRooms, roomIdOrAlias]);

  return joinedRoomId;
};
