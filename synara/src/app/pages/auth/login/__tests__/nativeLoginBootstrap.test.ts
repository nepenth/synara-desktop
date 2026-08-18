import test from 'node:test';
import assert from 'node:assert/strict';
import { LoginError, PasswordLoginError, completeNativeLoginBootstrap } from '../loginUtil';
import {
  getSessionBootstrapResult,
  resetSessionBootstrapForTests,
  type AsyncSessionStore,
} from '../../../../state/sessionBootstrap';
import type { Session } from '../../../../state/sessions';

const nativeSession: Session = {
  deviceId: 'DEVICEID',
  userId: '@alice:example.org',
  baseUrl: 'https://matrix.example.org',
};

const storeFor = (session: Session | undefined): AsyncSessionStore => ({
  getSession: async () => session,
});

test.beforeEach(() => {
  resetSessionBootstrapForTests();
});

test('completeNativeLoginBootstrap rehydrates identity-only native bootstrap', async () => {
  await completeNativeLoginBootstrap(storeFor(nativeSession));

  const result = getSessionBootstrapResult();
  assert.equal(result.source, 'native');
  assert.equal(result.session?.userId, '@alice:example.org');
  assert.equal(result.session?.baseUrl, 'https://matrix.example.org');
  assert.equal(result.nativeStoreError, undefined);
});

test('completeNativeLoginBootstrap fails closed when desktop session identity is missing', async () => {
  await assert.rejects(
    () => completeNativeLoginBootstrap(storeFor(undefined)),
    (err: unknown) => {
      assert.ok(err instanceof PasswordLoginError);
      assert.equal(err.errcode, LoginError.Unknown);
      assert.match(err.message, /desktop session identity is missing/i);
      return true;
    }
  );

  // Fail-closed: no bootstrap session survives the failed handoff.
  assert.equal(getSessionBootstrapResult().source, 'none');
});
