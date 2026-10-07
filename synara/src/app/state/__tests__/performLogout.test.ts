import test from 'node:test';
import assert from 'node:assert/strict';
// Mock client type: local structural projection (js-sdk MatrixClient type no longer imported).
import { readFileSync } from 'node:fs';
import {
  attemptLogout,
  LOGOUT_RETRY_COPY,
  reloadApplication,
  performLogout,
} from '../../../client/initMatrix';
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

for (const failingStep of ['bootstrap', 'storage']) {
  test(`confirmed logout completes other cleanup and reload when ${failingStep} cleanup throws`, async () => {
    const calls: string[] = [];
    const storage = createEnumeratedMemoryStorage({ after_login_redirect_url: '/old' });
    notifiedEventIdsCache.add('$old-account');
    await performLogout(undefined, {
      storage,
      logoutNativeSession: async () => {
        calls.push('native-logged-out');
      },
      clearPersistedSessions: async () => {
        calls.push('bootstrap');
        if (failingStep === 'bootstrap') throw new Error('renderer bootstrap cleanup failed');
      },
      clearSessionLocalStorage: (target) => {
        calls.push('storage');
        if (failingStep === 'storage') throw new Error('browser storage unavailable');
        clearSessionLocalStorage(target);
      },
      reload: () => {
        calls.push('reload');
      },
    });
    assert.deepEqual(calls, ['native-logged-out', 'bootstrap', 'storage', 'reload']);
    assert.equal(notifiedEventIdsCache.size, 0);
    if (failingStep === 'bootstrap')
      assert.equal(storage.getItem('after_login_redirect_url'), null);
  });
}

for (const storageFails of [false, true]) {
  test(`renderer recovery reloads after stop listener failure (storageFails=${storageFails})`, async () => {
    const originalWindow = globalThis.window;
    const originalStorage = globalThis.localStorage;
    const calls: string[] = [];
    notifiedEventIdsCache.add('$wedged-renderer');
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      value: {
        removeItem: () => {
          calls.push('navigation');
          if (storageFails) throw new Error('storage unavailable');
        },
      },
    });
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        location: {
          reload: () => {
            calls.push('reload');
          },
        },
      },
    });
    try {
      await reloadApplication({
        getSafeUserId: () => {
          calls.push('identity');
          return '@alice:example.org';
        },
        stopClient: async () => {
          calls.push('stop');
          throw new Error('renderer listener threw');
        },
      } as any);
      assert.deepEqual(calls, ['identity', 'stop', 'navigation', 'reload']);
      assert.equal(notifiedEventIdsCache.size, 0);
    } finally {
      Object.defineProperty(globalThis, 'window', { configurable: true, value: originalWindow });
      Object.defineProperty(globalThis, 'localStorage', {
        configurable: true,
        value: originalStorage,
      });
    }
  });
}

test('attemptLogout resolves retry and keeps renderer state when native logout rejects', async () => {
  const { deps, clearPersistedCalls, getReloaded } = createLogoutDeps();
  const outcome = await attemptLogout(undefined, {
    ...deps,
    logoutNativeSession: async () => {
      throw new Error('Native logout did not complete. Retry before signing out.');
    },
  });
  assert.equal(outcome, 'retry');
  assert.equal(clearPersistedCalls.length, 0);
  assert.equal(getReloaded(), false);
});

test('attemptLogout resolves retry when the facade reports an incomplete logout', async () => {
  const { deps, getReloaded } = createLogoutDeps();
  const mx = {
    logout: async () => {
      throw new Error('Native logout did not complete. Retry before signing out.');
    },
  } as unknown as any;
  assert.equal(await attemptLogout(mx, deps), 'retry');
  assert.equal(getReloaded(), false);
});

test('attemptLogout resolves logged_out after native and renderer cleanup', async () => {
  const { deps, getNativeLogoutCalls, getReloaded } = createLogoutDeps();
  assert.equal(await attemptLogout(undefined, deps), 'logged_out');
  assert.equal(getNativeLogoutCalls(), 1);
  assert.equal(getReloaded(), true);
});

test('LogoutDialog renders the fixed retry copy from the logout outcome', () => {
  const dialog = readFileSync('src/app/components/LogoutDialog.tsx', 'utf8');
  assert.match(dialog, /attemptLogout\(mx\)/);
  assert.match(dialog, /logoutState\.data === 'retry'/);
  assert.match(dialog, /\{LOGOUT_RETRY_COPY\}/);
  assert.doesNotMatch(dialog, /\.catch\(\(\) => undefined\)/);
  assert.doesNotMatch(dialog, /\(\) => \{\s*\/\/[^\n]*\n[^}]*\}\s*\)/);
  assert.equal(
    LOGOUT_RETRY_COPY,
    'Local sign out did not complete. Retry to finish local cleanup.'
  );
});
