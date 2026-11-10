import type { SidebarItems } from '../../hooks/useSidebarItems';

type UnreadLike = { total: number; highlight: number };

/**
 * Whether a space other than the one showing has unread, for the dot on the
 * switcher's chevron: red when any of them has a mention (the Slack rule).
 * Home's unread is not counted here; the bottom bar's Home badge carries it.
 */
export const otherSpacesUnread = (
  items: SidebarItems,
  selectedSpaceId: string | undefined,
  roomToUnread: ReadonlyMap<string, UnreadLike>
): { unread: boolean; attention: number } => {
  let unread = false;
  let attention = 0;
  items.forEach((item) => {
    const ids = typeof item === 'string' ? [item] : item.content;
    ids.forEach((spaceId) => {
      if (spaceId === selectedSpaceId) return;
      const spaceUnread = roomToUnread.get(spaceId);
      if (!spaceUnread) return;
      unread = true;
      attention += spaceUnread.highlight;
    });
  });
  return { unread, attention };
};
