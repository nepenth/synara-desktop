import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearLegacyRendererSessionCredentials,
  clearSessionLocalStorage,
  FALLBACK_SESSION_KEYS,
  SESSION_LOCAL_STORAGE_EXACT_KEYS,
  type SessionLocalStorage,
} from '../sessions';
import { getRoomDraftStorageKey } from '../../utils/drafts';

const createMemoryStorage = (initialValues: Record<string, string> = {}): SessionLocalStorage => {
  const values = new Map(Object.entries(initialValues));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
    get length() {
      return values.size;
    },
    key: (index) => Array.from(values.keys())[index] ?? null,
  };
};

test('bootstrap cleanup purges retired renderer credential keys only', () => {
  const storage = createMemoryStorage({
    synara_access_token: 'retired-token',
    synara_device_id: 'DEVICE',
    synara_user_id: '@alice:example.org',
    synara_hs_base_url: 'https://matrix.example.org',
    synara_session_generation: 'generation',
    settings: '{"themeId":"aurora"}',
  });

  clearLegacyRendererSessionCredentials(storage);

  Object.values(FALLBACK_SESSION_KEYS).forEach((key) => assert.equal(storage.getItem(key), null));
  assert.equal(storage.getItem('settings'), '{"themeId":"aurora"}');
});

test('logout cleanup removes documented session keys and navigation only', () => {
  const storage = createMemoryStorage({
    synara_access_token: 'retired-token',
    synara_device_id: 'DEVICE',
    synara_user_id: '@alice:example.org',
    synara_hs_base_url: 'https://matrix.example.org',
    after_login_redirect_url: '/room/123',
    'navToActivePath@alice:example.org': '{"home":{"pathname":"/home"}}',
    settings: '{"themeId":"aurora"}',
    platformSettings: '{"desktopShortcutShow":"CmdOrCtrl+1"}',
  });

  clearSessionLocalStorage(storage);

  SESSION_LOCAL_STORAGE_EXACT_KEYS.forEach((key) => assert.equal(storage.getItem(key), null));
  assert.equal(storage.getItem('navToActivePath@alice:example.org'), null);
  assert.equal(storage.getItem('settings'), '{"themeId":"aurora"}');
  assert.equal(storage.getItem('platformSettings'), '{"desktopShortcutShow":"CmdOrCtrl+1"}');
});

test('logout cleanup removes unsent drafts and per-user layout state', () => {
  const draftKey = getRoomDraftStorageKey('@alice:example.org', '!room:example.org');
  const storage = createMemoryStorage({
    [draftKey]: '{"version":1,"updatedAt":1,"value":[]}',
    'openedSidebarFolder@alice:example.org': '["folder"]',
    'closedNavCategories@alice:example.org': '["home"]',
    'closedLobbyCategories@alice:example.org': '["!space:example.org"]',
    spaceRooms: '["!room:example.org"]',
    settings: '{"themeId":"aurora"}',
    'synara.navigationPaneWidth': '320',
  });

  clearSessionLocalStorage(storage);

  assert.equal(storage.getItem(draftKey), null);
  assert.equal(storage.getItem('openedSidebarFolder@alice:example.org'), null);
  assert.equal(storage.getItem('closedNavCategories@alice:example.org'), null);
  assert.equal(storage.getItem('closedLobbyCategories@alice:example.org'), null);
  assert.equal(storage.getItem('spaceRooms'), null);
  // Device-level preferences stay.
  assert.equal(storage.getItem('settings'), '{"themeId":"aurora"}');
  assert.equal(storage.getItem('synara.navigationPaneWidth'), '320');
});
