import type { RoomListPresentation } from '../../src/app/features/matrix-dto/generated';
import type { RoomSummary } from '../../src/app/features/matrix-dto/room';

/**
 * Test-fixture mirror of Core's `room_list::presentation`, so mocked
 * room-list snapshots carry the presentation real Core attaches. The rules
 * themselves are owned and tested in Rust; keep this in step when they change.
 */
const normalizedName = (room: RoomSummary): string | undefined => {
  const name = room.name?.replace(/#/g, '').trim();
  return name ? name.toLowerCase() : undefined;
};

const byName = (a: RoomSummary, b: RoomSummary): number => {
  const an = normalizedName(a);
  const bn = normalizedName(b);
  if (an && bn && an !== bn) return an < bn ? -1 : 1;
  if (an && !bn) return -1;
  if (!an && bn) return 1;
  return a.roomId < b.roomId ? -1 : a.roomId > b.roomId ? 1 : 0;
};

const byRecent = (a: RoomSummary, b: RoomSummary): number => {
  const at = a.lastActivityTs;
  const bt = b.lastActivityTs;
  if (at !== undefined && bt !== undefined && at !== bt) return bt - at;
  if (at !== undefined && bt === undefined) return -1;
  if (at === undefined && bt !== undefined) return 1;
  return byName(a, b);
};

export const presentationFor = (rooms: RoomSummary[]): RoomListPresentation => {
  const unread = rooms
    .filter(
      (room) =>
        room.membership === 'join' &&
        !room.isSpace &&
        room.notificationMode !== 'mute' &&
        (room.markedUnread || room.unreadCount > 0 || room.highlightCount > 0)
    )
    .map((room) => ({
      roomId: room.roomId,
      highlight: room.highlightCount,
      total: Math.max(room.unreadCount, room.highlightCount, room.markedUnread ? 1 : 0),
    }));
  let highlightTotal = 0;
  let unreadTotal = 0;
  for (const row of unread) {
    if (row.highlight > 0) highlightTotal += row.highlight;
    else unreadTotal += row.total;
  }
  return {
    recentOrder: [...rooms].sort(byRecent).map((room) => room.roomId),
    nameOrder: [...rooms].sort(byName).map((room) => room.roomId),
    favoriteRoomIds: rooms
      .filter((room) => room.membership === 'join' && room.isFavorite)
      .map((room) => room.roomId),
    unread,
    highlightTotal,
    unreadTotal,
  };
};
