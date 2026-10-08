import React, { ReactNode } from 'react';
import { useParams } from 'react-router-dom';
import { useSelectedRoom } from '../../../hooks/router/useSelectedRoom';
import { IsDirectRoomProvider, RoomProvider } from '../../../hooks/useRoom';
import { JoinBeforeNavigate } from '../../../features/join-before-navigate';
import { useDirectRooms } from './useDirectRooms';

import { getNativeRoom } from '../../../native/nativeSession';
export function DirectRouteRoomProvider({ children }: { children: ReactNode }) {
  const rooms = useDirectRooms();

  const { roomIdOrAlias, eventId, threadRootId } = useParams();
  const roomId = useSelectedRoom();
  const room = getNativeRoom(roomId);

  if (!room || !rooms.includes(room.roomId)) {
    return (
      <JoinBeforeNavigate
        roomIdOrAlias={roomIdOrAlias!}
        eventId={eventId}
        threadRootId={threadRootId}
      />
    );
  }

  return (
    <RoomProvider key={room.roomId} value={room}>
      <IsDirectRoomProvider value>{children}</IsDirectRoomProvider>
    </RoomProvider>
  );
}
