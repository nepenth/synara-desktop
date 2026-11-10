import type { ISidebarFolder, SidebarItems, TSidebarItem } from '../../hooks/useSidebarItems';

/**
 * The order of spaces and folders, saved as `in.synara.spaces` account data.
 * Pure edits used by the space switcher (drag to reorder) and the Manage
 * spaces dialog (move, group, ungroup, rename). The drag rules are the former
 * rail's (`SpaceTabs`), less drop-onto-to-create-folder (decision D1).
 */

/** A dragged or targeted entry: a top-level space id, or a folder (or a space inside one). */
export type FolderDraggable = {
  folder: ISidebarFolder;
  /** A space inside `folder`; absent for the folder itself. */
  spaceId?: string;
};
export type SpaceDraggable = string | FolderDraggable;

export type ReorderInstruction = 'reorder-above' | 'reorder-below' | 'make-child';

export type SpaceOrderEdit = {
  items: SidebarItems;
  /** Folders left empty by the edit; their open state should be forgotten. */
  removedFolderIds: string[];
};

const matchDest = (sI: TSidebarItem, dI: SpaceDraggable): boolean => {
  if (typeof sI === 'string' && typeof dI === 'string') return sI === dI;
  if (typeof sI === 'object' && typeof dI === 'object') return sI.id === dI.folder.id;
  return false;
};

const itemAsFolderContent = (i: SpaceDraggable): string[] => {
  if (typeof i === 'string') return [i];
  if (i.spaceId) return [i.spaceId];
  return [...i.folder.content];
};

/**
 * Move `item` relative to `container`. `make-child` (dropping onto a space or
 * a closed folder) still folds the item in, for callers that allow it; the
 * switcher blocks it and offers folders in Manage spaces instead.
 */
export const reorderSpaceItems = (
  items: SidebarItems,
  item: SpaceDraggable,
  containerItem: SpaceDraggable,
  instructionType: ReorderInstruction,
  newFolderId: () => string
): SpaceOrderEdit => {
  const newItems: SidebarItems = [];
  const removedFolderIds: string[] = [];

  items.forEach((i) => {
    const sameFolders =
      typeof item === 'object' &&
      typeof containerItem === 'object' &&
      item.folder.id === containerItem.folder.id;

    // Take the dragged space out of its current position or folder.
    if (!sameFolders && matchDest(i, item)) {
      if (typeof item === 'object' && item.spaceId) {
        const folderContent = item.folder.content.filter((s) => s !== item.spaceId);
        if (folderContent.length === 0) {
          removedFolderIds.push(item.folder.id);
          return;
        }
        newItems.push({ ...item.folder, content: folderContent });
      }
      return;
    }
    if (matchDest(i, containerItem)) {
      if (instructionType === 'make-child') {
        const child = itemAsFolderContent(item);
        if (typeof containerItem === 'string') {
          newItems.push({ id: newFolderId(), content: [containerItem].concat(child) });
          return;
        }
        newItems.push({
          ...containerItem.folder,
          content: containerItem.folder.content.concat(child),
        });
        return;
      }

      // Dropped beside a space inside a folder, or reordered within one.
      if (typeof containerItem === 'object' && containerItem.spaceId) {
        const child = itemAsFolderContent(item);
        const newContent: string[] = [];
        containerItem.folder.content
          .filter((sId) => !child.includes(sId))
          .forEach((sId) => {
            if (sId === containerItem.spaceId) {
              if (instructionType === 'reorder-below') newContent.push(sId, ...child);
              if (instructionType === 'reorder-above') newContent.push(...child, sId);
              return;
            }
            newContent.push(sId);
          });
        newItems.push({ ...containerItem.folder, content: newContent });
        return;
      }

      // Dropped above or below a top-level space or folder.
      if (typeof item === 'string') {
        if (instructionType === 'reorder-below') newItems.push(i);
        newItems.push(item);
        if (instructionType === 'reorder-above') newItems.push(i);
      } else if (item.spaceId) {
        if (instructionType === 'reorder-above') newItems.push(item.spaceId);
        if (sameFolders && typeof i === 'object') {
          // Out of its own folder, placed beside that folder.
          const newI = { ...i, content: i.content.filter((sId) => sId !== item.spaceId) };
          if (newI.content.length > 0) newItems.push(newI);
          else removedFolderIds.push(i.id);
        } else {
          newItems.push(i);
        }
        if (instructionType === 'reorder-below') newItems.push(item.spaceId);
      } else {
        if (instructionType === 'reorder-below') newItems.push(i);
        newItems.push(item.folder);
        if (instructionType === 'reorder-above') newItems.push(i);
      }
      return;
    }
    newItems.push(i);
  });

  return { items: newItems, removedFolderIds };
};

const itemId = (item: TSidebarItem): string => (typeof item === 'string' ? item : item.id);

/**
 * Move a top-level entry (space id or folder id), or a space inside a folder,
 * one place up or down. Returns `items` unchanged at either end.
 */
