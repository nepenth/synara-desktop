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
