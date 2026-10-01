import test from 'node:test';
import assert from 'node:assert/strict';
// Mock client type: local structural projection (js-sdk MatrixClient type no longer imported).
import { reloadApplication, performLogout } from '../../../client/initMatrix';
import {
  notifiedEventIdsCache,
  unreadNotificationCache,
} from '../../notifications/notificationCaches';
import {
  clearSessionLocalStorage,
  SESSION_LOCAL_STORAGE_EXACT_KEYS,
  type SessionLocalStorage,
} from '../sessions';

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

const createMockMatrixClient = () => {
  const calls: string[] = [];

  const mx = {
    stopClient: () => {
      calls.push('stopClient');
    },
    logout: async () => {
      calls.push('logout');
    },
    clearStores: async () => {
      calls.push('clearStores');
    },
  } as unknown as any;

  return { mx, calls };
};

const createLogoutDeps = () => {
  const clearPersistedCalls: void[] = [];
  let nativeLogoutCalls = 0;
  let reloaded = false;

  const deps = {
    clearPersistedSessions: async () => {
      clearPersistedCalls.push(undefined);
    },
    clearSessionLocalStorage: () => undefined,
    logoutNativeSession: async () => {
      nativeLogoutCalls += 1;
    },
    reload: () => {
      reloaded = true;
    },
  };

  return {
    deps,
    clearPersistedCalls,
    getNativeLogoutCalls: () => nativeLogoutCalls,
    getReloaded: () => reloaded,
  };
};

test('performLogout clears renderer state only after native logout completes', async () => {
  const { mx, calls } = createMockMatrixClient();
  const { deps, clearPersistedCalls, getReloaded } = createLogoutDeps();

  await performLogout(mx, {
    ...deps,
    clearPersistedSessions: async () => {
      calls.push('clearPersistedSessions');
      await deps.clearPersistedSessions();
    },
  });

  assert.deepEqual(calls, ['logout', 'clearPersistedSessions']);
  assert.equal(clearPersistedCalls.length, 1);
  assert.equal(getReloaded(), true);
});

test('performLogout without matrix client clears the native and renderer sessions', async () => {
  const { deps, clearPersistedCalls, getNativeLogoutCalls, getReloaded } = createLogoutDeps();

  await performLogout(undefined, deps);

  assert.equal(clearPersistedCalls.length, 1);
  assert.equal(getNativeLogoutCalls(), 1);
  assert.equal(getReloaded(), true);
});

test('performLogout clears bounded notification caches', async () => {
  notifiedEventIdsCache.add('$approval-event');
  unreadNotificationCache.set('!room:example.org', {
    roomId: '!room:example.org',
    total: 1,
    highlight: 0,
  });

  await performLogout(undefined, createLogoutDeps().deps);

  assert.equal(notifiedEventIdsCache.size, 0);
  assert.equal(unreadNotificationCache.size, 0);
});

test('performLogout without matrix client removes session keys only', async () => {
  const storage = createEnumeratedMemoryStorage({
    synara_access_token: 'token',
    synara_device_id: 'DEVICE',
    synara_user_id: '@alice:example.org',
    synara_hs_base_url: 'https://matrix.example.org',
    after_login_redirect_url: '/room/123',
    'navToActivePath@alice:example.org': '{"home":{"pathname":"/home"}}',
    settings: JSON.stringify({ themeId: 'aurora', pageZoom: 120 }),
  });
  let reloaded = false;

  await performLogout(undefined, {
    ...createLogoutDeps().deps,
    clearSessionLocalStorage,
    reload: () => {
      reloaded = true;
    },
    storage,
  });

  SESSION_LOCAL_STORAGE_EXACT_KEYS.forEach((key) => {
    assert.equal(storage.getItem(key), null, `expected session key ${key} to be removed`);
  });
  assert.equal(storage.getItem('navToActivePath@alice:example.org'), null);
  assert.equal(storage.getItem('settings'), JSON.stringify({ themeId: 'aurora', pageZoom: 120 }));
  assert.equal(reloaded, true);
});

for (const withClient of [false, true]) {
  test(`native logout failure preserves renderer state and allows retry (client=${withClient})`, async () => {
    const failure = new Error('native local credential deletion failed');
    const { deps, clearPersistedCalls, getReloaded } = createLogoutDeps();
    let attempts = 0;
    const logout = async () => {
      attempts += 1;
      if (attempts === 1) throw failure;
    };
    const storage = createEnumeratedMemoryStorage({ after_login_redirect_url: '/home' });
    const client = withClient ? ({ logout } as any) : undefined;
    const options = { ...deps, storage, clearSessionLocalStorage, logoutNativeSession: logout };
    notifiedEventIdsCache.add('$retry-retained');
    await assert.rejects(performLogout(client, options), failure);
    assert.equal(getReloaded(), false);
    assert.equal(clearPersistedCalls.length, 0);
    assert.equal(storage.getItem('after_login_redirect_url'), '/home');
    assert.equal(notifiedEventIdsCache.has('$retry-retained'), true);
    await performLogout(client, options);
    assert.equal(attempts, 2);
    assert.equal(getReloaded(), true);
    assert.equal(storage.getItem('after_login_redirect_url'), null);
    assert.equal(notifiedEventIdsCache.size, 0);
  });
}

test('Reload Application clears renderer caches without logging out or invoking native deletion', async () => {
  const originalWindow = globalThis.window;
  const originalLocalStorage = globalThis.localStorage;
  const storage = createEnumeratedMemoryStorage({ 'navToActivePath@alice:example.org': '/home' });
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
  let reloaded = false;
  let stopped = false;
  notifiedEventIdsCache.add('$renderer-cache');
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      location: {
        reload: () => {
          reloaded = true;
        },
      },
      __SYNARA_DESKTOP__: {
        invoke: async () => {
          assert.fail('renderer refresh must not invoke native deletion');
        },
      },
    },
  });
  try {
    await reloadApplication({
      stopClient: async () => {
        stopped = true;
      },
      getSafeUserId: () => '@alice:example.org',
      logout: async () => {
        assert.fail('renderer refresh must not log out');
      },
    } as any);
    assert.equal(stopped, true);
    assert.equal(storage.getItem('navToActivePath@alice:example.org'), null);
    assert.equal(reloaded, true);
    assert.equal(notifiedEventIdsCache.size, 0);
  } finally {
    Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: originalLocalStorage,
    });
  }
});
