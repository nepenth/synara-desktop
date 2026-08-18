import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearLegacyRendererSessionCredentials,
  clearSessionLocalStorage,
  FALLBACK_SESSION_KEYS,
  SESSION_LOCAL_STORAGE_EXACT_KEYS,
  type SessionLocalStorage,
} from '../sessions';

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
