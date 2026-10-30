import { atom } from 'jotai';
import { normalizeAccentColor } from '../utils/themeAccent';
import { normalizeThemeBaseColor } from '../utils/themeBase';
import {
  DEFAULT_MESSAGE_TEXT_TONE,
  normalizeMessageTextTone,
  type MessageTextTone,
} from '../utils/messageTextTone';
import {
  DEFAULT_THREAD_DISPLAY,
  DEFAULT_THREAD_PANE_WIDTH,
  clampThreadPaneWidth,
  normalizeThreadDisplay,
  type ThreadDisplayMode,
} from '../utils/threadDisplay';

const SHARED_SETTINGS_STORAGE_KEY = 'settings';
const PLATFORM_SETTINGS_STORAGE_KEY = 'platformSettings';
/**
 * 2: GIF search and X posts on by default (2.5).
 * 3: on-device link previews on by default (2.8).
 */
const PREVIEW_DEFAULTS_VERSION = 3;
export type DateFormat =
  'D MMM YYYY' | 'DD/MM/YYYY' | 'MM/DD/YYYY' | 'YYYY/MM/DD' | 'YYYY-MM-DD' | '';
export type MessageSpacing = '0' | '100' | '200' | '300' | '400' | '500';
export enum MessageLayout {
  Modern = 0,
  Compact = 1,
  Bubble = 2,
}

export interface SharedSettings {
  themeId?: string;
  useSystemTheme: boolean;
  lightThemeId?: string;
  darkThemeId?: string;
  monochromeMode?: boolean;
  customAccentColor?: string;
  themeBaseColor?: string;
  messageTextTone: MessageTextTone;
  /** How a desktop room opens threads. */
  threadDisplay: ThreadDisplayMode;
  /**
   * True once the user picks a thread display. Every save writes the whole
   * snapshot, so without this a stored value could be an old default rather
   * than a choice; until it is set the current default applies.
   */
  threadDisplayChosen: boolean;
  /** Side-pane thread width in CSS px. */
  threadPaneWidth: number;
  isMarkdown: boolean;
  editorToolbar: boolean;
  twitterEmoji: boolean;
  pageZoom: number;
  hideActivity: boolean;

  isPeopleDrawer: boolean;
  memberSortFilterIndex: number;
  enterForNewline: boolean;
  messageLayout: MessageLayout;
  messageSpacing: MessageSpacing;
  hideMembershipEvents: boolean;
  hideNickAvatarEvents: boolean;
  mediaAutoLoad: boolean;
  gifSearchEnabled: boolean;
  indexedMessageSearch: boolean;
  gifOnboardingDismissed: boolean;
  /** Play Klipy and GIPHY links inline; the reader's device loads them from the provider. */
  inlineGifLinks: boolean;
  /** Show X posts, with their video, for x.com links; the reader's device contacts X. */
  xEmbeds: boolean;
  /**
   * Show a title and image card for a message's first link; this device
   * fetches the page itself, so the site sees its IP address.
   */
  linkPreviews: boolean;
  /**
   * Sync unsent composer drafts with this account's other Synara devices, end-to-end
   * encrypted in room account data. Per device.
   */
  syncDrafts: boolean;
  /**
   * The GIF, X post and link preview defaults this snapshot has taken. Every save writes the
   * whole snapshot, so a stored `false` from before 2.5 was the old default,
   * not a choice; reading such a snapshot turns them on once.
   */
  previewDefaultsVersion: number;
  showHiddenEvents: boolean;
  legacyUsernameColor: boolean;

  showNotifications: boolean;
  isNotificationSounds: boolean;
  /** Show the agent approval reason and command in OS notifications. */
  showApprovalNotificationDetails: boolean;

  hour24Clock: boolean;
  dateFormatString: string;

  developerTools: boolean;
  experimentalWidgetsEnabled: boolean;
  /** MSC4362 create/opt-in only. Already-flagged rooms still decrypt. Default on. */
  encryptedStateEvents: boolean;
}

export type AgentWidgetEntry = {
  id: string;
  name: string;
  url: string;
};

export interface DesktopPlatformSettings {
  desktopShortcutShow: string;
  desktopShortcutLater: string;
  desktopShortcutNotifications: string;
  desktopDiagnosticsEnabled: boolean;
  desktopDiagnosticsPerformance: boolean;
  desktopDiagnosticsSession: boolean;
  desktopDiagnosticsRoomState: boolean;
  desktopDiagnosticsOverlay: boolean;
  agentWidgetEntries: AgentWidgetEntry[];
}

