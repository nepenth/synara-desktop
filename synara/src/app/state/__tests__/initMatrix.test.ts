import test from 'node:test';
import assert from 'node:assert/strict';
// Native-boot contract for initClient (Option A + D1C). The renderer ceded
// token custody and crypto to native; no js-sdk stores/continuity remain.
import { initClient } from '../../../client/initMatrix';

const session = {
  baseUrl: 'https://matrix.example.org',
  accessToken: 'access-token',
  userId: '@alice:example.org',
  deviceId: 'ALICE_DEVICE',
  sessionGeneration: 'generation-1',
};

const createMockMatrixClient = (): any => ({ refresh: async () => undefined } as any);

test('initClient runs the native client bootstrap and records identity', async () => {
  const calls: string[] = [];

  await initClient(session, {
    isPendingFreshLoginIdentity: () => true,
    startMatrixClient: async () => {
      calls.push('start');
      return createMockMatrixClient();
    },
    setLastBootstrappedMatrixIdentity: () => {
      calls.push('record-identity');
    },
  });

  assert.deepEqual(calls, ['start', 'record-identity']);
});

test('initClient records bootstrapped identity after successful init', async () => {
  let recordedIdentity: { userId: string; deviceId: string } | undefined;

  await initClient(session, {
    isPendingFreshLoginIdentity: () => false,
    startMatrixClient: async () => createMockMatrixClient(),
    setLastBootstrappedMatrixIdentity: (identity) => {
      recordedIdentity = identity;
    },
  });

  assert.deepEqual(recordedIdentity, {
    userId: '@alice:example.org',
    deviceId: 'ALICE_DEVICE',
  });
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
