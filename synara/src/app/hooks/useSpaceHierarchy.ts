import { atom, useAtom, useAtomValue } from 'jotai';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { JoinRule, Room } from 'matrix-js-sdk';
import { useQuery } from '@tanstack/react-query';
import { useMatrixClient } from './useMatrixClient';
import { roomToParentsAtom } from '../state/room/roomToParents';
import { MSpaceChildContent, StateEvent } from '../../types/matrix/room';
import { getAllParents, getStateEvents, isValidChild } from '../utils/room';
import { isRoomId } from '../utils/matrix';
import { SortFunc, byOrderKey, byTsOldToNew, factoryRoomIdByActivity } from '../utils/sort';
import { useStateEventCallback } from './useStateEventCallback';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import {
  NativeSpaceHierarchyRoom,
  readSpaceHierarchyWithNativeOwner,
} from '../features/lobby/nativeSpaceHierarchyOwner';

export type HierarchyItemSpace = {
  roomId: string;
  content: MSpaceChildContent;
  ts: number;
  space: true;
  parentId?: string;
};

export type HierarchyItemRoom = {
  roomId: string;
  content: MSpaceChildContent;
  ts: number;
  parentId: string;
};

export type HierarchyItem = HierarchyItemSpace | HierarchyItemRoom;

type GetRoomCallback = (roomId: string) => Room | undefined;

const hierarchyItemTs: SortFunc<HierarchyItem> = (a, b) => byTsOldToNew(a.ts, b.ts);
const hierarchyItemByOrder: SortFunc<HierarchyItem> = (a, b) =>
  byOrderKey(a.content.order, b.content.order);

const getHierarchySpaces = (
  rootSpaceId: string,
  getRoom: GetRoomCallback,
  spaceRooms: Set<string>
): HierarchyItemSpace[] => {
  const rootSpaceItem: HierarchyItemSpace = {
    roomId: rootSpaceId,
    content: { via: [] },
    ts: 0,
    space: true,
  };
  let spaceItems: HierarchyItemSpace[] = [];

  const findAndCollectHierarchySpaces = (spaceItem: HierarchyItemSpace) => {
    if (spaceItems.find((item) => item.roomId === spaceItem.roomId)) return;
    const space = getRoom(spaceItem.roomId);
    spaceItems.push(spaceItem);

    if (!space) return;
    const childEvents = getStateEvents(space, StateEvent.SpaceChild);

    childEvents.forEach((childEvent) => {
      if (!isValidChild(childEvent)) return;
      const childId = childEvent.getStateKey();
      if (!childId || !isRoomId(childId)) return;

      // because we can not find if a childId is space without joining
      // or requesting room summary, we will look it into spaceRooms local
      // cache which we maintain as we load summary in UI.
      if (getRoom(childId)?.isSpaceRoom() || spaceRooms.has(childId)) {
        const childItem: HierarchyItemSpace = {
          roomId: childId,
          content: childEvent.getContent<MSpaceChildContent>(),
          ts: childEvent.getTs(),
          space: true,
          parentId: spaceItem.roomId,
        };
        findAndCollectHierarchySpaces(childItem);
      }
    });
  };
  findAndCollectHierarchySpaces(rootSpaceItem);

  spaceItems = [
    rootSpaceItem,
    ...spaceItems
      .filter((item) => item.roomId !== rootSpaceId)
      .sort(hierarchyItemTs)
      .sort(hierarchyItemByOrder),
  ];

  return spaceItems;
};

