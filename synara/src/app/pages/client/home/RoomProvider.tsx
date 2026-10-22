import React, { ReactNode } from 'react';
import { Navigate, useParams } from 'react-router-dom';
import { useSelectedRoom } from '../../../hooks/router/useSelectedRoom';
import { IsDirectRoomProvider, RoomProvider } from '../../../hooks/useRoom';
import { JoinBeforeNavigate } from '../../../features/join-before-navigate';
import { useHomeRooms } from './useHomeRooms';
import { useSearchParamsViaServers } from '../../../hooks/router/useSearchParamsViaServers';
import { useRoomNavigate } from '../../../hooks/useRoomNavigate';
import { useNativeRoomListSnapshot } from '../../../state/room-list/roomList';
import { isHomeRoomRoutePath } from '../../../hooks/roomRoutePath';

import { getNativeRoom } from '../../../native/nativeSession';

export function HomeRouteRoomProvider({ children }: { children: ReactNode }) {
  const rooms = useHomeRooms();
  const { rooms: roomSummaries } = useNativeRoomListSnapshot();
  const { roomPath } = useRoomNavigate();

  const { roomIdOrAlias, eventId, threadRootId } = useParams();
  const viaServers = useSearchParamsViaServers();
  const roomId = useSelectedRoom();
  const room = getNativeRoom(roomId);

  if (!room || !rooms.includes(room.roomId)) {
    // A joined DM or space room reached through a Home link (an older
    // notification, a reminder, a pasted route) opens where it belongs
    // instead of showing the "View" card.
    const joined =
      !!room &&
      !room.isSpaceRoom() &&
      roomSummaries.some(
        (summary) => summary.roomId === room.roomId && summary.membership === 'join'
      );
    const canonical =
      joined && room
        ? roomPath(room.roomId, threadRootId ? { threadRootId } : { eventId })
        : undefined;
    if (canonical && !isHomeRoomRoutePath(canonical)) {
      return <Navigate to={canonical} replace />;
    }
    return (
      <JoinBeforeNavigate
        roomIdOrAlias={roomIdOrAlias!}
        eventId={eventId}
        threadRootId={threadRootId}
        viaServers={viaServers}
      />
    );
  }

  return (
    <RoomProvider key={room.roomId} value={room}>
      <IsDirectRoomProvider value={false}>{children}</IsDirectRoomProvider>
    </RoomProvider>
  );
}
