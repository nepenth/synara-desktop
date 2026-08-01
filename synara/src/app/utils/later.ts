import { SynaraLaterContent, SynaraLaterItem } from '../../types/matrix/accountData';

export const LATER_ACCOUNT_DATA_VERSION = 1;

export const getLaterItemId = (roomId: string, eventId: string): string => `${roomId}\n${eventId}`;

export const emptyLaterContent = (): SynaraLaterContent => ({
  version: LATER_ACCOUNT_DATA_VERSION,
  items: {},
});

const normalizeLaterItem = (item: unknown): SynaraLaterItem | undefined => {
  if (!item || typeof item !== 'object') return undefined;
  const value = item as Partial<SynaraLaterItem>;
  if (
    typeof value.id !== 'string' ||
    (value.kind !== 'saved' && value.kind !== 'reminder') ||
    typeof value.roomId !== 'string' ||
    typeof value.eventId !== 'string' ||
    typeof value.createdAt !== 'number' ||
    !Number.isFinite(value.createdAt)
  ) {
    return undefined;
  }
  const next: SynaraLaterItem = {
    id: value.id,
    kind: value.kind,
    roomId: value.roomId,
    eventId: value.eventId,
    createdAt: value.createdAt,
  };
  if (typeof value.dueTs === 'number' && Number.isFinite(value.dueTs)) {
    next.dueTs = value.dueTs;
  }
  if (typeof value.remindedAt === 'number' && Number.isFinite(value.remindedAt)) {
    next.remindedAt = value.remindedAt;
  }
  if (typeof value.completedAt === 'number' && Number.isFinite(value.completedAt)) {
    next.completedAt = value.completedAt;
  }
  return next;
};

export const normalizeLaterContent = (
  content?: Partial<SynaraLaterContent>
): SynaraLaterContent => {
  const entries =
    content?.items && typeof content.items === 'object' ? Object.entries(content.items) : [];
  const items = entries.reduce<Record<string, SynaraLaterItem>>((normalized, [itemId, item]) => {
    const laterItem = normalizeLaterItem(item);
    if (laterItem) {
      return {
        ...normalized,
        [itemId]: laterItem,
      };
    }
    return normalized;
  }, {});

  return {
    version: LATER_ACCOUNT_DATA_VERSION,
    items,
  };
};

export const putLaterItem = (
  content: Partial<SynaraLaterContent> | undefined,
  item: SynaraLaterItem
): SynaraLaterContent => {
  const next = normalizeLaterContent(content);
  return {
    ...next,
    items: {
      ...next.items,
      [item.id]: item,
    },
  };
};

export const removeLaterItem = (
  content: Partial<SynaraLaterContent> | undefined,
  itemId: string
): SynaraLaterContent => {
  const next = normalizeLaterContent(content);
  const items = { ...next.items };
  delete items[itemId];
  return {
    ...next,
    items,
  };
};

export const updateLaterItemById = (
  content: Partial<SynaraLaterContent> | undefined,
  itemId: string,
  update: (item: SynaraLaterItem) => SynaraLaterItem
): SynaraLaterContent => {
  const next = normalizeLaterContent(content);
  const item = next.items?.[itemId];
  if (!item) return next;
  return {
    ...next,
    items: {
      ...next.items,
      [itemId]: update(item),
    },
  };
};

export const completeLaterItem = (
  content: Partial<SynaraLaterContent> | undefined,
  itemId: string,
  completedAt = Date.now()
): SynaraLaterContent =>
  updateLaterItemById(content, itemId, (item) => ({
    ...item,
    completedAt,
  }));

export const snoozeLaterItem = (
  content: Partial<SynaraLaterContent> | undefined,
  itemId: string,
  dueTs: number
): SynaraLaterContent =>
  updateLaterItemById(content, itemId, (item) => ({
    ...item,
    kind: 'reminder',
    dueTs,
    remindedAt: undefined,
    completedAt: undefined,
  }));

export const clearCompletedLaterItems = (
  content: Partial<SynaraLaterContent> | undefined
): SynaraLaterContent => {
  const next = normalizeLaterContent(content);
  const items = Object.fromEntries(
    Object.entries(next.items ?? {}).filter(([, item]) => !item.completedAt)
  );
  return {
    ...next,
    items,
  };
};

export const getSortedLaterItems = (
  content: Partial<SynaraLaterContent> | undefined,
  now = Date.now()
): SynaraLaterItem[] => {
  const items = normalizeLaterContent(content).items ?? {};
  return Object.values(items).sort((a, b) => {
    if (!!a.completedAt !== !!b.completedAt) return a.completedAt ? 1 : -1;
    const aDue = a.dueTs ?? Number.MAX_SAFE_INTEGER;
    const bDue = b.dueTs ?? Number.MAX_SAFE_INTEGER;
    if (aDue <= now !== bDue <= now) return aDue <= now ? -1 : 1;
    if (aDue !== bDue) return aDue - bDue;
    return b.createdAt - a.createdAt;
  });
};

export type LaterDueSummary = {
  active: number;
  completed: number;
  overdue: number;
  dueToday: number;
};

export const getLaterDueSummary = (
  content: Partial<SynaraLaterContent> | undefined,
  now = Date.now()
): LaterDueSummary => {
  const items = Object.values(normalizeLaterContent(content).items ?? {});
  const endOfToday = new Date(now);
  endOfToday.setHours(23, 59, 59, 999);
  return items.reduce<LaterDueSummary>(
    (summary, item) => {
      if (item.completedAt) {
        return { ...summary, completed: summary.completed + 1 };
      }
      const active = summary.active + 1;
      const overdue = item.dueTs && item.dueTs <= now ? summary.overdue + 1 : summary.overdue;
      const dueToday =
        item.dueTs && item.dueTs > now && item.dueTs <= endOfToday.getTime()
          ? summary.dueToday + 1
          : summary.dueToday;
      return { ...summary, active, overdue, dueToday };
    },
    { active: 0, completed: 0, overdue: 0, dueToday: 0 }
  );
};
