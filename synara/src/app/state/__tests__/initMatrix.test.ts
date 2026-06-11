import test from 'node:test';
import assert from 'node:assert/strict';
import type { MatrixClient } from 'matrix-js-sdk';
import { initClient } from '../../../client/initMatrix';

const session = {
  baseUrl: 'https://matrix.example.org',
  accessToken: 'access-token',
  userId: '@alice:example.org',
  deviceId: 'ALICE_DEVICE',
};

const createMockMatrixClient = (): MatrixClient => ({} as MatrixClient);

test('initClient clears matrix stores before init when bootstrapped identity differs', async () => {
  const calls: string[] = [];

  await initClient(session, {
    clearMatrixStoresForIdentityChange: async () => {
      calls.push('clear-for-identity');
      return true;
    },
    startMatrixClient: async () => {
      calls.push('start');
      return createMockMatrixClient();
    },
    setLastBootstrappedMatrixIdentity: () => {
      calls.push('record-identity');
    },
  });

  assert.deepEqual(calls, ['clear-for-identity', 'start', 'record-identity']);
});

test('initClient keeps crypto mismatch recovery as fallback after proactive clear', async () => {
  const mismatchError = new Error(
    "the account in the store doesn't match the account in the constructor: expected @bob:example.org:OLD, got @alice:example.org:ALICE_DEVICE"
  );
  let startAttempts = 0;
  let clearCalls = 0;

  const client = await initClient(session, {
    clearMatrixStoresForIdentityChange: async () => false,
    clearMatrixLocalStores: async () => {
      clearCalls += 1;
    },
    startMatrixClient: async () => {
      startAttempts += 1;
      if (startAttempts === 1) {
        throw mismatchError;
      }
      return createMockMatrixClient();
    },
    setLastBootstrappedMatrixIdentity: () => undefined,
  });

  assert.ok(client);
  assert.equal(startAttempts, 2);
  assert.equal(clearCalls, 1);
});

test('initClient records bootstrapped identity after successful init', async () => {
  let recordedIdentity: { userId: string; deviceId: string } | undefined;

  await initClient(session, {
    clearMatrixStoresForIdentityChange: async () => false,
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

test('initClient rethrows unrelated startup failures', async () => {
  const startupError = new Error('network timeout');

  await assert.rejects(
    () =>
      initClient(session, {
        clearMatrixStoresForIdentityChange: async () => false,
        startMatrixClient: async () => {
          throw startupError;
        },
      }),
    startupError
  );
});
