import { ReactNode, useCallback } from 'react';
import { MatrixClient, Room } from 'matrix-js-sdk';
import { useQuery } from '@tanstack/react-query';
import { useMatrixClient } from '../hooks/useMatrixClient';
import { LocalRoomSummary, useLocalRoomSummary } from '../hooks/useLocalRoomSummary';
import { AsyncState, AsyncStatus } from '../hooks/useAsyncCallback';
import { fetchNativeSpaceHierarchyLevel, SpaceHierarchyRoom } from '../hooks/useSpaceHierarchy';

export type IRoomSummary = Awaited<ReturnType<MatrixClient['getRoomSummary']>>;

type RoomSummaryLoaderProps = {
  roomIdOrAlias: string;
  children: (roomSummary?: IRoomSummary) => ReactNode;
};

export function RoomSummaryLoader({ roomIdOrAlias, children }: RoomSummaryLoaderProps) {
  const mx = useMatrixClient();

  const fetchSummary = useCallback(() => mx.getRoomSummary(roomIdOrAlias), [mx, roomIdOrAlias]);

  const { data } = useQuery({
    queryKey: [roomIdOrAlias, `summary`],
    queryFn: fetchSummary,
  });

  return children(data);
}

export function LocalRoomSummaryLoader({
  room,
  children,
}: {
  room: Room;
  children: (roomSummary: LocalRoomSummary) => ReactNode;
}) {
  const summary = useLocalRoomSummary(room);

  return children(summary);
}

export function HierarchyRoomSummaryLoader({
  roomId,
  children,
}: {
  roomId: string;
  children: (state: AsyncState<SpaceHierarchyRoom, Error>) => ReactNode;
}) {
  const fetchSummary = useCallback(async () => {
    const rooms = await fetchNativeSpaceHierarchyLevel(roomId);
    const summary = rooms.find((room) => room.room_id === roomId);
    if (!summary) throw new Error('Native Matrix room summary is unavailable.');
    return summary;
  }, [roomId]);

  const { data, error } = useQuery({
    queryKey: [roomId, `hierarchy`],
    queryFn: fetchSummary,
    retryOnMount: false,
    refetchOnWindowFocus: false,
    retry: 3,
  });

  let state: AsyncState<SpaceHierarchyRoom, Error> = {
    status: AsyncStatus.Loading,
  };
  if (error) {
    state = {
      status: AsyncStatus.Error,
      error,
    };
  }
  if (data) {
    state = {
      status: AsyncStatus.Success,
      data,
    };
  }

  return children(state);
}
