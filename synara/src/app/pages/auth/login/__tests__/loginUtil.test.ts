import test from 'node:test';
import assert from 'node:assert/strict';
import {
  completeAuthenticatedLogin,
  type CompleteAuthenticatedLoginDeps,
  type CustomLoginResponse,
} from '../loginUtil';

const loginData: CustomLoginResponse = {
  baseUrl: 'https://matrix.example.org',
  response: {
    access_token: 'access-token',
    device_id: 'DEVICEID',
    user_id: '@alice:example.org',
  },
};

test('completeAuthenticatedLogin pushes session to the service worker once after persist', async () => {
  const persistCalls: Array<Record<string, unknown>> = [];
  const pushCalls: Array<[string | undefined, string | undefined]> = [];

  await completeAuthenticatedLogin(loginData, {
    persistAuthenticatedSession: async (session, options) => {
      persistCalls.push({ session, options });
      return {
        session,
        source: 'native',
      };
    },
    pushSessionToSW: (baseUrl, accessToken) => {
      pushCalls.push([baseUrl, accessToken]);
    },
    nativeSessionStore: {} as CompleteAuthenticatedLoginDeps['nativeSessionStore'],
  });

  assert.equal(persistCalls.length, 1);
  assert.deepEqual(persistCalls[0]?.session, {
    accessToken: 'access-token',
    deviceId: 'DEVICEID',
    userId: '@alice:example.org',
    baseUrl: 'https://matrix.example.org',
  });
  assert.equal(pushCalls.length, 1);
  assert.deepEqual(pushCalls[0], ['https://matrix.example.org', 'access-token']);
});