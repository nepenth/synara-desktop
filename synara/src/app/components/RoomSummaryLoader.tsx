import { type ReactNode, useCallback } from 'react';
import { useQuery } from '@tanstack/react-query';
import { AsyncState, AsyncStatus } from '../hooks/useAsyncCallback';
import { fetchNativeSpaceHierarchyLevel, SpaceHierarchyRoom } from '../hooks/useSpaceHierarchy';
import {
  isNativeRoomId,
  readSpaceHierarchyRoomWithNativeOwner,
  type NativeSpaceHierarchyRoom,
} from '../features/lobby/nativeSpaceHierarchyOwner';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';

export type IRoomSummary = {
  room_id: string;
  name?: string;
  canonical_alias?: string;
  topic?: string;
  avatar_url?: string;
  room_type?: string;
  num_joined_members: number;
  join_rule: string;
  world_readable: boolean;
  guest_can_join: boolean;
};

export const toRoomSummaryView = (room: NativeSpaceHierarchyRoom): IRoomSummary => ({
  room_id: room.roomId,
  name: room.name,
  canonical_alias: room.canonicalAlias,
  topic: room.topic,
  avatar_url: room.avatarUrl,
  room_type: room.roomType,
  num_joined_members: room.numJoinedMembers,
  join_rule: room.joinRule,
  world_readable: room.worldReadable,
  guest_can_join: room.guestCanJoin,
});

type RoomSummaryLoaderProps = {
  roomIdOrAlias: string;
  children: (roomSummary: IRoomSummary) => ReactNode;
};

export function RoomSummaryLoader({ roomIdOrAlias, children }: RoomSummaryLoaderProps) {
  const isRoomId = isNativeRoomId(roomIdOrAlias);
  const fetchSummary = useCallback(async () => {
    const room = await readSpaceHierarchyRoomWithNativeOwner(
      roomIdOrAlias,
      isSynaraDesktop(),
      (command, args) => invokeDesktopWithAvailability(command, args)
    );
    return toRoomSummaryView(room);
  }, [roomIdOrAlias]);

  const { data, isError, isFetching } = useQuery({
    enabled: isRoomId,
    queryKey: [roomIdOrAlias, 'native-room-summary'],
    queryFn: fetchSummary,
    retry: false,
    refetchOnWindowFocus: false,
  });

  if (isFetching || isError || !data || data.room_id !== roomIdOrAlias) return null;
  return children(data);
}

export { LocalRoomSummaryLoader } from './LocalRoomSummaryLoader';

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
