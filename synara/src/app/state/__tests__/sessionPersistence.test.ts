import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearMatrixStoresForIdentityChange,
  clearPersistedSessions,
  getLastBootstrappedMatrixIdentity,
  getLastPersistedMatrixIdentity,
  isPersistedSessionExpired,
  LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY,
  LAST_PERSISTED_MATRIX_IDENTITY_KEY,
  matrixSessionIdentitiesMatch,
  migrateLegacySessionToNativeAfterClientInit,
  persistAuthenticatedSession,
  reconcileExpiredPersistedSession,
  SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS,
  setLastBootstrappedMatrixIdentity,
  shouldClearMatrixStoresBeforeInit,
} from '../sessionPersistence';
import {
  getActiveSession,
  getSessionBootstrapResult,
  resetSessionBootstrapForTests,
  setSessionBootstrapResult,
  type AsyncSessionStore,
} from '../sessionBootstrap';
import {
  clearSessionLocalStorage,
  createLocalStorageSessionStore,
  type Session,
  type SessionLocalStorage,
  type SessionStorage,
} from '../sessions';

const session: Session = {
  accessToken: 'access-token',
  baseUrl: 'https://matrix.example.org',
  deviceId: 'DEVICEID',
  userId: '@alice:example.org',
  fallbackSdkStores: true,
};

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

const createNativeSessionStore = ({
  setResult = true,
  throwOnSet = false,
}: {
  setResult?: boolean;
  throwOnSet?: boolean;
} = {}) => {
  const stored: Session[] = [];
  let removed = false;

  const store: Pick<AsyncSessionStore, 'setSession' | 'removeSession'> = {
    setSession: async (nextSession) => {
      if (throwOnSet) throw new Error('native unavailable');
      stored.push(nextSession);
      return setResult;
    },
    removeSession: async () => {
      removed = true;
      return true;
    },
  };

  return {
    store,
    get stored() {
      return stored;
    },
    get removed() {
      return removed;
    },
  };
};

test('isPersistedSessionExpired honors tolerance and missing metadata', () => {
  const storedAtMs = 1_000_000;
  const expiresInMs = 3_600_000;

  assert.equal(
    isPersistedSessionExpired(
      {
        ...session,
        storedAtMs,
        expiresInMs,
      },
      storedAtMs + expiresInMs + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS
    ),
    false
  );
  assert.equal(
    isPersistedSessionExpired(
      {
        ...session,
        storedAtMs,
        expiresInMs,
      },
      storedAtMs + expiresInMs + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS + 1
    ),
    true
  );
  assert.equal(isPersistedSessionExpired(session, storedAtMs + 9_999_999), false);
});

