import type {
  RoomListPresentation,
  RoomUnreadAttention,
} from '../../features/matrix-dto/generated';
import type { Unread } from '../../../types/matrix/room';

/**
 * Readers for Core's room-list presentation. Core owns the rules (ordering,
 * name normalization, favorites, which rooms count as unread, space rollup and
 * badge totals); these helpers only index into its result.
 */
export const EMPTY_ROOM_LIST_PRESENTATION: RoomListPresentation = {
  recentOrder: [],
  nameOrder: [],
  favoriteRoomIds: [],
  unread: [],
  highlightTotal: 0,
  unreadTotal: 0,
};

const isStringArray = (value: unknown): value is string[] =>
  Array.isArray(value) && value.every((item) => typeof item === 'string');

const isCount = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

const parseAttention = (value: unknown): RoomUnreadAttention | null => {
  if (!value || typeof value !== 'object') return null;
  const record = value as Record<string, unknown>;
  if (typeof record.roomId !== 'string') return null;
  if (!isCount(record.highlight) || !isCount(record.total)) return null;
  if (record.fromRoomIds !== undefined && !isStringArray(record.fromRoomIds)) return null;
  return {
    roomId: record.roomId,
    highlight: record.highlight,
    total: record.total,
    ...(record.fromRoomIds ? { fromRoomIds: record.fromRoomIds } : {}),
  };
};

/**
 * Validate the wire presentation. A payload without one (an older Core) gets
 * the empty presentation; a malformed one is rejected.
 */
export const parseRoomListPresentation = (value: unknown): RoomListPresentation | null => {
  if (value === undefined) return EMPTY_ROOM_LIST_PRESENTATION;
  if (!value || typeof value !== 'object') return null;
  const record = value as Record<string, unknown>;
  if (
    !isStringArray(record.recentOrder) ||
    !isStringArray(record.nameOrder) ||
    !isStringArray(record.favoriteRoomIds) ||
    !Array.isArray(record.unread) ||
    !isCount(record.highlightTotal) ||
    !isCount(record.unreadTotal)
  ) {
    return null;
  }
  const unread: RoomUnreadAttention[] = [];
  for (const row of record.unread) {
    const parsed = parseAttention(row);
    if (!parsed) return null;
    unread.push(parsed);
  }
  return {
    recentOrder: record.recentOrder,
    nameOrder: record.nameOrder,
    favoriteRoomIds: record.favoriteRoomIds,
    unread,
    highlightTotal: record.highlightTotal,
    unreadTotal: record.unreadTotal,
  };
};

export type PresentationSort = 'recent' | 'name';

/**
 * Order a section's ids by Core's order. Ids Core did not list (a room added
 * after the snapshot) keep their relative order at the end.
 */
export const orderRoomIdsByPresentation = (
  roomIds: readonly string[],
  presentation: RoomListPresentation,
  sort: PresentationSort
): string[] => {
  const order = sort === 'name' ? presentation.nameOrder : presentation.recentOrder;
  const rank = new Map(order.map((roomId, index) => [roomId, index]));
  return roomIds
    .map((roomId, index) => ({ roomId, index, rank: rank.get(roomId) }))
    .sort((left, right) => {
      if (left.rank !== undefined && right.rank !== undefined) return left.rank - right.rank;
      if (left.rank !== undefined) return -1;
      if (right.rank !== undefined) return 1;
      return left.index - right.index;
    })
    .map(({ roomId }) => roomId);
};

export const favoriteRoomIdsFromPresentation = (presentation: RoomListPresentation): Set<string> =>
  new Set(presentation.favoriteRoomIds);

export const attentionToUnread = (row: RoomUnreadAttention): Unread => ({
  highlight: row.highlight,
  total: row.total,
  from: row.fromRoomIds ? new Set(row.fromRoomIds) : null,
});

/** Unread for one room or space, or `undefined` when it needs no attention. */
export const unreadFromPresentation = (
  presentation: RoomListPresentation,
  roomId: string
): Unread | undefined => {
  const row = presentation.unread.find((item) => item.roomId === roomId);
  return row ? attentionToUnread(row) : undefined;
};

/** Attention rows for direct rooms (not space rollups) among `roomIds`. */
export const roomAttentionRows = (
  presentation: RoomListPresentation,
  roomIds?: ReadonlySet<string>
): RoomUnreadAttention[] =>
  presentation.unread.filter(
    (row) => !row.fromRoomIds && (roomIds === undefined || roomIds.has(row.roomId))
  );
