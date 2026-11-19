import type { RoomFolderPresentation, RoomListPresentation } from '../matrix-dto/generated';
import { makeNavCategoryId } from '../../state/closedNavCategories';
import type { RoomListNavSection } from '../desktop-keyboard/roomListNavigation';

/**
 * The room list as a list of sections (Favorites, one per room folder, Rooms,
 * Direct messages on Home; Favorites and folders before the space's own
 * groups in a space). Each section renders its own virtual list over the
 * pane's shared scroller (`VirtualRoomSection`).
 *
 * Core decides which rooms belong to a folder (`presentation.folders`, the
 * section rule in `room_list/folders.rs`) and their orders; this module only
 * picks the rooms the open view lists and lays the sections out.
 */

export type RoomListSectionKind = 'favorites' | 'folder' | 'rooms' | 'directs';

export type RoomFolderUnread = { highlight: number; total: number };

export type RoomListSectionModel = {
  /** `favorites`, `rooms`, `directs` or `folder:<id>` (`data-section-id`). */
  id: string;
  kind: RoomListSectionKind;
  /** Collapse id in the device's closed categories. */
  categoryId: string;
  closed: boolean;
  /** Every room of the section, in display order. */
  roomIds: string[];
  /** The rows shown: all of `roomIds`, or only unread and selected ones while collapsed. */
  rows: string[];
  /** Folders: id, name and position among all folders (display order). */
  folder?: { id: string; name: string; index: number; count: number };
  /** Folders: attention over `roomIds`, for the collapsed header. */
  unread?: RoomFolderUnread;
};

export const roomFolderSectionId = (folderId: string): string => `folder:${folderId}`;

/** `makeNavCategoryId('home', 'folder:<id>')`, or the space's equivalent. */
export const roomFolderCategoryId = (scope: string, folderId: string): string =>
  makeNavCategoryId(scope, roomFolderSectionId(folderId));

/** A collapsed section keeps its unread and selected rows visible (Slack). */
export const visibleWhenClosed = (
  roomIds: string[],
  closed: boolean,
  hasUnread: (roomId: string) => boolean,
  selectedRoomId: string | undefined
): string[] =>
  closed ? roomIds.filter((rId) => hasUnread(rId) || rId === selectedRoomId) : roomIds;

/**
 * How a folder's rooms are ordered: Core's Recent or Name order for the
 * folder (it follows the Rooms section's choice), or an explicit order (a
 * space's own order).
 */
export type RoomFolderOrder = 'recent' | 'name' | readonly string[];

/** The folder's rooms that this view lists, in `order`. */
export const roomFolderRoomIdsInView = (
  folder: RoomFolderPresentation,
  inView: (roomId: string) => boolean,
  order: RoomFolderOrder
): string[] => {
  if (order === 'recent') return folder.recentOrder.filter(inView);
  if (order === 'name') return folder.nameOrder.filter(inView);
  const filed = new Set(folder.roomIds);
  return order.filter((roomId) => filed.has(roomId) && inView(roomId));
};

/**
 * Attention for a folder section. When it lists every room of the folder this
 * is Core's rollup; otherwise (a space shows only its own rooms) the same rows
 * summed over the rooms shown. Muted rooms have no attention row in either.
 */
export const roomFolderUnread = (
  folder: RoomFolderPresentation,
  roomIds: readonly string[],
  presentation: RoomListPresentation
): RoomFolderUnread => {
  if (roomIds.length === folder.roomIds.length) {
    return { highlight: folder.unreadHighlight, total: folder.unreadTotal };
  }
  const shown = new Set(roomIds);
  let highlight = 0;
  let total = 0;
  for (const row of presentation.unread) {
    if (row.fromRoomIds || !shown.has(row.roomId)) continue;
    highlight += row.highlight;
    total += row.total;
  }
  return { highlight, total };
};

export type RoomFolderSectionsInput = {
  presentation: RoomListPresentation;
  /** `home` or the space's room id: the collapse-id scope. */
  scope: string;
  /** Rooms this view lists. Core already left out favorites, DMs and spaces. */
  inView: (roomId: string) => boolean;
  order: RoomFolderOrder;
  closedCategories: ReadonlySet<string>;
  hasUnread: (roomId: string) => boolean;
  selectedRoomId?: string;
  /**
   * Which folders with no rooms in this view still show. Home keeps them all,
   * so they can be found and managed. A space keeps only a folder that holds
   * no rooms anywhere (`unfiled`), so one just created from the space's menu
   * appears; a folder whose rooms are elsewhere stays out of the space.
   */
  keepEmpty: 'all' | 'unfiled';
};

/** One section per folder, in folder order. */
export const buildRoomFolderSections = ({
  presentation,
  scope,
  inView,
  order,
  closedCategories,
  hasUnread,
  selectedRoomId,
  keepEmpty,
}: RoomFolderSectionsInput): RoomListSectionModel[] => {
  const count = presentation.folders.length;
  return presentation.folders.flatMap((folder, index) => {
    const roomIds = roomFolderRoomIdsInView(folder, inView, order);
    if (roomIds.length === 0 && keepEmpty === 'unfiled' && folder.roomIds.length > 0) return [];
    const categoryId = roomFolderCategoryId(scope, folder.id);
    const closed = closedCategories.has(categoryId);
    return [
      {
        id: roomFolderSectionId(folder.id),
        kind: 'folder' as const,
        categoryId,
        closed,
        roomIds,
        rows: visibleWhenClosed(roomIds, closed, hasUnread, selectedRoomId),
        folder: { id: folder.id, name: folder.name, index, count },
        unread: roomFolderUnread(folder, roomIds, presentation),
      },
    ];
  });
};

/** Every room filed in a section of `sections` (those leave the default sections). */
export const roomIdsInFolderSections = (sections: readonly RoomListSectionModel[]): Set<string> =>
  new Set(sections.flatMap((section) => (section.kind === 'folder' ? section.roomIds : [])));

/** An open folder with no rooms in this view shows one quiet "No rooms" line. */
export const showsEmptyFolderLine = (section: RoomListSectionModel): boolean =>
  section.kind === 'folder' && section.roomIds.length === 0 && !section.closed;

/**
 * Previous/next room walk the sections in display order (Favorites, each
 * folder, then the default sections), through every row, rendered or not.
 */
export const roomListNavSections = (
  sections: readonly RoomListSectionModel[],
  pathFor: (roomId: string) => string,
  reveal: (sectionId: string, index: number) => void
): RoomListNavSection[] =>
  sections.map((section) => ({
    rows: section.rows,
    pathFor,
    reveal: (index) => reveal(section.id, index),
  }));

/** Folder ids and names, to notice a folder change from another device. */
export const roomFoldersSignature = (presentation: RoomListPresentation): string =>
  JSON.stringify(presentation.folders.map((folder) => [folder.id, folder.name]));
