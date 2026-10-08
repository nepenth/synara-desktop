import test from 'node:test';
import assert from 'node:assert/strict';
// Native-boot contract for initClient (Option A + D1C). The renderer ceded
// token custody and crypto to native; no js-sdk stores/continuity remain.
import { initClient } from '../../../client/initMatrix';
import { createNativeSession } from '../../native/nativeSession';
import { markPendingFreshLoginIdentity } from '../sessionPersistence';
import { PENDING_FRESH_LOGIN_IDENTITY_KEY } from '../sessions';

const session = {
  baseUrl: 'https://matrix.example.org',
  accessToken: 'access-token',
  userId: '@alice:example.org',
  deviceId: 'ALICE_DEVICE',
  sessionGeneration: 'generation-1',
};

const createMockMatrixClient = (): any => ({ refresh: async () => undefined }) as any;

test('initClient boots through the native client owner', async () => {
  const client = createMockMatrixClient();
  let starts = 0;
  const result = await initClient(session, {
    isPendingFreshLoginIdentity: () => false,
    startMatrixClient: async () => {
      starts += 1;
      return client;
    },
  });
  assert.equal(result, client);
  assert.equal(starts, 1);
});

test('initClient rethrows unrelated native boot failures', async () => {
  const startupError = new Error('network timeout');

  await assert.rejects(
    () =>
      initClient(session, {
        isPendingFreshLoginIdentity: () => false,
        startMatrixClient: async () => {
          throw startupError;
        },
      }),
    startupError
  );
});

for (const change of [
  'none',
  'user',
  'device',
  'homeserver',
  'generation',
  'logged-out',
  'unavailable',
  'failed',
]) {
  test(`fresh marker consumption requires the initialized native identity and generation (${change})`, async () => {
    const originalStorage = globalThis.localStorage;
    const values = new Map<string, string>();
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      removeItem: (key: string) => values.delete(key),
    };
    Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage });
    const freshSession = { ...session, sessionGeneration: '7' };
    markPendingFreshLoginIdentity(freshSession, storage);
    const nativeSession = {
      status: 'logged_in',
      userId: change === 'user' ? '@other:example.org' : freshSession.userId,
      deviceId: change === 'device' ? 'OTHER' : freshSession.deviceId,
      homeserverUrl: change === 'homeserver' ? 'https://other.example.org' : freshSession.baseUrl,
      sessionGeneration: change === 'generation' ? 8 : 7,
    };
    const client = createNativeSession(async (command: string) => {
      if (change === 'failed') throw new Error('refresh failed');
      if (command !== 'matrix_session_snapshot' || change === 'unavailable')
        return { available: false };
      return {
        available: true,
        value: change === 'logged-out' ? { status: 'logged_out' } : nativeSession,
      };
    });
    const run = () =>
      initClient(freshSession, {
        startMatrixClient: async () => {
          await client.refresh();
          return client;
        },
      });
    try {
      if (change === 'failed') await assert.rejects(run, /refresh failed/);
      else await run();
      assert.equal(values.has(PENDING_FRESH_LOGIN_IDENTITY_KEY), change !== 'none');
    } finally {
      Object.defineProperty(globalThis, 'localStorage', {
        configurable: true,
        value: originalStorage,
      });
    }
  });
}
