/**
 * The app's one shortcut registry: every window-level keyboard shortcut, its
 * default binding on macOS and on Windows/Linux, its label and the group it is
 * listed under in the shortcut help sheet. `DesktopAppShortcuts` is the only
 * dispatcher. macOS also lists some items in its menus (`menu.rs`); the
 * webview sees the key first, so a handled key never reaches the menu as
 * well, and a unit test keeps the menu accelerators equal to these bindings.
 *
 * Not app shortcuts, and so not here: the composer's formatting and send
 * keys, autocomplete and menu navigation, and arrow keys inside a list.
 */

export type AppShortcutAction =
  | 'search'
  | 'shortcut-help'
  | 'settings'
  | 'quit'
  | 'go-home'
  | 'go-activity'
  | 'go-approvals'
  | 'go-settings'
  | 'open-space-switcher'
  | 'previous-room'
  | 'next-room'
  | 'previous-unread-room'
  | 'next-unread-room'
  | 'focus-next-region'
  | 'focus-previous-region'
  | 'close-thread'
  | 'toggle-room-list'
  | 'zoom-in'
  | 'zoom-out'
  | 'zoom-reset';

/** Help sheet groups, in the order the sheet lists them. */
export type AppShortcutScope = 'general' | 'navigation' | 'room' | 'view';

export const APP_SHORTCUT_SCOPES: readonly { scope: AppShortcutScope; label: string }[] = [
  { scope: 'general', label: 'General' },
  { scope: 'navigation', label: 'Navigation' },
  { scope: 'room', label: 'Rooms and threads' },
  { scope: 'view', label: 'View' },
];

/**
 * One key combination. Modifiers not named must be up, except Shift when it
 * is `'any'` (a key such as `+` or `/` that some layouts type with Shift).
 */
export type KeyBinding = {
  /** `KeyboardEvent.key` values; single characters compare case-insensitively. */
  keys: readonly string[];
  ctrl?: boolean;
  meta?: boolean;
  alt?: boolean;
  shift?: boolean | 'any';
};

export type AppShortcut = {
  id: AppShortcutAction;
  /** Sentence case, as the help sheet lists it. */
  label: string;
  scope: AppShortcutScope;
  /** Default binding on macOS; absent when the shortcut does not exist there. */
  mac?: KeyBinding;
  /** Default binding on Windows and Linux. Never the Super key. */
  windowsLinux?: KeyBinding;
  /** Only on Linux (no app menu there). */
  linuxOnly?: boolean;
  /** Needs the desktop shell; the dispatcher ignores it in a plain browser. */
  desktopOnly?: boolean;
  /** Listed in the macOS menu with this binding as its accelerator. */
  menu?: boolean;
  /**
   * A second binding for that shortcut's action: the help sheet lists these
   * keys on that shortcut's row rather than repeating the label.
   */
  alsoFor?: AppShortcutAction;
  /**
   * Handled by its own component rather than the dispatcher (listed in the
   * help sheet only). Quit lives on the Linux title strip, which is mounted
   * on every screen, sign-in included.
   */
  external?: boolean;
};

/** Cmd on macOS, Ctrl on Windows and Linux. */
const primary = (
  keys: readonly string[],
  extra: Pick<KeyBinding, 'alt' | 'shift'> = {}
): Pick<AppShortcut, 'mac' | 'windowsLinux'> => ({
  mac: { keys, meta: true, ...extra },
  windowsLinux: { keys, ctrl: true, ...extra },
});

/**
 * The places, in bottom-bar order. Control on macOS and Alt on Windows and
 * Linux: Cmd/Ctrl+1–3 are the composer's heading keys, and Super belongs to
 * the window manager.
 */
const place = (digit: string): Pick<AppShortcut, 'mac' | 'windowsLinux'> => ({
  mac: { keys: [digit], ctrl: true },
  windowsLinux: { keys: [digit], alt: true },
});

/** The same keys on every platform. */
const everywhere = (binding: KeyBinding): Pick<AppShortcut, 'mac' | 'windowsLinux'> => ({
  mac: binding,
  windowsLinux: binding,
});

