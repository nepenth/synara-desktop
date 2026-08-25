import {
  SynaraRoomNoteItem,
  SynaraRoomNoteItemKind,
  SynaraRoomNotesContent,
} from '../../types/matrix/accountData';

export const ROOM_NOTES_ACCOUNT_DATA_VERSION = 1;
const MAX_NOTE_BODY_LENGTH = 4000;
const MAX_MESSAGE_BODY_LENGTH = 1000;
const MAX_ITEMS_PER_ROOM = 200;

const limitText = (value: string, maxLength: number): string => value.trim().slice(0, maxLength);

const getRoomBucket = (content: SynaraRoomNotesContent, roomId: string) =>
  content.rooms?.[roomId] ?? { items: {} };

const createRoomNoteId = (kind: SynaraRoomNoteItemKind, now = Date.now()): string =>
  `${kind}:${now.toString(36)}:${Math.random().toString(36).slice(2, 10)}`;

const normalizeRoomNoteItem = (item: unknown): SynaraRoomNoteItem | undefined => {
  if (!item || typeof item !== 'object') return undefined;
  const value = item as Partial<SynaraRoomNoteItem>;
  if (
    typeof value.id !== 'string' ||
    (value.kind !== 'note' && value.kind !== 'todo' && value.kind !== 'message') ||
    typeof value.roomId !== 'string' ||
    typeof value.createdAt !== 'number' ||
    !Number.isFinite(value.createdAt) ||
    typeof value.updatedAt !== 'number' ||
    !Number.isFinite(value.updatedAt)
  ) {
    return undefined;
  }

  const next: SynaraRoomNoteItem = {
    id: value.id,
    kind: value.kind,
    roomId: value.roomId,
    createdAt: value.createdAt,
    updatedAt: value.updatedAt,
  };

  if (typeof value.body === 'string') {
    next.body = limitText(
      value.body,
      value.kind === 'message' ? MAX_MESSAGE_BODY_LENGTH : MAX_NOTE_BODY_LENGTH
    );
  }
  if (typeof value.completedAt === 'number' && Number.isFinite(value.completedAt)) {
    next.completedAt = value.completedAt;
  }
  if (typeof value.order === 'number' && Number.isFinite(value.order)) {
    next.order = value.order;
  }
  if (typeof value.eventId === 'string') {
    next.eventId = value.eventId;
  }
  if (typeof value.eventTs === 'number' && Number.isFinite(value.eventTs)) {
    next.eventTs = value.eventTs;
  }
  if (typeof value.sender === 'string') {
    next.sender = value.sender;
  }

  if ((next.kind === 'note' || next.kind === 'todo') && !next.body) return undefined;
  if (next.kind === 'message' && !next.eventId) return undefined;
  return next;
};

export const normalizeRoomNotesContent = (
  content?: Partial<SynaraRoomNotesContent>
): SynaraRoomNotesContent => {
  const roomEntries =
    content?.rooms && typeof content.rooms === 'object' ? Object.entries(content.rooms) : [];
  const rooms = roomEntries.reduce<NonNullable<SynaraRoomNotesContent['rooms']>>(
    (normalizedRooms, [roomId, roomNotes]) => {
      if (!roomNotes || typeof roomNotes !== 'object') return normalizedRooms;
      const itemEntries =
        roomNotes.items && typeof roomNotes.items === 'object'
          ? Object.entries(roomNotes.items)
          : [];
      const items = itemEntries
        .map(([, item]) => normalizeRoomNoteItem(item))
        .filter((item): item is SynaraRoomNoteItem => !!item && item.roomId === roomId)
        .sort((a, b) => b.updatedAt - a.updatedAt)
        .slice(0, MAX_ITEMS_PER_ROOM)
        .reduce<Record<string, SynaraRoomNoteItem>>(
          (normalizedItems, item) => ({
            ...normalizedItems,
            [item.id]: item,
          }),
          {}
        );

      if (Object.keys(items).length === 0) return normalizedRooms;
      return {
        ...normalizedRooms,
        [roomId]: { items },
      };
    },
    {}
  );

  return {
    version: ROOM_NOTES_ACCOUNT_DATA_VERSION,
    rooms,
  };
};

export const getRoomNoteItems = (
  content: Partial<SynaraRoomNotesContent> | undefined,
  roomId: string
): SynaraRoomNoteItem[] => {
  const items = Object.values(normalizeRoomNotesContent(content).rooms?.[roomId]?.items ?? {});
  return items.sort((a, b) => {
    if (!!a.completedAt !== !!b.completedAt) return a.completedAt ? 1 : -1;
    if (a.kind !== b.kind) {
      const rank = { todo: 0, note: 1, message: 2 };
      return rank[a.kind] - rank[b.kind];
    }
    return (b.order ?? b.updatedAt) - (a.order ?? a.updatedAt);
  });
};

export const getRoomNotesSummary = (
  content: Partial<SynaraRoomNotesContent> | undefined,
  roomId: string
) => {
  const items = getRoomNoteItems(content, roomId);
  return {
    total: items.length,
    activeTodos: items.filter((item) => item.kind === 'todo' && !item.completedAt).length,
  };
};

