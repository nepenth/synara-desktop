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

const nativeSession = {
  deviceId: 'NATIVE',
  userId: '@native:example.org',
  baseUrl: 'https://native.example.org',
};

test('session bootstrap accepts identity only from the native owner', async () => {
  const nativeStore: AsyncSessionStore = { getSession: async () => nativeSession };
  assert.deepEqual(await resolveSessionBootstrap({ nativeSessionStore: nativeStore }), {
    session: nativeSession,
    source: 'native',
  });
});

test('session bootstrap fails closed when native identity is absent', async () => {
  const nativeStore: AsyncSessionStore = { getSession: async () => undefined };
  assert.deepEqual(await resolveSessionBootstrap({ nativeSessionStore: nativeStore }), {
    source: 'none',
    nativeStoreError: undefined,
  });
});

test('session bootstrap fails closed and records native store errors', async () => {
  const nativeStore: AsyncSessionStore = {
    getSession: async () => {
      throw new Error('backend unavailable');
    },
  };
  assert.deepEqual(await resolveSessionBootstrap({ nativeSessionStore: nativeStore }), {
    source: 'none',
    nativeStoreError: NATIVE_SESSION_STORE_ERROR,
  });
});

test('native store warning explains fail-closed behavior without mentioning tokens', () => {
  assert.equal(shouldSurfaceNativeStoreErrorWarning(NATIVE_SESSION_STORE_ERROR, true), true);
  assert.equal(shouldSurfaceNativeStoreErrorWarning(NATIVE_SESSION_STORE_ERROR, false), false);
  const message = getNativeStoreErrorWarningMessage();
  assert.match(message, /did not restore a session/i);
  assert.doesNotMatch(message, /token/i);
});

test('initializeSessionBootstrap caches native identity for synchronous consumers', async () => {
  resetSessionBootstrapForTests();
  try {
    await initializeSessionBootstrap({
      nativeSessionStore: { getSession: async () => nativeSession },
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