export const APP_SHORTCUTS: readonly AppShortcut[] = [
  // General
  { id: 'search', label: 'Search and jump to', scope: 'general', ...primary(['k']) },
  {
    id: 'shortcut-help',
    label: 'Show keyboard shortcuts',
    scope: 'general',
    ...primary(['/'], { shift: 'any' }),
  },
  {
    id: 'settings',
    label: 'Open settings',
    scope: 'general',
    desktopOnly: true,
    menu: true,
    ...primary([',']),
  },
  {
    id: 'quit',
    label: 'Quit Synara',
    scope: 'general',
    linuxOnly: true,
    external: true,
    windowsLinux: { keys: ['q'], ctrl: true },
  },
  // Navigation
  { id: 'go-home', label: 'Open home', scope: 'navigation', ...place('1') },
  { id: 'go-activity', label: 'Open activity', scope: 'navigation', ...place('2') },
  { id: 'go-approvals', label: 'Open approvals', scope: 'navigation', ...place('3') },
  {
    id: 'go-settings',
    label: 'Open settings',
    scope: 'navigation',
    alsoFor: 'settings',
    ...place('4'),
  },
  {
    // Cmd/Ctrl+S alone is the composer's strikethrough; with Shift it is free
    // (no composer, menu or global default takes it).
    id: 'open-space-switcher',
    label: 'Open space switcher',
    scope: 'navigation',
    ...primary(['s'], { shift: true }),
  },
  {
    id: 'previous-room',
    label: 'Previous room',
    scope: 'navigation',
    desktopOnly: true,
    ...everywhere({ keys: ['ArrowUp'], alt: true }),
  },
  {
    id: 'next-room',
    label: 'Next room',
    scope: 'navigation',
    desktopOnly: true,
    ...everywhere({ keys: ['ArrowDown'], alt: true }),
  },
  {
    id: 'previous-unread-room',
    label: 'Previous unread room',
    scope: 'navigation',
    desktopOnly: true,
    ...everywhere({ keys: ['ArrowUp'], alt: true, shift: true }),
  },
  {
    id: 'next-unread-room',
    label: 'Next unread room',
    scope: 'navigation',
    desktopOnly: true,
    ...everywhere({ keys: ['ArrowDown'], alt: true, shift: true }),
  },
  {
    id: 'focus-next-region',
    label: 'Move focus to the next area',
    scope: 'navigation',
    ...everywhere({ keys: ['F6'] }),
  },
  {
    id: 'focus-previous-region',
    label: 'Move focus to the previous area',
    scope: 'navigation',
    ...everywhere({ keys: ['F6'], shift: true }),
  },
  // Rooms and threads
  {
    id: 'close-thread',
    label: 'Close the thread pane',
    scope: 'room',
    ...everywhere({ keys: ['Escape'] }),
  },
  // View
  {
    id: 'toggle-room-list',
    label: 'Show or hide the room list',
    scope: 'view',
    desktopOnly: true,
    menu: true,
    // Slack's "toggle sidebar".
    ...primary(['d'], { shift: true }),
  },
  {
    id: 'zoom-in',
    label: 'Zoom in',
    scope: 'view',
    desktopOnly: true,
    menu: true,
    // `+` is Shift+= on most layouts; accept both.
    ...primary(['=', '+'], { shift: 'any' }),
  },
  {
    id: 'zoom-out',
    label: 'Zoom out',
    scope: 'view',
    desktopOnly: true,
    menu: true,
    ...primary(['-']),
  },
  {
    id: 'zoom-reset',
    label: 'Actual size',
    scope: 'view',
    desktopOnly: true,
    menu: true,
    ...primary(['0']),
  },
];

export const APP_SHORTCUT_ACTIONS: readonly AppShortcutAction[] = APP_SHORTCUTS.map(
  (shortcut) => shortcut.id
);

/** Menu clicks arrive as this event with an `AppShortcutAction` payload. */
export const APP_SHORTCUT_MENU_EVENT = 'synara://app-shortcut';

export type AppShortcutPlatform = {
  /** Cmd is the primary modifier. */
  macOS: boolean;
  /** Ctrl+Q quits (no app menu there). */
  linux: boolean;
};

export const appShortcut = (id: AppShortcutAction): AppShortcut => {
  const shortcut = APP_SHORTCUTS.find((candidate) => candidate.id === id);
  if (!shortcut) throw new Error(`Unknown app shortcut ${id}`);
  return shortcut;
};

/** The shortcut's binding on this platform, if it has one here. */
export const shortcutBinding = (
  shortcut: AppShortcut,
  platform: AppShortcutPlatform
): KeyBinding | undefined => {
  if (shortcut.linuxOnly && !platform.linux) return undefined;
  return platform.macOS ? shortcut.mac : shortcut.windowsLinux;
};

type ShortcutKeyEvent = Pick<
  KeyboardEvent,
  'key' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey' | 'isComposing'
>;

const sameKey = (pressed: string, bound: string): boolean =>
  bound.length === 1 ? pressed.toLowerCase() === bound.toLowerCase() : pressed === bound;

export const bindingMatches = (evt: ShortcutKeyEvent, binding: KeyBinding): boolean =>
  evt.ctrlKey === !!binding.ctrl &&
  evt.metaKey === !!binding.meta &&
  evt.altKey === !!binding.alt &&
  (binding.shift === 'any' || evt.shiftKey === !!binding.shift) &&
  binding.keys.some((bound) => sameKey(evt.key, bound));

