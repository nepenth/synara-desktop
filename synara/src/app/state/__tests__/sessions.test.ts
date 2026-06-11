import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearSessionLocalStorage,
  createLocalStorageSessionStore,
  SESSION_LOCAL_STORAGE_EXACT_KEYS,
  type SessionLocalStorage,
  type SessionStorage,
} from '../sessions';

const createMemoryStorage = (initialValues: Record<string, string> = {}): SessionStorage => {
  const values = new Map(Object.entries(initialValues));

  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
  };
};

const createEnumeratedMemoryStorage = (
  initialValues: Record<string, string> = {}
): SessionLocalStorage => {
  const values = new Map(Object.entries(initialValues));

  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
    removeItem: (key) => {
      values.delete(key);
    },
    get length() {
      return values.size;
    },
    key: (index) => Array.from(values.keys())[index] ?? null,
  };
};

test('local storage session store persists the fallback session shape', () => {
  const store = createLocalStorageSessionStore(createMemoryStorage());

  store.setFallbackSession({
    accessToken: 'token',
    deviceId: 'DEVICE',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
  });

  assert.deepEqual(store.getFallbackSession(), {
    accessToken: 'token',
    deviceId: 'DEVICE',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
    fallbackSdkStores: true,
  });
});

test('local storage session store returns undefined for incomplete fallback sessions', () => {
  const store = createLocalStorageSessionStore(
    createMemoryStorage({
      synara_access_token: 'token',
      synara_user_id: '@alice:example.org',
    })
  );

  assert.equal(store.getFallbackSession(), undefined);
});

test('clearSessionLocalStorage removes documented session keys only', () => {
  const storage = createEnumeratedMemoryStorage({
    synara_access_token: 'token',
    synara_device_id: 'DEVICE',
    synara_user_id: '@alice:example.org',
    synara_hs_base_url: 'https://matrix.example.org',
    after_login_redirect_url: '/room/123',
    'navToActivePath@alice:example.org': '{"home":{"pathname":"/home"}}',
    settings: JSON.stringify({ themeId: 'aurora', pageZoom: 120 }),
    platformSettings: JSON.stringify({ desktopShortcutShow: 'CmdOrCtrl+1' }),
    'synara.performance.debug': 'true',
  });

  clearSessionLocalStorage(storage);

  SESSION_LOCAL_STORAGE_EXACT_KEYS.forEach((key) => {
    assert.equal(storage.getItem(key), null, `expected session key ${key} to be removed`);
  });
  assert.equal(storage.getItem('navToActivePath@alice:example.org'), null);
  assert.equal(storage.getItem('settings'), JSON.stringify({ themeId: 'aurora', pageZoom: 120 }));
  assert.equal(
    storage.getItem('platformSettings'),
    JSON.stringify({ desktopShortcutShow: 'CmdOrCtrl+1' })
  );
  assert.equal(storage.getItem('synara.performance.debug'), 'true');
});

test('local storage session store removes all fallback session fields', () => {
  const store = createLocalStorageSessionStore(createMemoryStorage());
  store.setFallbackSession({
    accessToken: 'token',
    deviceId: 'DEVICE',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
  });

  store.removeFallbackSession();

  assert.equal(store.getFallbackSession(), undefined);
});
