import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearPersistedSessions,
  migrateLegacySessionToNativeAfterClientInit,
  persistAuthenticatedSession,
} from '../sessionPersistence';
import {
  getActiveSession,
  getSessionBootstrapResult,
  resetSessionBootstrapForTests,
  setSessionBootstrapResult,
  type AsyncSessionStore,
} from '../sessionBootstrap';
import { createLocalStorageSessionStore, type Session, type SessionStorage } from '../sessions';

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
    assert.deepEqual(nativeStore.stored, [
      {
        accessToken: 'access-token',
        baseUrl: 'https://matrix.example.org',
        deviceId: 'DEVICEID',
        userId: '@alice:example.org',
      },
    ]);
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
    assert.deepEqual(nativeStore.stored, [
      {
        accessToken: 'access-token',
        baseUrl: 'https://matrix.example.org',
        deviceId: 'DEVICEID',
        userId: '@alice:example.org',
      },
    ]);
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