test('reconcileExpiredPersistedSession clears expired bootstrap sessions', async () => {
  resetSessionBootstrapForTests();
  const storedAtMs = 1_000_000;
  setSessionBootstrapResult({
    session: {
      accessToken: 'access-token',
      baseUrl: 'https://matrix.example.org',
      deviceId: 'DEVICEID',
      userId: '@alice:example.org',
      storedAtMs,
      expiresInMs: 60_000,
    },
    source: 'native',
  });
  const nativeStore = createNativeSessionStore();

  try {
    const result = await reconcileExpiredPersistedSession({
      nativeSessionStore: nativeStore.store,
    });

    assert.equal(result.source, 'none');
    assert.equal(nativeStore.removed, true);
    assert.equal(getActiveSession(), undefined);
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('persistAuthenticatedSession writes native first and removes legacy fallback', async () => {
  resetSessionBootstrapForTests();
  const storage = createMemoryStorage();
  const fallbackStore = createLocalStorageSessionStore(storage);
  fallbackStore.setFallbackSession(session);
  const nativeStore = createNativeSessionStore();

  try {
    const result = await persistAuthenticatedSession(session, {
      nativeSessionStore: nativeStore.store,
      fallbackStore,
    });

    assert.equal(result.source, 'native');
    assert.equal(result.nativeStoreError, undefined);
    assert.equal(fallbackStore.getFallbackSession(), undefined);
    assert.equal(nativeStore.stored.length, 1);
    assert.equal(nativeStore.stored[0]?.accessToken, 'access-token');
    assert.equal(nativeStore.stored[0]?.baseUrl, 'https://matrix.example.org');
    assert.equal(nativeStore.stored[0]?.deviceId, 'DEVICEID');
    assert.equal(nativeStore.stored[0]?.userId, '@alice:example.org');
    assert.equal(typeof nativeStore.stored[0]?.storedAtMs, 'number');
    assert.deepEqual(getActiveSession(), nativeStore.stored[0]);
    assert.deepEqual(getSessionBootstrapResult(), {
      session: nativeStore.stored[0],
      source: 'native',
      nativeStoreError: undefined,
    });
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('persistAuthenticatedSession keeps fallback when native write fails', async () => {
  resetSessionBootstrapForTests();
  const fallbackStore = createLocalStorageSessionStore(createMemoryStorage());
  const nativeStore = createNativeSessionStore({ throwOnSet: true });

  try {
    const result = await persistAuthenticatedSession(session, {
      nativeSessionStore: nativeStore.store,
      fallbackStore,
    });

    assert.equal(result.source, 'legacy-fallback');
    assert.equal(result.nativeStoreError, 'native-session-store-error');
    assert.deepEqual(fallbackStore.getFallbackSession(), session);
    assert.deepEqual(getActiveSession(), session);
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('migrateLegacySessionToNativeAfterClientInit clears native store error after successful migration', async () => {
  resetSessionBootstrapForTests();
  const fallbackStore = createLocalStorageSessionStore(createMemoryStorage());
  fallbackStore.setFallbackSession(session);
  setSessionBootstrapResult({
    session,
    source: 'legacy-fallback',
    nativeStoreError: 'native-session-store-error',
  });
  const nativeStore = createNativeSessionStore();

  try {
    const result = await migrateLegacySessionToNativeAfterClientInit({
      nativeSessionStore: nativeStore.store,
      fallbackStore,
    });

    assert.equal(result.status, 'migrated');
    assert.equal(getSessionBootstrapResult().nativeStoreError, undefined);
    assert.equal(getSessionBootstrapResult().source, 'native');
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('migrateLegacySessionToNativeAfterClientInit removes fallback only after native write', async () => {
  resetSessionBootstrapForTests();
  const fallbackStore = createLocalStorageSessionStore(createMemoryStorage());
  fallbackStore.setFallbackSession(session);
  setSessionBootstrapResult({ session, source: 'legacy-fallback' });
  const nativeStore = createNativeSessionStore();

  try {
    const result = await migrateLegacySessionToNativeAfterClientInit({
      nativeSessionStore: nativeStore.store,
      fallbackStore,
    });

    assert.equal(result.status, 'migrated');
    assert.equal(fallbackStore.getFallbackSession(), undefined);
    assert.equal(nativeStore.stored.length, 1);
    assert.equal(nativeStore.stored[0]?.accessToken, 'access-token');
    assert.equal(nativeStore.stored[0]?.baseUrl, 'https://matrix.example.org');
    assert.equal(nativeStore.stored[0]?.deviceId, 'DEVICEID');
    assert.equal(nativeStore.stored[0]?.userId, '@alice:example.org');
    assert.equal(typeof nativeStore.stored[0]?.storedAtMs, 'number');
    assert.deepEqual(getSessionBootstrapResult(), {
      session: nativeStore.stored[0],
      source: 'native',
      nativeStoreError: undefined,
    });
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('migrateLegacySessionToNativeAfterClientInit keeps fallback when native is unavailable', async () => {
  resetSessionBootstrapForTests();
  const fallbackStore = createLocalStorageSessionStore(createMemoryStorage());
  fallbackStore.setFallbackSession(session);
  setSessionBootstrapResult({ session, source: 'legacy-fallback' });
  const nativeStore = createNativeSessionStore({ setResult: false });

  try {
    const result = await migrateLegacySessionToNativeAfterClientInit({
      nativeSessionStore: nativeStore.store,
      fallbackStore,
    });

    assert.equal(result.status, 'native-unavailable');
    assert.deepEqual(fallbackStore.getFallbackSession(), session);
    assert.deepEqual(getSessionBootstrapResult(), {
      session,
      source: 'legacy-fallback',
      nativeStoreError: undefined,
    });
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('clearPersistedSessions with clearSessionLocalStorage preserves user settings', async () => {
  resetSessionBootstrapForTests();
  const storage = createEnumeratedMemoryStorage({
    synara_access_token: 'access-token',
    synara_device_id: 'DEVICEID',
    synara_user_id: '@alice:example.org',
    synara_hs_base_url: 'https://matrix.example.org',
    after_login_redirect_url: '/home',
    settings: JSON.stringify({ themeId: 'aurora', pageZoom: 125 }),
    platformSettings: JSON.stringify({ desktopShortcutShow: 'CmdOrCtrl+1' }),
  });
  const fallbackStore = createLocalStorageSessionStore(storage);
  fallbackStore.setFallbackSession(session);
  setSessionBootstrapResult({ session, source: 'legacy-fallback' });
  const nativeStore = createNativeSessionStore();

  await clearPersistedSessions({
    nativeSessionStore: nativeStore.store,
    fallbackStore,
  });
  clearSessionLocalStorage(storage);

  assert.equal(nativeStore.removed, true);
  assert.equal(fallbackStore.getFallbackSession(), undefined);
  assert.equal(getActiveSession(), undefined);
  assert.equal(storage.getItem('settings'), JSON.stringify({ themeId: 'aurora', pageZoom: 125 }));
  assert.equal(
    storage.getItem('platformSettings'),
    JSON.stringify({ desktopShortcutShow: 'CmdOrCtrl+1' })
  );
});

test('matrixSessionIdentitiesMatch compares userId and deviceId', () => {
  const alice = { userId: '@alice:example.org', deviceId: 'ALICE_DEVICE' };
  const bob = { userId: '@bob:example.org', deviceId: 'BOB_DEVICE' };

  assert.equal(matrixSessionIdentitiesMatch(alice, alice), true);
  assert.equal(matrixSessionIdentitiesMatch(alice, { ...alice, deviceId: 'OTHER_DEVICE' }), false);
  assert.equal(matrixSessionIdentitiesMatch(alice, bob), false);
  assert.equal(matrixSessionIdentitiesMatch(undefined, alice), false);
});

test('shouldClearMatrixStoresBeforeInit clears only when bootstrapped identity differs', () => {
  const current = { userId: '@bob:example.org', deviceId: 'BOB_DEVICE' };
  const previous = { userId: '@alice:example.org', deviceId: 'ALICE_DEVICE' };

  assert.equal(shouldClearMatrixStoresBeforeInit(current, undefined), false);
  assert.equal(shouldClearMatrixStoresBeforeInit(current, previous), true);
  assert.equal(shouldClearMatrixStoresBeforeInit(current, current), false);
});

test('clearMatrixStoresForIdentityChange clears stores only on identity mismatch', async () => {
  const storage = createMemoryStorage({
    [LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY]: JSON.stringify({
      userId: '@alice:example.org',
      deviceId: 'ALICE_DEVICE',
    }),
  });
  let clearCalls = 0;

  const cleared = await clearMatrixStoresForIdentityChange(
    { userId: '@bob:example.org', deviceId: 'BOB_DEVICE' },
    {
      storage,
      clearStores: async () => {
        clearCalls += 1;
      },
    }
  );

  assert.equal(cleared, true);
  assert.equal(clearCalls, 1);

  clearCalls = 0;
  const skipped = await clearMatrixStoresForIdentityChange(
    { userId: '@alice:example.org', deviceId: 'ALICE_DEVICE' },
    {
      storage,
      clearStores: async () => {
        clearCalls += 1;
      },
    }
  );

  assert.equal(skipped, false);
  assert.equal(clearCalls, 0);
});

test('persistAuthenticatedSession records last persisted matrix identity metadata', async () => {
  resetSessionBootstrapForTests();
  const storage = createMemoryStorage();
  const fallbackStore = createLocalStorageSessionStore(storage);
  const nativeStore = createNativeSessionStore();

  try {
    await persistAuthenticatedSession(session, {
      nativeSessionStore: nativeStore.store,
      fallbackStore,
      storage,
    });

    assert.deepEqual(getLastPersistedMatrixIdentity(storage), {
      userId: '@alice:example.org',
      deviceId: 'DEVICEID',
    });
    assert.equal(getLastBootstrappedMatrixIdentity(storage), undefined);
  } finally {
    resetSessionBootstrapForTests();
  }
});

test('bootstrapped matrix identity metadata round-trips through storage', () => {
  const storage = createMemoryStorage();
  const identity = { userId: '@alice:example.org', deviceId: 'DEVICEID' };

  setLastBootstrappedMatrixIdentity(identity, storage);

  assert.deepEqual(getLastBootstrappedMatrixIdentity(storage), identity);
  assert.equal(storage.getItem(LAST_PERSISTED_MATRIX_IDENTITY_KEY), null);
});

test('clearPersistedSessions clears native and legacy session locations', async () => {
  resetSessionBootstrapForTests();
  const fallbackStore = createLocalStorageSessionStore(createMemoryStorage());
  fallbackStore.setFallbackSession(session);
  setSessionBootstrapResult({ session, source: 'legacy-fallback' });
  const nativeStore = createNativeSessionStore();

  await clearPersistedSessions({
    nativeSessionStore: nativeStore.store,
    fallbackStore,
  });

  assert.equal(nativeStore.removed, true);
  assert.equal(fallbackStore.getFallbackSession(), undefined);
  assert.equal(getActiveSession(), undefined);
  assert.deepEqual(getSessionBootstrapResult(), {
    session: undefined,
    source: 'none',
    nativeStoreError: undefined,
  });
});