export type SpaceHierarchy = {
  space: HierarchyItemSpace;
  rooms?: HierarchyItemRoom[];
};
const getSpaceHierarchy = (
  rootSpaceId: string,
  spaceRooms: Set<string>,
  getRoom: (roomId: string) => Room | undefined,
  closedCategory: (spaceId: string) => boolean
): SpaceHierarchy[] => {
  const spaceItems: HierarchyItemSpace[] = getHierarchySpaces(rootSpaceId, getRoom, spaceRooms);

  const hierarchy: SpaceHierarchy[] = spaceItems.map((spaceItem) => {
    const space = getRoom(spaceItem.roomId);
    if (!space || closedCategory(spaceItem.roomId)) {
      return {
        space: spaceItem,
      };
    }
    const childEvents = getStateEvents(space, StateEvent.SpaceChild);
    const childItems: HierarchyItemRoom[] = [];
    childEvents.forEach((childEvent) => {
      if (!isValidChild(childEvent)) return;
      const childId = childEvent.getStateKey();
      if (!childId || !isRoomId(childId)) return;
      if (getRoom(childId)?.isSpaceRoom() || spaceRooms.has(childId)) return;

      const childItem: HierarchyItemRoom = {
        roomId: childId,
        content: childEvent.getContent<MSpaceChildContent>(),
        ts: childEvent.getTs(),
        parentId: spaceItem.roomId,
      };
      childItems.push(childItem);
    });

    return {
      space: spaceItem,
      rooms: childItems.sort(hierarchyItemTs).sort(hierarchyItemByOrder),
    };
  });

  return hierarchy;
};

export const useSpaceHierarchy = (
  spaceId: string,
  spaceRooms: Set<string>,
  getRoom: (roomId: string) => Room | undefined,
  closedCategory: (spaceId: string) => boolean
): SpaceHierarchy[] => {
  const mx = useMatrixClient();
  const roomToParents = useAtomValue(roomToParentsAtom);

  const [hierarchyAtom] = useState(() =>
    atom(getSpaceHierarchy(spaceId, spaceRooms, getRoom, closedCategory))
  );
  const [hierarchy, setHierarchy] = useAtom(hierarchyAtom);

  useEffect(() => {
    setHierarchy(getSpaceHierarchy(spaceId, spaceRooms, getRoom, closedCategory));
  }, [mx, spaceId, spaceRooms, setHierarchy, getRoom, closedCategory]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (mEvent.getType() !== StateEvent.SpaceChild) return;
        const eventRoomId = mEvent.getRoomId();
        if (!eventRoomId) return;

        if (spaceId === eventRoomId || getAllParents(roomToParents, eventRoomId).has(spaceId)) {
          setHierarchy(getSpaceHierarchy(spaceId, spaceRooms, getRoom, closedCategory));
        }
      },
      [spaceId, roomToParents, setHierarchy, spaceRooms, getRoom, closedCategory]
    )
  );

  return hierarchy;
};

const getSpaceJoinedHierarchy = (
  rootSpaceId: string,
  getRoom: GetRoomCallback,
  excludeRoom: (parentId: string, roomId: string) => boolean,
  sortRoomItems: (parentId: string, items: HierarchyItem[]) => HierarchyItem[]
): HierarchyItem[] => {
  const spaceItems: HierarchyItemSpace[] = getHierarchySpaces(rootSpaceId, getRoom, new Set());

  const hierarchy: HierarchyItem[] = spaceItems.flatMap((spaceItem) => {
    const space = getRoom(spaceItem.roomId);
    if (!space) {
      return [];
    }
    const joinedRoomEvents = getStateEvents(space, StateEvent.SpaceChild).filter((childEvent) => {
      if (!isValidChild(childEvent)) return false;
      const childId = childEvent.getStateKey();
      if (!childId || !isRoomId(childId)) return false;
      const room = getRoom(childId);
      if (!room || room.isSpaceRoom()) return false;

      return true;
    });

    if (joinedRoomEvents.length === 0) return [];

    const childItems: HierarchyItemRoom[] = [];
    joinedRoomEvents.forEach((childEvent) => {
      const childId = childEvent.getStateKey();
      if (!childId) return;

      if (excludeRoom(space.roomId, childId)) return;

      const childItem: HierarchyItemRoom = {
        roomId: childId,
        content: childEvent.getContent<MSpaceChildContent>(),
        ts: childEvent.getTs(),
        parentId: spaceItem.roomId,
      };
      childItems.push(childItem);
    });
    return [spaceItem, ...sortRoomItems(spaceItem.roomId, childItems)];
  });

  return hierarchy;
};

