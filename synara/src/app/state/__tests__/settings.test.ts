import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createLocalStorageSettingsStore,
  defaultDesktopPlatformSettings,
  defaultSettings,
  defaultSharedSettings,
  mergeSettingsSnapshot,
  splitSettings,
  type SettingsStorage,
} from '../settings';

type MemoryStorage = SettingsStorage & {
  getObject: (key: string) => Record<string, unknown> | undefined;
};

const createMemoryStorage = (initialValues: Record<string, string> = {}): MemoryStorage => {
  const values = new Map(Object.entries(initialValues));

  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    getObject: (key) => {
      const value = values.get(key);
      if (!value) return undefined;
      return JSON.parse(value) as Record<string, unknown>;
    },
  };
};

test('modernization opt-in settings default off', () => {
  const store = createLocalStorageSettingsStore(createMemoryStorage());

  assert.equal(defaultSettings.gifSearchEnabled, false);
  assert.equal(defaultSettings.gifOnboardingDismissed, false);
  assert.equal(defaultSettings.timelineVirtualizationEnabled, true);
  assert.equal(store.getSettings().gifSearchEnabled, false);
  assert.equal(store.getSettings().timelineVirtualizationEnabled, true);
});

test('shared settings defaults exclude desktop shortcut settings', () => {
  assert.equal('desktopShortcutShow' in defaultSharedSettings, false);
  assert.equal(
    defaultDesktopPlatformSettings.desktopShortcutShow,
    defaultSettings.desktopShortcutShow
  );
});

test('settings store reads legacy desktop shortcuts from the shared settings blob', () => {
  const store = createLocalStorageSettingsStore(
    createMemoryStorage({
      settings: JSON.stringify({
        themeId: 'aurora',
        timelineVirtualizationEnabled: false,
        desktopShortcutShow: 'CmdOrCtrl+1',
        desktopShortcutLater: 'CmdOrCtrl+2',
        desktopShortcutNotifications: 'CmdOrCtrl+3',
      }),
    })
  );

  assert.equal(store.getSharedSettings().themeId, 'aurora');
  assert.equal(store.getSharedSettings().timelineVirtualizationEnabled, false);
  assert.equal(
    (store.getSharedSettings() as Record<string, unknown>).desktopShortcutShow,
    undefined
  );
  assert.deepEqual(store.getPlatformSettings(), {
    desktopShortcutShow: 'CmdOrCtrl+1',
    desktopShortcutLater: 'CmdOrCtrl+2',
    desktopShortcutNotifications: 'CmdOrCtrl+3',
  });
  assert.equal(store.getSettings().desktopShortcutNotifications, 'CmdOrCtrl+3');
});

test('settings store writes shared and desktop platform settings separately', () => {
  const storage = createMemoryStorage();
  const store = createLocalStorageSettingsStore(storage);

  store.setSettings({
    ...defaultSettings,
    themeId: 'aurora',
    desktopShortcutShow: 'CmdOrCtrl+1',
    desktopShortcutLater: 'CmdOrCtrl+2',
    desktopShortcutNotifications: 'CmdOrCtrl+3',
  });

  const shared = storage.getObject('settings');
  const platform = storage.getObject('platformSettings');

  assert.equal(shared?.themeId, 'aurora');
  assert.equal(shared?.desktopShortcutShow, undefined);
  assert.deepEqual(platform, {
    desktopShortcutShow: 'CmdOrCtrl+1',
    desktopShortcutLater: 'CmdOrCtrl+2',
    desktopShortcutNotifications: 'CmdOrCtrl+3',
  });
});

test('splitSettings and mergeSettingsSnapshot round-trip known settings', () => {
  const settings = {
    ...defaultSettings,
    lightThemeId: 'paper',
    desktopShortcutShow: 'CmdOrCtrl+1',
  };

  assert.deepEqual(mergeSettingsSnapshot(splitSettings(settings)), settings);
});