export type PlatformSettings = DesktopPlatformSettings;
export type Settings = SharedSettings & DesktopPlatformSettings;
export type SettingsSnapshot = {
  shared: SharedSettings;
  platform: PlatformSettings;
};
export type SettingsStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
};
export type SettingsStore = {
  getSettings: () => Settings;
  setSettings: (settings: Settings) => void;
  getSharedSettings: () => SharedSettings;
  setSharedSettings: (settings: SharedSettings) => void;
  getPlatformSettings: () => PlatformSettings;
  setPlatformSettings: (settings: PlatformSettings) => void;
};

export const defaultSharedSettings: SharedSettings = {
  themeId: undefined,
  useSystemTheme: true,
  lightThemeId: undefined,
  darkThemeId: undefined,
  monochromeMode: false,
  customAccentColor: undefined,
  themeBaseColor: undefined,
  messageTextTone: DEFAULT_MESSAGE_TEXT_TONE,
  threadDisplay: DEFAULT_THREAD_DISPLAY,
  threadDisplayChosen: false,
  threadPaneWidth: DEFAULT_THREAD_PANE_WIDTH,
  isMarkdown: true,
  editorToolbar: false,
  twitterEmoji: false,
  pageZoom: 100,
  hideActivity: false,

  isPeopleDrawer: true,
  memberSortFilterIndex: 0,
  enterForNewline: false,
  messageLayout: 0,
  messageSpacing: '400',
  hideMembershipEvents: false,
  hideNickAvatarEvents: true,
  mediaAutoLoad: true,
  gifSearchEnabled: true,
  indexedMessageSearch: true,
  gifOnboardingDismissed: false,
  inlineGifLinks: true,
  xEmbeds: true,
  linkPreviews: true,
  syncDrafts: true,
  previewDefaultsVersion: PREVIEW_DEFAULTS_VERSION,
  showHiddenEvents: false,
  legacyUsernameColor: false,

  showNotifications: true,
  isNotificationSounds: true,
  showApprovalNotificationDetails: true,

  hour24Clock: false,
  dateFormatString: 'D MMM YYYY',

  developerTools: false,
  experimentalWidgetsEnabled: false,
  encryptedStateEvents: true,
};

const MAX_AGENT_WIDGET_ENTRIES = 32;
const MAX_AGENT_WIDGET_FIELD = 128;
const MAX_AGENT_WIDGET_URL = 2_048;

const sanitizeAgentWidgetEntries = (value: unknown): AgentWidgetEntry[] => {
  if (!Array.isArray(value)) return [];
  const entries: AgentWidgetEntry[] = [];
  for (const item of value) {
    if (!item || typeof item !== 'object') continue;
    const record = item as Record<string, unknown>;
    const id = typeof record.id === 'string' ? record.id.trim() : '';
    const name = typeof record.name === 'string' ? record.name.trim() : '';
    const url = typeof record.url === 'string' ? record.url.trim() : '';
    if (
      !id ||
      !name ||
      !url ||
      id.length > MAX_AGENT_WIDGET_FIELD ||
      name.length > MAX_AGENT_WIDGET_FIELD ||
      url.length > MAX_AGENT_WIDGET_URL
    ) {
      continue;
    }
    entries.push({ id, name, url });
    if (entries.length >= MAX_AGENT_WIDGET_ENTRIES) break;
  }
  return entries;
};

const sanitizePlatformSettings = (settings: DesktopPlatformSettings): DesktopPlatformSettings => ({
  ...settings,
  agentWidgetEntries: sanitizeAgentWidgetEntries(settings.agentWidgetEntries),
});

/**
 * Global shortcuts take their keys from every other app while Synara runs, so
 * the defaults use three modifiers. An empty value turns a shortcut off.
 */
export const DEFAULT_DESKTOP_SHORTCUTS = {
  desktopShortcutShow: 'CmdOrCtrl+Alt+Shift+M',
  desktopShortcutLater: 'CmdOrCtrl+Alt+Shift+L',
  desktopShortcutNotifications: 'CmdOrCtrl+Alt+Shift+N',
} as const;

/** The defaults before 2.3; they took Copy in Linux terminals and New Folder in Finder. */
const RETIRED_DESKTOP_SHORTCUTS = {
  desktopShortcutShow: 'CmdOrCtrl+Shift+C',
  desktopShortcutLater: 'CmdOrCtrl+Shift+L',
  desktopShortcutNotifications: 'CmdOrCtrl+Shift+N',
} as const;

/**
 * Snapshots saved before 2.5 hold GIF search and X posts off, the old
 * defaults, and snapshots saved before 2.8 have no link previews, so each
 * moves to the newer defaults once. A snapshot already at 2 keeps its GIF and
 * X post choices and only gains link previews.
 */
