import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  LoginError,
  PasswordLoginError,
  completeNativeLoginBootstrap,
  runNativeSessionHandoff,
} from '../loginUtil';
import { getHomePath } from '../../../pathUtils';
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

test('a failed session handoff reports failure instead of spinning without navigation', async () => {
  const navigated: string[] = [];
  const entered = await runNativeSessionHandoff({
    completeBootstrap: () => completeNativeLoginBootstrap(storeFor(undefined)),
    takeRedirectPath: () => '/rooms/!a:example.org',
    navigate: (path) => navigated.push(path),
    isActive: () => true,
  });
  assert.equal(entered, false);
  assert.deepEqual(navigated, []);
});

test('a successful handoff enters the app at the saved redirect or Home', async () => {
  const navigated: string[] = [];
  const deps = {
    completeBootstrap: () => completeNativeLoginBootstrap(storeFor(nativeSession)),
    navigate: (path: string) => navigated.push(path),
    isActive: () => true,
  };
  assert.equal(await runNativeSessionHandoff({ ...deps, takeRedirectPath: () => '/inbox/' }), true);
  assert.equal(await runNativeSessionHandoff({ ...deps, takeRedirectPath: () => undefined }), true);
  assert.equal(navigated[0], '/inbox/');
  assert.equal(navigated[1], getHomePath());
});

test('login forms show handoff failures and expose password-manager hints', () => {
  const login = readFileSync('src/app/pages/auth/login/PasswordLoginForm.tsx', 'utf8');
  assert.match(login, /autoComplete="username"/);
  assert.match(login, /autoComplete="current-password"/);
  assert.match(login, /htmlFor=\{usernameInputId\}/);
  assert.match(login, /id=\{usernameInputId\}/);
  assert.match(login, /htmlFor=\{passwordInputId\}/);
  assert.match(login, /id=\{passwordInputId\}/);
  assert.match(login, /NATIVE_SESSION_HANDOFF_FAILED_COPY/);
  // The spinner closes when the handoff fails.
  assert.match(login, /loginState\.status === AsyncStatus\.Success && !handoffFailed/);

  const register = readFileSync('src/app/pages/auth/register/PasswordRegisterForm.tsx', 'utf8');
  assert.match(register, /autoComplete="username"/);
  assert.equal(register.match(/autoComplete="new-password"/g)?.length, 2);
  assert.match(register, /htmlFor=\{passwordInputId\}/);
});
