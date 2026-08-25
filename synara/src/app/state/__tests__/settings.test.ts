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
  assert.equal(defaultSettings.desktopDiagnosticsEnabled, false);
  assert.equal(defaultSettings.desktopDiagnosticsPerformance, false);
  assert.equal(defaultSettings.desktopDiagnosticsSession, false);
  assert.equal(defaultSettings.desktopDiagnosticsRoomState, false);
  assert.equal(defaultSettings.desktopDiagnosticsOverlay, false);
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
  assert.equal('desktopShortcutShow' in store.getSharedSettings(), false);
  assert.deepEqual(store.getPlatformSettings(), {
    desktopShortcutShow: 'CmdOrCtrl+1',
    desktopShortcutLater: 'CmdOrCtrl+2',
    desktopShortcutNotifications: 'CmdOrCtrl+3',
    desktopDiagnosticsEnabled: false,
    desktopDiagnosticsPerformance: false,
    desktopDiagnosticsSession: false,
    desktopDiagnosticsRoomState: false,
    desktopDiagnosticsOverlay: false,
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
    desktopDiagnosticsEnabled: true,
    desktopDiagnosticsPerformance: true,
    desktopDiagnosticsSession: true,
    desktopDiagnosticsRoomState: true,
    desktopDiagnosticsOverlay: true,
  });

  const shared = storage.getObject('settings');
  const platform = storage.getObject('platformSettings');

  assert.equal(shared?.themeId, 'aurora');
  assert.equal(shared?.desktopShortcutShow, undefined);
  assert.deepEqual(platform, {
    desktopShortcutShow: 'CmdOrCtrl+1',
    desktopShortcutLater: 'CmdOrCtrl+2',
    desktopShortcutNotifications: 'CmdOrCtrl+3',
    desktopDiagnosticsEnabled: true,
    desktopDiagnosticsPerformance: true,
    desktopDiagnosticsSession: true,
    desktopDiagnosticsRoomState: true,
    desktopDiagnosticsOverlay: true,
  });
});

test('desktop diagnostic settings stay out of the shared settings payload', () => {
  const storage = createMemoryStorage();
  const store = createLocalStorageSettingsStore(storage);

  store.setSettings({
    ...defaultSettings,
    desktopDiagnosticsEnabled: true,
    desktopDiagnosticsRoomState: true,
  });

  const shared = storage.getObject('settings');
  const platform = storage.getObject('platformSettings');
  assert.equal(shared?.desktopDiagnosticsEnabled, undefined);
  assert.equal(shared?.desktopDiagnosticsRoomState, undefined);
  assert.equal(platform?.desktopDiagnosticsEnabled, true);
  assert.equal(platform?.desktopDiagnosticsRoomState, true);
});

test('shared settings persist a user-selected theme base color', () => {
  const storage = createMemoryStorage();
  const store = createLocalStorageSettingsStore(storage);

  store.setSettings({
    ...defaultSettings,
    themeBaseColor: '#5865f2',
  });

  const shared = storage.getObject('settings');
  const platform = storage.getObject('platformSettings');

  assert.equal(shared?.themeBaseColor, '#5865f2');
  assert.equal(platform?.themeBaseColor, undefined);
  assert.equal(store.getSharedSettings().themeBaseColor, '#5865f2');
});

test('message text tone defaults bright and persists only valid shared choices', () => {
  const storage = createMemoryStorage();
  const store = createLocalStorageSettingsStore(storage);

  assert.equal(store.getSharedSettings().messageTextTone, 'bright');
  store.setSettings({ ...defaultSettings, messageTextTone: 'soft' });
  assert.equal(storage.getObject('settings')?.messageTextTone, 'soft');
  assert.equal(storage.getObject('platformSettings')?.messageTextTone, undefined);

  const poisoned = createLocalStorageSettingsStore(
    createMemoryStorage({
      settings: JSON.stringify({ ...defaultSharedSettings, messageTextTone: 'blinding' }),
    })
  );
  assert.equal(poisoned.getSharedSettings().messageTextTone, 'bright');
});

test('invalid theme base colors are dropped instead of persisted', () => {
  const storage = createMemoryStorage();
  const store = createLocalStorageSettingsStore(storage);

  store.setSettings({
    ...defaultSettings,
    themeBaseColor: 'aabbcc',
  });

  const shared = storage.getObject('settings');
  assert.equal(shared?.themeBaseColor, undefined);
  assert.equal(store.getSharedSettings().themeBaseColor, undefined);

  const poisoned = createLocalStorageSettingsStore(
    createMemoryStorage({
      settings: JSON.stringify({
        ...defaultSharedSettings,
        themeBaseColor: 'red',
      }),
    })
  );
  assert.equal(poisoned.getSharedSettings().themeBaseColor, undefined);
});

test('splitSettings and mergeSettingsSnapshot round-trip known settings', () => {
  const settings = {
    ...defaultSettings,
    lightThemeId: 'paper',
    desktopShortcutShow: 'CmdOrCtrl+1',
  };

  assert.deepEqual(mergeSettingsSnapshot(splitSettings(settings)), settings);
});