export const useSpaceJoinedHierarchy = (
  spaceId: string,
  getRoom: GetRoomCallback,
  excludeRoom: (parentId: string, roomId: string) => boolean,
  sortByActivity: (spaceId: string) => boolean
): HierarchyItem[] => {
  const mx = useMatrixClient();
  const roomToParents = useAtomValue(roomToParentsAtom);

  const sortRoomItems = useCallback(
    (sId: string, items: HierarchyItem[]) => {
      if (sortByActivity(sId)) {
        items.sort((a, b) => factoryRoomIdByActivity(mx)(a.roomId, b.roomId));
        return items;
      }
      items.sort(hierarchyItemTs).sort(hierarchyItemByOrder);
      return items;
    },
    [mx, sortByActivity]
  );

  const [hierarchyAtom] = useState(() =>
    atom(getSpaceJoinedHierarchy(spaceId, getRoom, excludeRoom, sortRoomItems))
  );
  const [hierarchy, setHierarchy] = useAtom(hierarchyAtom);

  useEffect(() => {
    setHierarchy(getSpaceJoinedHierarchy(spaceId, getRoom, excludeRoom, sortRoomItems));
  }, [mx, spaceId, setHierarchy, getRoom, excludeRoom, sortRoomItems]);

  useStateEventCallback(
    mx,
    useCallback(
      (mEvent) => {
        if (mEvent.getType() !== StateEvent.SpaceChild) return;
        const eventRoomId = mEvent.getRoomId();
        if (!eventRoomId) return;

        if (spaceId === eventRoomId || getAllParents(roomToParents, eventRoomId).has(spaceId)) {
          setHierarchy(getSpaceJoinedHierarchy(spaceId, getRoom, excludeRoom, sortRoomItems));
        }
      },
      [spaceId, roomToParents, setHierarchy, getRoom, excludeRoom, sortRoomItems]
    )
  );

  return hierarchy;
};

export type SpaceHierarchyRoom = {
  room_id: string;
  name?: string;
  canonical_alias?: string;
  topic?: string;
  avatar_url?: string;
  room_type?: string;
  num_joined_members: number;
  join_rule: JoinRule;
  world_readable: boolean;
  guest_can_join: boolean;
};

const toSpaceHierarchyRoom = (room: NativeSpaceHierarchyRoom): SpaceHierarchyRoom => ({
  room_id: room.roomId,
  name: room.name,
  canonical_alias: room.canonicalAlias,
  topic: room.topic,
  avatar_url: room.avatarUrl,
  room_type: room.roomType,
  num_joined_members: room.numJoinedMembers,
  join_rule: room.joinRule as JoinRule,
  world_readable: room.worldReadable,
  guest_can_join: room.guestCanJoin,
});

export async function fetchNativeSpaceHierarchyLevel(
  roomId: string
): Promise<SpaceHierarchyRoom[]> {
  const result = await readSpaceHierarchyWithNativeOwner(
    roomId,
    isSynaraDesktop(),
    (command, args) => invokeDesktopWithAvailability(command, args)
  );
  return result.rooms.map(toSpaceHierarchyRoom);
}

export type FetchSpaceHierarchyLevelData = {
  fetching: boolean;
  error: Error | null;
  rooms: Map<string, SpaceHierarchyRoom>;
};
export const useFetchSpaceHierarchyLevel = (
  roomId: string,
  enable: boolean
): FetchSpaceHierarchyLevelData => {
  const queryResponse = useQuery({
    enabled: enable,
    refetchOnMount: enable ? 'always' : false,
    queryKey: [roomId, 'hierarchy_level'],
    queryFn: () => fetchNativeSpaceHierarchyLevel(roomId),
    retry: 5,
    retryDelay: (failureCount) => 500 * failureCount,
  });

  const { data, isLoading, isFetching, error } = queryResponse;

  const rooms: Map<string, SpaceHierarchyRoom> = useMemo(() => {
    const roomsMap: Map<string, SpaceHierarchyRoom> = new Map();
    if (!data) return roomsMap;
    data.forEach((r) => {
      roomsMap.set(r.room_id, r);
    });
    return roomsMap;
  }, [data]);

  const fetching = isLoading || isFetching;

  return {
    fetching,
    error,
    rooms,
  };
};