const migratePreviewDefaults = (
  settings: SharedSettings,
  stored: Record<string, unknown> | undefined
): SharedSettings => {
  if (!stored) return settings;
  const storedVersion =
    typeof stored.previewDefaultsVersion === 'number' ? stored.previewDefaultsVersion : 0;
  // Written by this version or a newer one (after a downgrade): its choices stand.
  if (storedVersion >= PREVIEW_DEFAULTS_VERSION) return settings;
  return {
    ...settings,
    ...(storedVersion < 2 ? { gifSearchEnabled: true, inlineGifLinks: true, xEmbeds: true } : {}),
    linkPreviews: true,
    previewDefaultsVersion: PREVIEW_DEFAULTS_VERSION,
  };
};

/**
 * A stored set equal to the retired defaults was never customised (it is
 * written with the rest of the settings), so it moves to the new defaults.
 */
const migrateRetiredShortcutDefaults = (
  settings: DesktopPlatformSettings
): DesktopPlatformSettings =>
  settings.desktopShortcutShow === RETIRED_DESKTOP_SHORTCUTS.desktopShortcutShow &&
  settings.desktopShortcutLater === RETIRED_DESKTOP_SHORTCUTS.desktopShortcutLater &&
  settings.desktopShortcutNotifications === RETIRED_DESKTOP_SHORTCUTS.desktopShortcutNotifications
    ? { ...settings, ...DEFAULT_DESKTOP_SHORTCUTS }
    : settings;

export const defaultDesktopPlatformSettings: DesktopPlatformSettings = {
  ...DEFAULT_DESKTOP_SHORTCUTS,
  desktopDiagnosticsEnabled: false,
  desktopDiagnosticsPerformance: false,
  desktopDiagnosticsSession: false,
  desktopDiagnosticsRoomState: false,
  desktopDiagnosticsOverlay: false,
  agentWidgetEntries: [],
};

export const defaultPlatformSettings: PlatformSettings = {
  ...defaultDesktopPlatformSettings,
};
export const defaultSettings: Settings = {
  ...defaultSharedSettings,
  ...defaultPlatformSettings,
};

const unavailableSettingsStorage: SettingsStorage = {
  getItem: () => null,
  setItem: () => undefined,
};

const getDefaultSettingsStorage = (): SettingsStorage =>
  typeof localStorage === 'undefined' ? unavailableSettingsStorage : localStorage;

const readStoredSettings = (
  storage: SettingsStorage,
  key: string
): Record<string, unknown> | undefined => {
  const value = storage.getItem(key);
  if (value === null) return undefined;

  try {
    const parsed = JSON.parse(value);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return undefined;
    return parsed as Record<string, unknown>;
  } catch {
    return undefined;
  }
};

const pickKnownSettings = <T extends object>(defaults: T, source: object | undefined): T => {
  const settings = { ...defaults };
  if (!source) return settings;

  (Object.keys(defaults) as Array<keyof T>).forEach((key) => {
    const value: unknown = Reflect.get(source, key);
    if (value !== undefined) {
      settings[key] = value as T[typeof key];
    }
  });

  return settings;
};

const sanitizeSharedSettings = (settings: SharedSettings): SharedSettings => ({
  ...settings,
  customAccentColor: normalizeAccentColor(
    typeof settings.customAccentColor === 'string' ? settings.customAccentColor : undefined
  ),
  themeBaseColor: normalizeThemeBaseColor(
    typeof settings.themeBaseColor === 'string' ? settings.themeBaseColor : undefined
  ),
  messageTextTone: normalizeMessageTextTone(settings.messageTextTone),
  threadDisplay:
    settings.threadDisplayChosen === true
      ? normalizeThreadDisplay(settings.threadDisplay)
      : DEFAULT_THREAD_DISPLAY,
  threadDisplayChosen: settings.threadDisplayChosen === true,
  threadPaneWidth: clampThreadPaneWidth(settings.threadPaneWidth),
});

export const mergeSettingsSnapshot = (snapshot: SettingsSnapshot): Settings => ({
  ...snapshot.shared,
  ...snapshot.platform,
});

export const splitSettings = (settings: Settings): SettingsSnapshot => ({
  shared: sanitizeSharedSettings(pickKnownSettings(defaultSharedSettings, settings)),
  platform: sanitizePlatformSettings(pickKnownSettings(defaultPlatformSettings, settings)),
});

