import { atom, useAtom } from 'jotai';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { JoinRule, Room } from 'matrix-js-sdk';
import { useQuery } from '@tanstack/react-query';
import { useMatrixClient } from './useMatrixClient';
import { MSpaceChildContent } from '../../types/matrix/room';
import { isRoomId } from '../utils/matrix';
import { SortFunc, byOrderKey, byTsOldToNew, factoryRoomIdByActivity } from '../utils/sort';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../utils/desktop';
import {
  NativeSpaceHierarchyRoom,
  readSpaceHierarchyWithNativeOwner,
} from '../features/lobby/nativeSpaceHierarchyOwner';
import {
  NativeSpaceChildEdge,
  readSpaceChildrenWithNativeOwner,
  spaceChildContentFromEdge,
} from '../features/lobby/nativeSpaceChildOwner';

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

type ChildEdge = {
  childId: string;
  content: MSpaceChildContent;
  ts: number;
};

/** parentId → valid child edges from the native local graph. */
export type SpaceChildEdgeMap = Map<string, ChildEdge[]>;

const hierarchyItemTs: SortFunc<HierarchyItem> = (a, b) => byTsOldToNew(a.ts, b.ts);
const hierarchyItemByOrder: SortFunc<HierarchyItem> = (a, b) =>
  byOrderKey(a.content.order, b.content.order);

export const spaceChildEdgeMapFromNative = (edges: NativeSpaceChildEdge[]): SpaceChildEdgeMap => {
  const map: SpaceChildEdgeMap = new Map();
  for (const edge of edges) {
    if (!isRoomId(edge.childId) || !isRoomId(edge.parentId)) continue;
    // Match legacy isValidChild: content must expose a via array.
    if (!Array.isArray(edge.via)) continue;
    const list = map.get(edge.parentId) ?? [];
    list.push({
      childId: edge.childId,
      content: spaceChildContentFromEdge(edge),
      ts: edge.originServerTs,
    });
    map.set(edge.parentId, list);
  }
  return map;
};

const edgesForParent = (edgeMap: SpaceChildEdgeMap, parentId: string): ChildEdge[] =>
  edgeMap.get(parentId) ?? [];

