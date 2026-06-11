import test from 'node:test';
import assert from 'node:assert/strict';
import {
  getActiveSession,
  getNativeStoreErrorWarningMessage,
  getSessionBootstrapResult,
  initializeSessionBootstrap,
  NATIVE_SESSION_STORE_ERROR,
  resetSessionBootstrapForTests,
  resolveSessionBootstrap,
  shouldSurfaceNativeStoreErrorWarning,
  type AsyncSessionStore,
} from '../sessionBootstrap';
import { type SessionStore } from '../sessions';

const nativeSession = {
  accessToken: 'native-token',
  deviceId: 'NATIVE',
  userId: '@native:example.org',
  baseUrl: 'https://native.example.org',
};

const fallbackSession = {
  accessToken: 'legacy-token',
  deviceId: 'LEGACY',
  userId: '@legacy:example.org',
  baseUrl: 'https://legacy.example.org',
  fallbackSdkStores: true,
};

const createFallbackStore = (
  session: ReturnType<SessionStore['getFallbackSession']>
): Pick<SessionStore, 'getFallbackSession'> => ({
  getFallbackSession: () => session,
});

test('session bootstrap prefers native sessions over legacy fallback storage', async () => {
  const nativeStore: AsyncSessionStore = {
    getSession: async () => nativeSession,
  };

  assert.deepEqual(
    await resolveSessionBootstrap({
      nativeSessionStore: nativeStore,
      fallbackStore: createFallbackStore(fallbackSession),
    }),
    {
      session: nativeSession,
      source: 'native',
    }
  );
});

test('session bootstrap reads legacy fallback when native storage is empty', async () => {
  const nativeStore: AsyncSessionStore = {
    getSession: async () => undefined,
  };

  assert.deepEqual(
    await resolveSessionBootstrap({
      nativeSessionStore: nativeStore,
      fallbackStore: createFallbackStore(fallbackSession),
    }),
    {
      session: fallbackSession,
      source: 'legacy-fallback',
      nativeStoreError: undefined,
    }
  );
});

test('session bootstrap keeps legacy fallback when native storage fails', async () => {
  const nativeStore: AsyncSessionStore = {
    getSession: async () => {
      throw new Error('backend unavailable');
    },
  };

  assert.deepEqual(
    await resolveSessionBootstrap({
      nativeSessionStore: nativeStore,
      fallbackStore: createFallbackStore(fallbackSession),
    }),
    {
      session: fallbackSession,
      source: 'legacy-fallback',
      nativeStoreError: 'native-session-store-error',
    }
  );
});

test('session bootstrap returns none when no storage source has a session', async () => {
  assert.deepEqual(
    await resolveSessionBootstrap({
      fallbackStore: createFallbackStore(undefined),
    }),
    {
      source: 'none',
      nativeStoreError: undefined,
    }
  );
});

test('shouldSurfaceNativeStoreErrorWarning is shown only on desktop when native store fails', () => {
  assert.equal(shouldSurfaceNativeStoreErrorWarning(NATIVE_SESSION_STORE_ERROR, true), true);
  assert.equal(shouldSurfaceNativeStoreErrorWarning(NATIVE_SESSION_STORE_ERROR, false), false);
  assert.equal(shouldSurfaceNativeStoreErrorWarning(undefined, true), false);
});

test('getNativeStoreErrorWarningMessage explains legacy fallback without exposing tokens', () => {
  const message = getNativeStoreErrorWarningMessage();

  assert.match(message, /legacy browser storage/i);
  assert.doesNotMatch(message, /token/i);
  assert.doesNotMatch(message, /access/i);
});

test('initializeSessionBootstrap caches resolved sessions for synchronous consumers', async () => {
  resetSessionBootstrapForTests();

  const nativeStore: AsyncSessionStore = {
    getSession: async () => nativeSession,
  };

  try {
    await initializeSessionBootstrap({
      nativeSessionStore: nativeStore,
      fallbackStore: createFallbackStore(undefined),
    });

    assert.deepEqual(getActiveSession(), nativeSession);
    assert.deepEqual(getSessionBootstrapResult(), {
      session: nativeSession,
      source: 'native',
      nativeStoreError: undefined,
    });
  } finally {
    resetSessionBootstrapForTests();
  }
});