export const createLocalStorageSettingsStore = (storage: SettingsStorage): SettingsStore => {
  const getSnapshot = (): SettingsSnapshot => {
    const legacyOrSharedSettings = readStoredSettings(storage, SHARED_SETTINGS_STORAGE_KEY);
    const platformSettings = readStoredSettings(storage, PLATFORM_SETTINGS_STORAGE_KEY);

    return {
      shared: sanitizeSharedSettings(
        migratePreviewDefaults(
          pickKnownSettings(defaultSharedSettings, legacyOrSharedSettings),
          legacyOrSharedSettings
        )
      ),
      platform: sanitizePlatformSettings(
        migrateRetiredShortcutDefaults(
          pickKnownSettings(defaultPlatformSettings, {
            ...legacyOrSharedSettings,
            ...platformSettings,
          })
        )
      ),
    };
  };

  const setSnapshot = (snapshot: SettingsSnapshot): void => {
    storage.setItem(
      SHARED_SETTINGS_STORAGE_KEY,
      JSON.stringify(sanitizeSharedSettings(snapshot.shared))
    );
    storage.setItem(
      PLATFORM_SETTINGS_STORAGE_KEY,
      JSON.stringify(sanitizePlatformSettings(snapshot.platform))
    );
  };

  return {
    getSettings: () => mergeSettingsSnapshot(getSnapshot()),
    setSettings: (settings) => {
      setSnapshot(splitSettings(settings));
    },
    getSharedSettings: () => getSnapshot().shared,
    setSharedSettings: (settings) => {
      setSnapshot({ shared: settings, platform: getSnapshot().platform });
    },
    getPlatformSettings: () => getSnapshot().platform,
    setPlatformSettings: (settings) => {
      setSnapshot({ shared: getSnapshot().shared, platform: settings });
    },
  };
};

export const settingsStore: SettingsStore = {
  getSettings: () => createLocalStorageSettingsStore(getDefaultSettingsStorage()).getSettings(),
  setSettings: (settings) =>
    createLocalStorageSettingsStore(getDefaultSettingsStorage()).setSettings(settings),
  getSharedSettings: () =>
    createLocalStorageSettingsStore(getDefaultSettingsStorage()).getSharedSettings(),
  setSharedSettings: (settings) =>
    createLocalStorageSettingsStore(getDefaultSettingsStorage()).setSharedSettings(settings),
  getPlatformSettings: () =>
    createLocalStorageSettingsStore(getDefaultSettingsStorage()).getPlatformSettings(),
  setPlatformSettings: (settings) =>
    createLocalStorageSettingsStore(getDefaultSettingsStorage()).setPlatformSettings(settings),
};

export const getSettings = () => {
  return settingsStore.getSettings();
};

/**
 * The user's receipt privacy mode: read receipts are public `m.read` unless
 * "Hide Typing & Read Receipts" is on, when Core sends `m.read.private`.
 */
export const readReceiptsArePublic = (): boolean => !getSettings().hideActivity;

export const setSettings = (settings: Settings) => {
  settingsStore.setSettings(settings);
};

export const getSharedSettings = () => {
  return settingsStore.getSharedSettings();
};

export const setSharedSettings = (settings: SharedSettings) => {
  settingsStore.setSharedSettings(settings);
};

export const getPlatformSettings = () => {
  return settingsStore.getPlatformSettings();
};

export const setPlatformSettings = (settings: PlatformSettings) => {
  settingsStore.setPlatformSettings(settings);
};

const baseSharedSettings = atom<SharedSettings>(getSharedSettings());
const basePlatformSettings = atom<PlatformSettings>(getPlatformSettings());

export const settingsAtom = atom<Settings, [Settings], undefined>(
  (get) =>
    mergeSettingsSnapshot({
      shared: get(baseSharedSettings),
      platform: get(basePlatformSettings),
    }),
  (get, set, update) => {
    const snapshot = splitSettings(update);
    set(baseSharedSettings, snapshot.shared);
    set(basePlatformSettings, snapshot.platform);
    settingsStore.setSettings(update);
  }
);

export const sharedSettingsAtom = atom<SharedSettings, [SharedSettings], undefined>(
  (get) => get(baseSharedSettings),
  (get, set, update) => {
    set(baseSharedSettings, update);
    settingsStore.setSettings(
      mergeSettingsSnapshot({
        shared: update,
        platform: get(basePlatformSettings),
      })
    );
  }
);

export const desktopPlatformSettingsAtom = atom<
  DesktopPlatformSettings,
  [DesktopPlatformSettings],
  undefined
>(
  (get) => get(basePlatformSettings),
  (get, set, update) => {
    set(basePlatformSettings, update);
    settingsStore.setSettings(
      mergeSettingsSnapshot({
        shared: get(baseSharedSettings),
        platform: update,
      })
    );
  }
);
