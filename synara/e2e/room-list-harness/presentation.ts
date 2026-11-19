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

/** A stored room folder (`in.synara.room_folders`): display order, filing order. */
export type FixtureRoomFolder = { id: string; name: string; rooms: string[] };

/**
 * Core's section rule (`room_list/folders.rs`): a folder shows its listed,
 * joined rooms that are neither DMs, spaces nor favorites, in filing order,
 * with the Recent and Name orders and the attention rollup over them.
 */
const foldersFor = (
  rooms: RoomSummary[],
  folders: FixtureRoomFolder[],
  unread: RoomListPresentation['unread'],
  recentOrder: string[],
  nameOrder: string[]
): RoomListPresentation['folders'] => {
  const byId = new Map(rooms.map((room) => [room.roomId, room]));
  return folders.map((folder) => {
    const roomIds = folder.rooms.filter((roomId) => {
      const room = byId.get(roomId);
      return (
        !!room && room.membership === 'join' && !room.isDirect && !room.isSpace && !room.isFavorite
      );
    });
    const filed = new Set(roomIds);
    const rows = unread.filter((row) => filed.has(row.roomId));
    return {
      id: folder.id,
      name: folder.name,
      roomIds,
      recentOrder: recentOrder.filter((roomId) => filed.has(roomId)),
      nameOrder: nameOrder.filter((roomId) => filed.has(roomId)),
      unreadHighlight: rows.reduce((sum, row) => sum + row.highlight, 0),
      unreadTotal: rows.reduce((sum, row) => sum + row.total, 0),
      unreadRoomIds: rows.map((row) => row.roomId).sort(),
    };
  });
};

/** Filed rooms carry `folderId` (joined, not DMs or spaces; favorites keep theirs). */
export const withFolderIds = (
  rooms: RoomSummary[],
  folders: FixtureRoomFolder[]
): RoomSummary[] => {
  const folderOf = new Map<string, string>();
  for (const folder of folders) {
    for (const roomId of folder.rooms) if (!folderOf.has(roomId)) folderOf.set(roomId, folder.id);
  }
  return rooms.map((room) => {
    const folderId = folderOf.get(room.roomId);
    const next: RoomSummary = { ...room };
    delete next.folderId;
    if (folderId && room.membership === 'join' && !room.isDirect && !room.isSpace) {
      next.folderId = folderId;
    }
    return next;
  });
};

export const presentationFor = (
  rooms: RoomSummary[],
  folders: FixtureRoomFolder[] = []
): RoomListPresentation => {
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
  const recentOrder = [...rooms].sort(byRecent).map((room) => room.roomId);
  const nameOrder = [...rooms].sort(byName).map((room) => room.roomId);
  return {
    recentOrder,
    nameOrder,
    favoriteRoomIds: rooms
      .filter((room) => room.membership === 'join' && room.isFavorite)
      .map((room) => room.roomId),
    unread,
    highlightTotal,
    unreadTotal,
    folders: foldersFor(rooms, folders, unread, recentOrder, nameOrder),
  };
};
