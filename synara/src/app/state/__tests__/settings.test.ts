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

test('encrypted state events setting defaults on and is shared', () => {
  const store = createLocalStorageSettingsStore(createMemoryStorage());
  assert.equal(defaultSharedSettings.encryptedStateEvents, true);
  assert.equal(defaultSettings.encryptedStateEvents, true);
  assert.equal(store.getSharedSettings().encryptedStateEvents, true);
  store.setSettings({ ...defaultSettings, encryptedStateEvents: false });
  assert.equal(store.getSharedSettings().encryptedStateEvents, false);
});

test('modernization opt-in settings default off', () => {
  const store = createLocalStorageSettingsStore(createMemoryStorage());

  assert.equal(defaultSettings.experimentalWidgetsEnabled, false);
  assert.deepEqual(defaultSettings.agentWidgetEntries, []);
  assert.equal(store.getSettings().experimentalWidgetsEnabled, false);
  assert.deepEqual(store.getSettings().agentWidgetEntries, []);
  assert.equal(defaultSettings.indexedMessageSearch, true);
  assert.equal(defaultSettings.gifOnboardingDismissed, false);
  assert.equal(store.getSettings().indexedMessageSearch, true);
  assert.equal('timelineVirtualizationEnabled' in store.getSettings(), false);
  assert.equal(defaultSettings.desktopDiagnosticsEnabled, false);
  assert.equal(defaultSettings.desktopDiagnosticsPerformance, false);
  assert.equal(defaultSettings.desktopDiagnosticsSession, false);
  assert.equal(defaultSettings.desktopDiagnosticsRoomState, false);
  assert.equal(defaultSettings.desktopDiagnosticsOverlay, false);
});

test('GIF search, inline GIF links and X posts default on', () => {
  const store = createLocalStorageSettingsStore(createMemoryStorage());

  assert.equal(defaultSettings.gifSearchEnabled, true);
  assert.equal(defaultSettings.inlineGifLinks, true);
  assert.equal(defaultSettings.xEmbeds, true);
  assert.equal(defaultSettings.linkPreviews, true);
  assert.equal(store.getSettings().linkPreviews, true);
  assert.equal(store.getSettings().gifSearchEnabled, true);
  assert.equal(store.getSettings().xEmbeds, true);
});

test('settings saved before 2.5 turn GIFs and X posts on once; later choices stay', () => {
  const storage = createMemoryStorage({
    settings: JSON.stringify({ gifSearchEnabled: false, inlineGifLinks: false, xEmbeds: false }),
  });
  const store = createLocalStorageSettingsStore(storage);
  const migrated = store.getSharedSettings();
  assert.equal(migrated.gifSearchEnabled, true);
  assert.equal(migrated.inlineGifLinks, true);
  assert.equal(migrated.xEmbeds, true);
  assert.equal(migrated.linkPreviews, true);

  // Turning one off afterwards is a choice and survives the next read.
  store.setSharedSettings({ ...migrated, xEmbeds: false });
  assert.equal(storage.getObject('settings')?.previewDefaultsVersion, 3);
  assert.equal(store.getSharedSettings().xEmbeds, false);
  assert.equal(store.getSharedSettings().gifSearchEnabled, true);
});

test('settings saved by 2.5 to 2.7 turn link previews on once and keep GIF and X choices', () => {
  const storage = createMemoryStorage({
    settings: JSON.stringify({
      gifSearchEnabled: false,
      inlineGifLinks: false,
      xEmbeds: false,
      linkPreviews: false,
      previewDefaultsVersion: 2,
    }),
  });
  const store = createLocalStorageSettingsStore(storage);
  const migrated = store.getSharedSettings();
  assert.equal(migrated.linkPreviews, true);
  assert.equal(migrated.gifSearchEnabled, false);
  assert.equal(migrated.inlineGifLinks, false);
  assert.equal(migrated.xEmbeds, false);

  store.setSharedSettings({ ...migrated, linkPreviews: false });
  assert.equal(storage.getObject('settings')?.previewDefaultsVersion, 3);
  assert.equal(store.getSharedSettings().linkPreviews, false);
});

test('shared settings defaults exclude desktop shortcut settings', () => {
  assert.equal('desktopShortcutShow' in defaultSharedSettings, false);
  assert.equal(
    defaultDesktopPlatformSettings.desktopShortcutShow,
    defaultSettings.desktopShortcutShow
  );
});

test('desktop shortcut defaults use three modifiers and stay on', () => {
  assert.deepEqual(
    [
      defaultSettings.desktopShortcutShow,
      defaultSettings.desktopShortcutLater,
      defaultSettings.desktopShortcutNotifications,
    ],
    ['CmdOrCtrl+Alt+Shift+M', 'CmdOrCtrl+Alt+Shift+L', 'CmdOrCtrl+Alt+Shift+N']
  );
});

test('stored pre-2.3 shortcut defaults move to the new defaults; custom and cleared ones stay', () => {
  const retired = createLocalStorageSettingsStore(
    createMemoryStorage({
      platformSettings: JSON.stringify({
        desktopShortcutShow: 'CmdOrCtrl+Shift+C',
        desktopShortcutLater: 'CmdOrCtrl+Shift+L',
        desktopShortcutNotifications: 'CmdOrCtrl+Shift+N',
      }),
    })
  );
  assert.equal(retired.getPlatformSettings().desktopShortcutShow, 'CmdOrCtrl+Alt+Shift+M');
  assert.equal(retired.getPlatformSettings().desktopShortcutNotifications, 'CmdOrCtrl+Alt+Shift+N');

  const customised = createLocalStorageSettingsStore(
    createMemoryStorage({
      platformSettings: JSON.stringify({
        desktopShortcutShow: 'CmdOrCtrl+Shift+C',
        desktopShortcutLater: '',
        desktopShortcutNotifications: 'CmdOrCtrl+Shift+N',
      }),
    })
  );
  assert.equal(customised.getPlatformSettings().desktopShortcutShow, 'CmdOrCtrl+Shift+C');
  // An empty value is a turned-off shortcut, not a missing one.
  assert.equal(customised.getPlatformSettings().desktopShortcutLater, '');
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
  // A retired key in the stored blob is ignored, not carried forward.
  assert.equal('timelineVirtualizationEnabled' in store.getSharedSettings(), false);
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
    agentWidgetEntries: [],
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
    agentWidgetEntries: [],
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
