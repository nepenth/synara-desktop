import test from 'node:test';
import assert from 'node:assert/strict';
// Mock client type: local structural projection (js-sdk MatrixClient type no longer imported).
import { performLogout } from '../../../client/initMatrix';
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
  const clearPersistedCalls: Array<Record<string, unknown>> = [];
  let nativeLogoutCalls = 0;
  let reloaded = false;

  const deps = {
    clearPersistedSessions: async (options?: Record<string, unknown>) => {
      clearPersistedCalls.push(options ?? {});
    },
    clearSessionLocalStorage: () => undefined,
    logoutNativeSession: async () => {
      nativeLogoutCalls += 1;
    },
    nativeSessionStore: {},
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

test('performLogout with matrix client stops client, clears stores, and reloads', async () => {
  const { mx, calls } = createMockMatrixClient();
  const { deps, clearPersistedCalls, getReloaded } = createLogoutDeps();

  await performLogout(mx, {
    ...deps,
    clearPersistedSessions: async (options) => {
      calls.push('clearPersistedSessions');
      await deps.clearPersistedSessions(options);
    },
  });

  assert.deepEqual(calls, ['stopClient', 'logout', 'clearPersistedSessions']);
  assert.equal(clearPersistedCalls.length, 1);
  assert.deepEqual(clearPersistedCalls[0], { nativeSessionStore: deps.nativeSessionStore });
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