export const putRoomNoteItem = (
  content: Partial<SynaraRoomNotesContent> | undefined,
  item: SynaraRoomNoteItem
): SynaraRoomNotesContent => {
  const next = normalizeRoomNotesContent(content);
  const roomBucket = getRoomBucket(next, item.roomId);
  return {
    ...next,
    rooms: {
      ...next.rooms,
      [item.roomId]: {
        items: {
          ...roomBucket.items,
          [item.id]: item,
        },
      },
    },
  };
};

export const removeRoomNoteItem = (
  content: Partial<SynaraRoomNotesContent> | undefined,
  roomId: string,
  itemId: string
): SynaraRoomNotesContent => {
  const next = normalizeRoomNotesContent(content);
  const roomBucket = getRoomBucket(next, roomId);
  const items = { ...roomBucket.items };
  delete items[itemId];
  return {
    ...next,
    rooms: {
      ...next.rooms,
      [roomId]: { items },
    },
  };
};

export const completeRoomTodoItem = (
  content: Partial<SynaraRoomNotesContent> | undefined,
  roomId: string,
  itemId: string,
  completed: boolean,
  now = Date.now()
): SynaraRoomNotesContent => {
  const next = normalizeRoomNotesContent(content);
  const item = next.rooms?.[roomId]?.items?.[itemId];
  if (!item || item.kind !== 'todo') return next;
  return putRoomNoteItem(next, {
    ...item,
    updatedAt: now,
    completedAt: completed ? now : undefined,
  });
};

export const moveRoomTodoItem = (
  content: Partial<SynaraRoomNotesContent> | undefined,
  roomId: string,
  itemId: string,
  direction: 'up' | 'down',
  now = Date.now()
): SynaraRoomNotesContent => {
  const next = normalizeRoomNotesContent(content);
  const roomBucket = getRoomBucket(next, roomId);
  const todoItems = Object.values(roomBucket.items ?? {})
    .filter((item) => item.kind === 'todo')
    .sort((a, b) => {
      if (!!a.completedAt !== !!b.completedAt) return a.completedAt ? 1 : -1;
      return (b.order ?? b.updatedAt) - (a.order ?? a.updatedAt);
    });
  const currentIndex = todoItems.findIndex((item) => item.id === itemId);
  const targetIndex = direction === 'up' ? currentIndex - 1 : currentIndex + 1;
  const currentItem = todoItems[currentIndex];
  const targetItem = todoItems[targetIndex];

  if (
    !currentItem ||
    !targetItem ||
    Boolean(currentItem.completedAt) !== Boolean(targetItem.completedAt)
  ) {
    return next;
  }

  return putRoomNoteItem(
    putRoomNoteItem(next, {
      ...currentItem,
      order: targetItem.order ?? targetItem.updatedAt,
      updatedAt: now,
    }),
    {
      ...targetItem,
      order: currentItem.order ?? currentItem.updatedAt,
      updatedAt: now,
    }
  );
};

export const rankRoomNoteItem = (
  roomItems: SynaraRoomNoteItem[],
  itemId: string,
  direction: 'up' | 'down'
): SynaraRoomNoteItem | undefined => {
  const item = roomItems.find((candidate) => candidate.id === itemId);
  if (!item || (item.kind !== 'note' && item.kind !== 'todo')) return undefined;

  const group = roomItems.filter(
    (candidate) =>
      candidate.kind === item.kind &&
      (item.kind !== 'todo' || Boolean(candidate.completedAt) === Boolean(item.completedAt))
  );
  const sourceIndex = group.findIndex((candidate) => candidate.id === itemId);
  const targetIndex = direction === 'up' ? sourceIndex - 1 : sourceIndex + 1;
  if (sourceIndex < 0 || targetIndex < 0 || targetIndex >= group.length) return undefined;

  const reordered = [...group];
  const [moved] = reordered.splice(sourceIndex, 1);
  reordered.splice(targetIndex, 0, moved);
  const previous = reordered[targetIndex - 1];
  const next = reordered[targetIndex + 1];
  const score = (candidate: SynaraRoomNoteItem): number => candidate.order ?? candidate.updatedAt;
  const edgeStep = (value: number): number => Math.max(1, Math.abs(value) * 1e-9);

  let order: number;
  if (previous && next) {
    const previousOrder = score(previous);
    const nextOrder = score(next);
    if (previousOrder <= nextOrder) return undefined;
    order = nextOrder + (previousOrder - nextOrder) / 2;
  } else if (previous) {
    const previousOrder = score(previous);
    order = previousOrder - edgeStep(previousOrder);
  } else if (next) {
    const nextOrder = score(next);
    order = nextOrder + edgeStep(nextOrder);
  } else {
    return undefined;
  }

  return { ...moved, order };
};

export const createManualRoomNoteItem = (
  roomId: string,
  kind: Extract<SynaraRoomNoteItemKind, 'note' | 'todo'>,
  body: string,
  now = Date.now()
): SynaraRoomNoteItem | undefined => {
  const normalizedBody = limitText(body, MAX_NOTE_BODY_LENGTH);
  if (!normalizedBody) return undefined;
  return {
    id: createRoomNoteId(kind, now),
    kind,
    roomId,
    body: normalizedBody,
    createdAt: now,
    updatedAt: now,
    order: now,
  };
};
