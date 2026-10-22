/**
 * Window-level keyboard shortcuts of the desktop app: page zoom, Settings,
 * previous/next room and (Linux) Quit. macOS also lists the zoom and Settings
 * items in its app menu; the webview sees the key first, so a handled key
 * never reaches the menu as well.
 */

export type AppShortcutAction =
  'zoom-in' | 'zoom-out' | 'zoom-reset' | 'settings' | 'previous-room' | 'next-room' | 'quit';

export const APP_SHORTCUT_ACTIONS: readonly AppShortcutAction[] = [
  'zoom-in',
  'zoom-out',
  'zoom-reset',
  'settings',
  'previous-room',
  'next-room',
  'quit',
];

/** Menu clicks arrive as this event with an `AppShortcutAction` payload. */
export const APP_SHORTCUT_MENU_EVENT = 'synara://app-shortcut';

/** Zoom levels the shortcuts step through; the Appearance field allows 75–200. */
export const PAGE_ZOOM_STEPS = [75, 80, 90, 100, 110, 125, 150, 175, 200] as const;

export const nextPageZoom = (
  current: number,
  action: Extract<AppShortcutAction, 'zoom-in' | 'zoom-out' | 'zoom-reset'>
): number => {
  if (action === 'zoom-reset') return 100;
  if (action === 'zoom-in') {
    return PAGE_ZOOM_STEPS.find((step) => step > current) ?? PAGE_ZOOM_STEPS.at(-1) ?? current;
  }
  return [...PAGE_ZOOM_STEPS].reverse().find((step) => step < current) ?? PAGE_ZOOM_STEPS[0];
};

type ShortcutKeyEvent = Pick<
  KeyboardEvent,
  'key' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey' | 'isComposing'
>;

export type AppShortcutPlatform = {
  /** Cmd is the primary modifier. */
  macOS: boolean;
  /** Ctrl+Q quits (no app menu there). */
  linux: boolean;
};

export const appShortcutAction = (
  evt: ShortcutKeyEvent,
  platform: AppShortcutPlatform
): AppShortcutAction | undefined => {
  if (evt.isComposing) return undefined;
  const primary = platform.macOS ? evt.metaKey : evt.ctrlKey;
  const otherPrimary = platform.macOS ? evt.ctrlKey : evt.metaKey;
  if (primary && !otherPrimary && !evt.altKey) {
    // `+` is Shift+= on most layouts; accept both.
    if (evt.key === '=' || evt.key === '+') return 'zoom-in';
    if (evt.shiftKey) return undefined;
    if (evt.key === '-') return 'zoom-out';
    if (evt.key === '0') return 'zoom-reset';
    if (evt.key === ',') return 'settings';
    if (platform.linux && evt.key.toLowerCase() === 'q') return 'quit';
    return undefined;
  }
  if (evt.altKey && !evt.ctrlKey && !evt.metaKey && !evt.shiftKey) {
    if (evt.key === 'ArrowUp') return 'previous-room';
    if (evt.key === 'ArrowDown') return 'next-room';
  }
  return undefined;
};

/**
 * The room link one step from the open room in sidebar order. The list is
 * virtual, so only rows on screen are candidates (the open room's
 * neighbours normally are). Without an open room, the first or last row.
 */
export const adjacentRoomLink = <T extends { getAttribute: (name: string) => string | null }>(
  links: readonly T[],
  direction: 'previous-room' | 'next-room'
): T | undefined => {
  if (links.length === 0) return undefined;
  const current = links.findIndex((link) => link.getAttribute('aria-current') === 'page');
  if (current < 0) return direction === 'next-room' ? links[0] : links[links.length - 1];
  const next = direction === 'next-room' ? current + 1 : current - 1;
  return links[next];
};