export const moveSpaceItem = (items: SidebarItems, id: string, direction: -1 | 1): SidebarItems => {
  const index = items.findIndex((item) => itemId(item) === id);
  if (index >= 0) {
    const target = index + direction;
    if (target < 0 || target >= items.length) return items;
    const next = [...items];
    [next[index], next[target]] = [next[target], next[index]];
    return next;
  }
  return items.map((item) => {
    if (typeof item === 'string' || !item.content.includes(id)) return item;
    const at = item.content.indexOf(id);
    const target = at + direction;
    if (target < 0 || target >= item.content.length) return item;
    const content = [...item.content];
    [content[at], content[target]] = [content[target], content[at]];
    return { ...item, content };
  });
};

/**
 * Group top-level spaces into a new folder at the position of the first of
 * them. Spaces already in a folder are left where they are.
 */
export const groupSpacesIntoFolder = (
  items: SidebarItems,
  spaceIds: string[],
  folderId: string,
  name?: string
): SidebarItems => {
  const chosen = new Set(spaceIds);
  const content = items.filter(
    (item): item is string => typeof item === 'string' && chosen.has(item)
  );
  if (content.length === 0) return items;
  const trimmed = name?.trim();
  const folder: ISidebarFolder = trimmed
    ? { id: folderId, name: trimmed, content }
    : { id: folderId, content };
  const next: SidebarItems = [];
  let placed = false;
  items.forEach((item) => {
    if (typeof item === 'string' && chosen.has(item)) {
      if (!placed) {
        next.push(folder);
        placed = true;
      }
      return;
    }
    next.push(item);
  });
  return next;
};

/** Move top-level spaces into the end of an existing folder. */
export const addSpacesToFolder = (
  items: SidebarItems,
  spaceIds: string[],
  folderId: string
): SidebarItems => {
  const chosen = new Set(spaceIds);
  const moving = items.filter(
    (item): item is string => typeof item === 'string' && chosen.has(item)
  );
  if (moving.length === 0) return items;
  return items
    .filter((item) => !(typeof item === 'string' && chosen.has(item)))
    .map((item) =>
      typeof item === 'object' && item.id === folderId
        ? { ...item, content: [...item.content, ...moving] }
        : item
    );
};

/** Put a folder's spaces back at the top level, where the folder was. */
export const ungroupFolder = (items: SidebarItems, folderId: string): SidebarItems =>
  items.flatMap((item) =>
    typeof item === 'object' && item.id === folderId ? [...item.content] : [item]
  );

/** Take one space out of its folder and place it right after that folder. */
export const removeSpaceFromFolder = (items: SidebarItems, spaceId: string): SidebarItems =>
  items.flatMap((item) => {
    if (typeof item === 'string' || !item.content.includes(spaceId)) return [item];
    const content = item.content.filter((id) => id !== spaceId);
    return content.length > 0 ? [{ ...item, content }, spaceId] : [spaceId];
  });

/** Rename a folder; an empty name falls back to listing its spaces. */
export const renameFolder = (items: SidebarItems, folderId: string, name: string): SidebarItems =>
  items.map((item) => {
    if (typeof item === 'string' || item.id !== folderId) return item;
    const trimmed = name.trim();
    if (trimmed) return { ...item, name: trimmed };
    return { id: item.id, content: item.content };
  });

/** A folder's label: its name, else its spaces' names. */
export const folderLabel = (folder: ISidebarFolder, nameOf: (spaceId: string) => string): string =>
  folder.name ??
  (folder.content
    .map(nameOf)
    .filter((name) => name)
    .join(', ') ||
    'Folder');

export type SwitcherRow =
  | { kind: 'space'; spaceId: string; depth: 0 | 1; folder?: ISidebarFolder }
  | { kind: 'folder'; folder: ISidebarFolder; open: boolean };

/**
 * The switcher's rows in order. Folders are collapsible groups; a filter
 * flattens everything to the matching spaces (case- and accent-insensitive).
 */
export const switcherRows = (
  items: SidebarItems,
  openFolderIds: ReadonlySet<string>,
  filter: string,
  nameOf: (spaceId: string) => string
): SwitcherRow[] => {
  const query = normalizeQuery(filter);
  if (query) {
    const rows: SwitcherRow[] = [];
    items.forEach((item) => {
      const ids = typeof item === 'string' ? [item] : item.content;
      ids.forEach((spaceId) => {
        if (normalizeQuery(nameOf(spaceId)).includes(query)) {
          rows.push({ kind: 'space', spaceId, depth: 0 });
        }
      });
    });
    return rows;
  }
  const rows: SwitcherRow[] = [];
  items.forEach((item) => {
    if (typeof item === 'string') {
      rows.push({ kind: 'space', spaceId: item, depth: 0 });
      return;
    }
    const open = openFolderIds.has(item.id);
    rows.push({ kind: 'folder', folder: item, open });
    if (open) {
      item.content.forEach((spaceId) =>
        rows.push({ kind: 'space', spaceId, depth: 1, folder: item })
      );
    }
  });
  return rows;
};

const normalizeQuery = (text: string): string =>
  text.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase().trim();

/** The switcher offers a type-ahead filter once the list is longer than this. */
export const SWITCHER_FILTER_THRESHOLD = 8;

export const spaceCount = (items: SidebarItems): number =>
  items.reduce((total, item) => total + (typeof item === 'string' ? 1 : item.content.length), 0);