export const appShortcutAction = (
  evt: ShortcutKeyEvent,
  platform: AppShortcutPlatform
): AppShortcutAction | undefined => {
  if (evt.isComposing) return undefined;
  return APP_SHORTCUTS.find((shortcut) => {
    const binding = shortcutBinding(shortcut, platform);
    return binding !== undefined && bindingMatches(evt, binding);
  })?.id;
};

const KEY_NAMES: Record<string, string> = {
  ArrowUp: '↑',
  ArrowDown: '↓',
  ArrowLeft: '←',
  ArrowRight: '→',
  Escape: 'Esc',
};

const keyName = (key: string): string =>
  KEY_NAMES[key] ?? (key.length === 1 ? key.toUpperCase() : key);

/** How the binding is written for people: `⌘⇧D` on macOS, `Ctrl+Shift+D` elsewhere. */
export const bindingLabel = (binding: KeyBinding, macOS: boolean): string => {
  const key = keyName(binding.keys[0]);
  const shift = binding.shift === true;
  if (macOS) {
    return `${binding.ctrl ? '⌃' : ''}${binding.meta ? '⌘' : ''}${binding.alt ? '⌥' : ''}${
      shift ? '⇧' : ''
    }${key}`;
  }
  return [
    binding.ctrl && 'Ctrl',
    binding.meta && 'Meta',
    binding.alt && 'Alt',
    shift && 'Shift',
    key,
  ]
    .filter(Boolean)
    .join('+');
};

/** The binding as an `aria-keyshortcuts` value, such as `Meta+Shift+D`. */
export const bindingAriaKeyShortcuts = (binding: KeyBinding): string =>
  [
    binding.ctrl && 'Control',
    binding.meta && 'Meta',
    binding.alt && 'Alt',
    binding.shift === true && 'Shift',
    binding.keys[0].length === 1 ? binding.keys[0].toUpperCase() : binding.keys[0],
  ]
    .filter(Boolean)
    .join('+');

/**
 * The macOS menu accelerator (Tauri syntax) for a menu shortcut, derived from
 * its macOS binding: `CmdOrCtrl+Shift+D`.
 */
export const menuAccelerator = (shortcut: AppShortcut): string | undefined => {
  const binding = shortcut.mac;
  if (!shortcut.menu || !binding) return undefined;
  return [
    binding.meta && 'CmdOrCtrl',
    binding.ctrl && 'Ctrl',
    binding.alt && 'Alt',
    binding.shift === true && 'Shift',
    binding.keys[0].length === 1 ? binding.keys[0].toUpperCase() : binding.keys[0],
  ]
    .filter(Boolean)
    .join('+');
};

export type AppShortcutHelpGroup = {
  scope: AppShortcutScope;
  label: string;
  items: {
    id: AppShortcutAction;
    label: string;
    /** Every combination that reaches the action, the row's own first. */
    keys: string[];
    /** The same combinations as an `aria-keyshortcuts` value (space-separated). */
    ariaKeyShortcuts: string;
  }[];
};

/** A shortcut's binding on this platform as shown (`⌃1`) and announced (`Control+1`). */
export const appShortcutHint = (
  id: AppShortcutAction,
  platform: AppShortcutPlatform
): { label: string; ariaKeyShortcuts: string } | undefined => {
  const binding = shortcutBinding(appShortcut(id), platform);
  if (!binding) return undefined;
  return {
    label: bindingLabel(binding, platform.macOS),
    ariaKeyShortcuts: bindingAriaKeyShortcuts(binding),
  };
};

/**
 * The help sheet's contents for this platform: every bound action once, by
 * group, with all of its key combinations on the one row.
 */
export const appShortcutHelp = (platform: AppShortcutPlatform): AppShortcutHelpGroup[] => {
  const boundHere = (shortcut: AppShortcut) => shortcutBinding(shortcut, platform) !== undefined;
  return APP_SHORTCUT_SCOPES.map(({ scope, label }) => ({
    scope,
    label,
    items: APP_SHORTCUTS.flatMap((shortcut) => {
      if (shortcut.scope !== scope || !boundHere(shortcut)) return [];
      // Listed on the other shortcut's row, unless that one has no binding here.
      if (shortcut.alsoFor && boundHere(appShortcut(shortcut.alsoFor))) return [];
      const bindings = [
        shortcut,
        ...APP_SHORTCUTS.filter((other) => other.alsoFor === shortcut.id),
      ].flatMap((entry) => shortcutBinding(entry, platform) ?? []);
      return [
        {
          id: shortcut.id,
          label: shortcut.label,
          keys: bindings.map((binding) => bindingLabel(binding, platform.macOS)),
          ariaKeyShortcuts: bindings.map(bindingAriaKeyShortcuts).join(' '),
        },
      ];
    }),
  })).filter((group) => group.items.length > 0);
};

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