const getHierarchySpaces = (
  rootSpaceId: string,
  getRoom: GetRoomCallback,
  spaceRooms: Set<string>,
  edgeMap: SpaceChildEdgeMap,
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
    edgesForParent(edgeMap, spaceItem.roomId).forEach((edge) => {
      const childId = edge.childId;
      // because we can not find if a childId is space without joining
      // or requesting room summary, we will look it into spaceRooms local
      // cache which we maintain as we load summary in UI.
      if (getRoom(childId)?.isSpaceRoom() || spaceRooms.has(childId)) {
        const childItem: HierarchyItemSpace = {
          roomId: childId,
          content: edge.content,
          ts: edge.ts,
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
  closedCategory: (spaceId: string) => boolean,
  edgeMap: SpaceChildEdgeMap,
): SpaceHierarchy[] => {
  const spaceItems: HierarchyItemSpace[] = getHierarchySpaces(
    rootSpaceId,
    getRoom,
    spaceRooms,
    edgeMap,
  );

  const hierarchy: SpaceHierarchy[] = spaceItems.map((spaceItem) => {
    const space = getRoom(spaceItem.roomId);
    if (!space || closedCategory(spaceItem.roomId)) {
      return {
        space: spaceItem,
      };
    }
    const childItems: HierarchyItemRoom[] = [];
    edgesForParent(edgeMap, spaceItem.roomId).forEach((edge) => {
      const childId = edge.childId;
      if (getRoom(childId)?.isSpaceRoom() || spaceRooms.has(childId)) return;

      childItems.push({
        roomId: childId,
        content: edge.content,
        ts: edge.ts,
        parentId: spaceItem.roomId,
      });
    });

    return {
      space: spaceItem,
      rooms: childItems.sort(hierarchyItemTs).sort(hierarchyItemByOrder),
    };
  });

  return hierarchy;
};

const useNativeSpaceChildEdgeMap = (): SpaceChildEdgeMap => {
  const [edgeMap, setEdgeMap] = useState<SpaceChildEdgeMap>(() => new Map());

  useEffect(() => {
    let disposed = false;
    let inFlight = false;

    const refresh = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        if (!isSynaraDesktop()) {
          if (!disposed) setEdgeMap(new Map());
          return;
        }
        const snapshot = await readSpaceChildrenWithNativeOwner(true, (command, args) =>
          invokeDesktopWithAvailability(command, args),
        );
        if (!disposed) {
          setEdgeMap(spaceChildEdgeMapFromNative(snapshot.edges));
        }
      } catch {
        // Keep the last known graph during transient failures.
      } finally {
        inFlight = false;
      }
    };

    void refresh();
    const pollId = window.setInterval(() => void refresh(), 1_000);
    return () => {
      disposed = true;
      window.clearInterval(pollId);
    };
  }, []);

  return edgeMap;
};

export const useSpaceHierarchy = (
  spaceId: string,
  spaceRooms: Set<string>,
  getRoom: (roomId: string) => Room | undefined,
  closedCategory: (spaceId: string) => boolean,
): SpaceHierarchy[] => {
  const edgeMap = useNativeSpaceChildEdgeMap();

  const [hierarchyAtom] = useState(() =>
    atom(getSpaceHierarchy(spaceId, spaceRooms, getRoom, closedCategory, edgeMap)),
  );
  const [hierarchy, setHierarchy] = useAtom(hierarchyAtom);

  useEffect(() => {
    setHierarchy(getSpaceHierarchy(spaceId, spaceRooms, getRoom, closedCategory, edgeMap));
  }, [spaceId, spaceRooms, setHierarchy, getRoom, closedCategory, edgeMap]);

  return hierarchy;
};

const getSpaceJoinedHierarchy = (
  rootSpaceId: string,
  getRoom: GetRoomCallback,
  excludeRoom: (parentId: string, roomId: string) => boolean,
  sortRoomItems: (parentId: string, items: HierarchyItem[]) => HierarchyItem[],
  edgeMap: SpaceChildEdgeMap,
): HierarchyItem[] => {
  const spaceItems: HierarchyItemSpace[] = getHierarchySpaces(
    rootSpaceId,
    getRoom,
    new Set(),
    edgeMap,
  );

  const hierarchy: HierarchyItem[] = spaceItems.flatMap((spaceItem) => {
    const space = getRoom(spaceItem.roomId);
    if (!space) {
      return [];
    }
    const joinedRoomEdges = edgesForParent(edgeMap, spaceItem.roomId).filter((edge) => {
      const room = getRoom(edge.childId);
      if (!room || room.isSpaceRoom()) return false;
      return true;
    });

    if (joinedRoomEdges.length === 0) return [];

    const childItems: HierarchyItemRoom[] = [];
    joinedRoomEdges.forEach((edge) => {
      if (excludeRoom(space.roomId, edge.childId)) return;
      childItems.push({
        roomId: edge.childId,
        content: edge.content,
        ts: edge.ts,
        parentId: spaceItem.roomId,
      });
    });
    return [spaceItem, ...sortRoomItems(spaceItem.roomId, childItems)];
  });

  return hierarchy;
};

export const useSpaceJoinedHierarchy = (
  spaceId: string,
  getRoom: GetRoomCallback,
  excludeRoom: (parentId: string, roomId: string) => boolean,
  sortByActivity: (spaceId: string) => boolean,
): HierarchyItem[] => {
  const mx = useMatrixClient();
  const edgeMap = useNativeSpaceChildEdgeMap();

  const sortRoomItems = useCallback(
    (sId: string, items: HierarchyItem[]) => {
      if (sortByActivity(sId)) {
        items.sort((a, b) => factoryRoomIdByActivity(mx)(a.roomId, b.roomId));
        return items;
      }
      items.sort(hierarchyItemTs).sort(hierarchyItemByOrder);
      return items;
    },
    [mx, sortByActivity],
  );

  const [hierarchyAtom] = useState(() =>
    atom(getSpaceJoinedHierarchy(spaceId, getRoom, excludeRoom, sortRoomItems, edgeMap)),
  );
  const [hierarchy, setHierarchy] = useAtom(hierarchyAtom);

  useEffect(() => {
    setHierarchy(getSpaceJoinedHierarchy(spaceId, getRoom, excludeRoom, sortRoomItems, edgeMap));
  }, [spaceId, setHierarchy, getRoom, excludeRoom, sortRoomItems, edgeMap]);

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
  roomId: string,
): Promise<SpaceHierarchyRoom[]> {
  const result = await readSpaceHierarchyWithNativeOwner(
    roomId,
    isSynaraDesktop(),
    (command, args) => invokeDesktopWithAvailability(command, args),
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
  enable: boolean,
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
