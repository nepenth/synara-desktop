import { useEffect } from 'react';

/**
 * Previous/next room and previous/next unread room walk the open list's data,
 * not its DOM: the list is virtual, so most rows are not rendered. Home and
 * Space publish the rows they show (sections in order, collapsed sections
 * already reduced to their unread and selected rows) and how to scroll a row
 * into view; the shortcut dispatcher reads the list that is mounted.
 */

export type RoomListNavSection = {
  /** Rows in display order; `undefined` for a row that is not a room (a section header). */
  rows: readonly (string | undefined)[];
  /** Route of a room in this section. */
  pathFor: (roomId: string) => string;
  /** Scroll row `index` of `rows` into view (a virtual list renders it first). */
  reveal?: (index: number) => void;
};

export type RoomListNavSource = {
  sections: readonly RoomListNavSection[];
  /** The open room, if it is in this list. */
  selectedRoomId?: string;
};

export type RoomListNavTarget = {
  roomId: string;
  path: string;
  section: RoomListNavSection;
  /** Row index within `section.rows`. */
  index: number;
};

export type RoomListNavDirection = 'previous' | 'next';

/** Every room row in display order. */
export const roomListNavTargets = (source: RoomListNavSource): RoomListNavTarget[] =>
  source.sections.flatMap((section) =>
    section.rows.flatMap((roomId, index) =>
      roomId === undefined ? [] : [{ roomId, path: section.pathFor(roomId), section, index }]
    )
  );

/**
 * The room one step from the open room, without wrapping. Without an open
 * room in the list, the first or last room.
 */
export const adjacentRoomTarget = (
  targets: readonly RoomListNavTarget[],
  selectedRoomId: string | undefined,
  direction: RoomListNavDirection
): RoomListNavTarget | undefined => {
  if (targets.length === 0) return undefined;
  const current = targets.findIndex((target) => target.roomId === selectedRoomId);
  if (current < 0) return direction === 'next' ? targets[0] : targets[targets.length - 1];
  return targets[direction === 'next' ? current + 1 : current - 1];
};

/**
 * The nearest unread room in `direction` from the open room, wrapping around
 * the list once; never the open room itself. Without an open room in the list,
 * the search starts before the first (next) or after the last (previous) row.
 */
export const adjacentUnreadTarget = (
  targets: readonly RoomListNavTarget[],
  selectedRoomId: string | undefined,
  direction: RoomListNavDirection,
  isUnread: (roomId: string) => boolean
): RoomListNavTarget | undefined => {
  const count = targets.length;
  if (count === 0) return undefined;
  const current = targets.findIndex((target) => target.roomId === selectedRoomId);
  const step = direction === 'next' ? 1 : -1;
  const start = current >= 0 ? current : direction === 'next' ? -1 : count;
  for (let offset = 1; offset <= count; offset += 1) {
    const target = targets[(((start + step * offset) % count) + count) % count];
    if (target.roomId !== selectedRoomId && isUnread(target.roomId)) return target;
  }
  return undefined;
};

// The mounted list. A stack, so a list that mounts before the previous one
// unmounts (a route change) does not lose its registration on that cleanup.
const sources: RoomListNavSource[] = [];

export const registerRoomListNavSource = (source: RoomListNavSource): (() => void) => {
  sources.push(source);
  return () => {
    const index = sources.lastIndexOf(source);
    if (index >= 0) sources.splice(index, 1);
  };
};

export const currentRoomListNavSource = (): RoomListNavSource | undefined => sources.at(-1);

/** Publish this list's rows to the keyboard shortcuts while it is mounted. */
export const useRoomListNavSource = (source: RoomListNavSource) => {
  useEffect(() => registerRoomListNavSource(source), [source]);
};
