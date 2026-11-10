/**
 * The desktop room list pane can be hidden (the list header's button,
 * ⌘⇧D / Ctrl+Shift+D, macOS View menu) to give the room the window's width.
 * The pane stays mounted, so its scroll position, sections and search survive.
 * While it is hidden, page headers show "Show room list" with its unread.
 */
import {
  appShortcut,
  bindingAriaKeyShortcuts,
  bindingLabel,
  type KeyBinding,
} from '../desktop-keyboard/appShortcuts';

/** The pane `PageRoot` collapses; the toggles' `aria-controls`. */
export const ROOM_LIST_PANE_ID = 'synara-room-list-pane';
/** The page header's "Show room list" button: focus lands here when the list hides. */
export const ROOM_LIST_TOGGLE_ID = 'synara-room-list-toggle';

/** The `toggle-room-list` binding from the shortcut registry, as shown and announced. */
export const roomListShortcut = (macOS: boolean) => {
  const shortcut = appShortcut('toggle-room-list');
  const binding = (macOS ? shortcut.mac : shortcut.windowsLinux) as KeyBinding;
  return {
    label: bindingLabel(binding, macOS),
    ariaKeyShortcuts: bindingAriaKeyShortcuts(binding),
  };
};

export const roomListToggleTitle = (collapsed: boolean): string =>
  collapsed ? 'Show room list' : 'Hide room list';

export type RoomListToggleBadge =
  { kind: 'none' } | { kind: 'dot' } | { kind: 'count'; count: number };

/**
 * The Slack rule (a red count for mentions and DMs, a dot for plain unread),
 * only while the list is hidden: a visible list shows its own unread.
 */
export const roomListToggleBadge = (
  collapsed: boolean,
  unread: boolean,
  attention: number
): RoomListToggleBadge => {
  if (!collapsed || !unread) return { kind: 'none' };
  return attention > 0 ? { kind: 'count', count: attention } : { kind: 'dot' };
};

/** The toggle's accessible name: its action, then what the hidden list holds. */
export const roomListToggleLabel = (collapsed: boolean, badge: RoomListToggleBadge): string => {
  const title = roomListToggleTitle(collapsed);
  if (badge.kind === 'count') {
    const noun = badge.count === 1 ? 'mention or direct message' : 'mentions and direct messages';
    return `${title}, ${badge.count} ${noun}`;
  }
  if (badge.kind === 'dot') return `${title}, unread messages`;
  return title;
};

/**
 * The open room's row (a link). Not any `aria-current`: the pane's bottom bar
 * marks its current place (Home, Activity) the same way.
 */
export const ROOM_LIST_CURRENT_ROW = 'a[aria-current="page"]';

/**
 * Where focus goes when the shortcut reveals the list: the open room's row
 * (the row Alt+↑/↓ steps from), else the pane's search field.
 */
export const roomListFocusTarget = (pane: ParentNode): HTMLElement | null =>
  pane.querySelector<HTMLElement>(ROOM_LIST_CURRENT_ROW) ??
  pane.querySelector<HTMLElement>('[data-testid="nav-search-field"]');
