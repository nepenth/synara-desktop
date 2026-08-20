import { atom } from 'jotai';

const SHARED_SETTINGS_STORAGE_KEY = 'settings';
const PLATFORM_SETTINGS_STORAGE_KEY = 'platformSettings';
export type DateFormat =
  | 'D MMM YYYY'
  | 'DD/MM/YYYY'
  | 'MM/DD/YYYY'
  | 'YYYY/MM/DD'
  | 'YYYY-MM-DD'
  | '';
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
  gifOnboardingDismissed: boolean;
  timelineVirtualizationEnabled: boolean;
  showHiddenEvents: boolean;
  legacyUsernameColor: boolean;

  showNotifications: boolean;
  isNotificationSounds: boolean;

  hour24Clock: boolean;
  dateFormatString: string;

  developerTools: boolean;
}

export interface DesktopPlatformSettings {
  desktopShortcutShow: string;
  desktopShortcutLater: string;
  desktopShortcutNotifications: string;
  desktopDiagnosticsEnabled: boolean;
  desktopDiagnosticsPerformance: boolean;
  desktopDiagnosticsSession: boolean;
  desktopDiagnosticsRoomState: boolean;
  desktopDiagnosticsOverlay: boolean;
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
  gifSearchEnabled: false,
  gifOnboardingDismissed: false,
  timelineVirtualizationEnabled: true,
  showHiddenEvents: false,
  legacyUsernameColor: false,

  showNotifications: true,
  isNotificationSounds: true,

  hour24Clock: false,
  dateFormatString: 'D MMM YYYY',

  developerTools: false,
};

export const defaultDesktopPlatformSettings: DesktopPlatformSettings = {
  desktopShortcutShow: 'CmdOrCtrl+Shift+C',
  desktopShortcutLater: 'CmdOrCtrl+Shift+L',
  desktopShortcutNotifications: 'CmdOrCtrl+Shift+N',
  desktopDiagnosticsEnabled: false,
  desktopDiagnosticsPerformance: false,
  desktopDiagnosticsSession: false,
  desktopDiagnosticsRoomState: false,
  desktopDiagnosticsOverlay: false,
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

export const mergeSettingsSnapshot = (snapshot: SettingsSnapshot): Settings => ({
  ...snapshot.shared,
  ...snapshot.platform,
});

export const splitSettings = (settings: Settings): SettingsSnapshot => ({
  shared: pickKnownSettings(defaultSharedSettings, settings),
  platform: pickKnownSettings(defaultPlatformSettings, settings),
});

export const createLocalStorageSettingsStore = (storage: SettingsStorage): SettingsStore => {
  const getSnapshot = (): SettingsSnapshot => {
    const legacyOrSharedSettings = readStoredSettings(storage, SHARED_SETTINGS_STORAGE_KEY);
    const platformSettings = readStoredSettings(storage, PLATFORM_SETTINGS_STORAGE_KEY);

    return {
      shared: pickKnownSettings(defaultSharedSettings, legacyOrSharedSettings),
      platform: pickKnownSettings(defaultPlatformSettings, {
        ...legacyOrSharedSettings,
        ...platformSettings,
      }),
    };
  };

  const setSnapshot = (snapshot: SettingsSnapshot): void => {
    storage.setItem(SHARED_SETTINGS_STORAGE_KEY, JSON.stringify(snapshot.shared));
    storage.setItem(PLATFORM_SETTINGS_STORAGE_KEY, JSON.stringify(snapshot.platform));
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
