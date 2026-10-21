import type { RoomToParents } from '../../types/matrix/room';
import {
  getDirectRoomPath,
  getDirectRoomThreadPath,
  getHomeRoomPath,
  getHomeRoomThreadPath,
  getSpaceRoomPath,
  getSpaceRoomThreadPath,
} from '../pages/pathUtils';
import { HOME_PATH } from '../pages/paths';
import { getOrphanParents } from '../utils/room';

/** Everything that decides which section owns a joined room's route. */
export type RoomRouteScope = {
  roomToParents: RoomToParents;
  directRoomIds: ReadonlySet<string>;
  spaceSelectedId?: string;
  developerTools?: boolean;
  /** Picks one of several top-level spaces; the first one otherwise. */
  pickParent?: (roomId: string, parents: string[]) => string | undefined;
  /** Canonical alias lookup; the room ID otherwise. */
  toIdOrAlias?: (roomId: string) => string;
};

export type RoomRouteTarget = { eventId?: string } | { threadRootId: string };

/**
 * The route in-app navigation opens for a room: its space when it has one,
 * Direct for DMs, Home otherwise. Notification clicks use the same route, so
 * they never land on the Home "View" card for a DM or a space room.
 */
export const resolveRoomRoutePath = (
  roomId: string,
  target: RoomRouteTarget,
  scope: RoomRouteScope
): string => {
  const toIdOrAlias = scope.toIdOrAlias ?? ((id: string) => id);
  const roomIdOrAlias = toIdOrAlias(roomId);
  const threadRootId = 'threadRootId' in target ? target.threadRootId : undefined;
  const eventId = 'eventId' in target ? target.eventId : undefined;
  const openSpaceTimeline = Boolean(scope.developerTools) && scope.spaceSelectedId === roomId;

  const orphanParents = openSpaceTimeline
    ? [roomId]
    : getOrphanParents(scope.roomToParents, roomId);
  if (orphanParents.length > 0) {
    const parentSpace =
      scope.spaceSelectedId && orphanParents.includes(scope.spaceSelectedId)
        ? scope.spaceSelectedId
        : (scope.pickParent?.(roomId, orphanParents) ?? orphanParents[0]);
    const spaceIdOrAlias = toIdOrAlias(parentSpace);
    const roomSegment = openSpaceTimeline ? roomId : roomIdOrAlias;
    return threadRootId
      ? getSpaceRoomThreadPath(spaceIdOrAlias, roomSegment, threadRootId)
      : getSpaceRoomPath(spaceIdOrAlias, roomSegment, eventId);
  }

  if (scope.directRoomIds.has(roomId)) {
    return threadRootId
      ? getDirectRoomThreadPath(roomIdOrAlias, threadRootId)
      : getDirectRoomPath(roomIdOrAlias, eventId);
  }

  return threadRootId
    ? getHomeRoomThreadPath(roomIdOrAlias, threadRootId)
    : getHomeRoomPath(roomIdOrAlias, eventId);
};

/** A Home route is canonical only when the room resolves back into Home. */
export const isHomeRoomRoutePath = (path: string): boolean => path.startsWith(HOME_PATH);
