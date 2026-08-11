import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { clearDesktopDiagnostics } from '../../../../utils/desktopDiagnostics';
import {
  LoginError,
  PasswordLoginError,
  loginPassword,
  type PasswordLoginInvoke,
} from '../loginUtil';

// Work from the package root (synara/) the same way modernization tests do.
const packageRoot = process.cwd();
const loginDir = path.join(packageRoot, 'src/app/pages/auth/login');
const repoRoot = path.resolve(packageRoot, '..');

test('desktop password login: no js createClient / loginRequest / matrix-js-sdk in owners', () => {
  const loginUtil = readFileSync(path.join(loginDir, 'loginUtil.ts'), 'utf8');
  assert.match(loginUtil, /matrix_login_password/);
  assert.match(loginUtil, /isSynaraDesktop|isDesktop/);
  assert.doesNotMatch(loginUtil, /from ['"]matrix-js-sdk['"]/);
  // Live construction/call sites (comments alone are not product fallbacks).
  assert.doesNotMatch(loginUtil, /\bcreateClient\s*\(/);
  assert.doesNotMatch(loginUtil, /\bloginRequest\s*\(/);
  assert.doesNotMatch(loginUtil, /import\s*\{[^}]*\bcreateClient\b/);
  // Non-desktop must fail closed (no js fallback call).
  assert.match(loginUtil, /Password login requires the native desktop Matrix runtime/);
  assert.doesNotMatch(loginUtil, /return login\(/);

  const form = readFileSync(path.join(loginDir, 'PasswordLoginForm.tsx'), 'utf8');
  assert.doesNotMatch(form, /from ['"]matrix-js-sdk['"]/);
  assert.doesNotMatch(form, /\bcreateClient\b/);
  assert.match(form, /loginPassword/);

  const libRs = readFileSync(path.join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
  assert.match(libRs, /matrix_login_password/);
});

test('loginPassword invokes matrix_login_password on desktop and returns native identity', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: PasswordLoginInvoke = async (command, args) => {
    calls.push({ command, args });
    return {
      available: true,
      value: {
        userId: '@alice:example.org',
        deviceId: 'DEVICEID',
        homeserverUrl: 'https://matrix.example.org',
      },
    };
  };

  const result = await loginPassword(
    'https://matrix.example.org',
    {
      type: 'm.login.password',
      identifier: { type: 'm.id.user', user: '@alice:example.org' },
      password: 's3cret',
    },
    { isDesktop: () => true, invoke }
  );

  assert.equal(calls.length, 1);
  assert.equal(calls[0]?.command, 'matrix_login_password');
  assert.equal(calls[0]?.args?.homeserverUrl, 'https://matrix.example.org');
  assert.equal(calls[0]?.args?.user, '@alice:example.org');
  assert.equal(calls[0]?.args?.password, 's3cret');
  assert.equal(result.native, true);
  assert.equal(result.identity.userId, '@alice:example.org');
  assert.equal(result.identity.deviceId, 'DEVICEID');
});

test('loginPassword fails closed when not desktop (no js client construction)', async () => {
  let invokeCalled = false;
  const invoke: PasswordLoginInvoke = async () => {
    invokeCalled = true;
    return { available: true, value: {} };
  };

  await assert.rejects(
    () =>
      loginPassword(
        'https://matrix.example.org',
        {
          type: 'm.login.password',
          identifier: { type: 'm.id.user', user: '@alice:example.org' },
          password: 's3cret',
        },
        { isDesktop: () => false, invoke }
      ),
    (err: unknown) => {
      assert.ok(err instanceof PasswordLoginError);
      assert.equal(err.errcode, LoginError.Unknown);
      assert.match(err.message, /native desktop Matrix runtime/i);
      return true;
    }
  );
  assert.equal(invokeCalled, false);
});

test('loginPassword fails closed when native command unavailable', async () => {
  const invoke: PasswordLoginInvoke = async () => ({ available: false });

  await assert.rejects(
    () =>
      loginPassword(
        'https://matrix.example.org',
        {
          type: 'm.login.password',
          identifier: { type: 'm.id.user', user: '@alice:example.org' },
          password: 's3cret',
        },
        { isDesktop: () => true, invoke }
      ),
    (err: unknown) => {
      assert.ok(err instanceof PasswordLoginError);
      assert.equal(err.errcode, LoginError.Unknown);
      assert.match(err.message, /unavailable/i);
      return true;
    }
  );
});

test('loginPassword maps native Forbidden code', async () => {
  const invoke: PasswordLoginInvoke = async () => {
    throw { code: 'Forbidden', message: 'rejected' };
  };

  await assert.rejects(
    () =>
      loginPassword(
        'https://matrix.example.org',
        {
          type: 'm.login.password',
          identifier: { type: 'm.id.user', user: '@alice:example.org' },
          password: 'bad',
        },
        { isDesktop: () => true, invoke }
      ),
    (err: unknown) => {
      assert.ok(err instanceof PasswordLoginError);
      assert.equal(err.errcode, LoginError.Forbidden);
      return true;
    }
  );
});

test('loginPassword default bridge logs only a mapped static diagnostic', async () => {
  const originalWindow = globalThis.window;
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  clearDesktopDiagnostics();

  try {
    (globalThis as any).window = {
      __SYNARA_DESKTOP__: {
        platform: 'tauri',
        invoke: async (command: string, args?: Record<string, unknown>) => {
          calls.push({ command, args });
          if (command === 'desktop_append_log') return undefined;
          throw {
            code: 'Unknown',
            diagnosticId: 'p3.2-login-http-api-response',
            message:
              'server body for @alice:example.org password=hunter2 https://private.example/token=secret',
          };
        },
      },
    };

    await assert.rejects(
      () =>
        loginPassword(
          'https://matrix.example.org',
          {
            type: 'm.login.password',
            identifier: { type: 'm.id.user', user: '@alice:example.org' },
            password: 's3cret',
          },
          { isDesktop: () => true }
        ),
      (err: unknown) => {
        assert.ok(err instanceof PasswordLoginError);
        assert.equal(err.errcode, LoginError.Unknown);
        assert.equal(err.diagnosticId, 'p3.2-login-http-api-response');
        return true;
      }
    );

    const logMessages = calls
      .filter((call) => call.command === 'desktop_append_log')
      .map((call) => call.args?.message)
      .filter((message): message is string => typeof message === 'string');
    assert.deepEqual(logMessages, ['matrix_login_password failed: p3.2-login-http-api-response']);
    assert.doesNotMatch(
      logMessages.join('\n'),
      /alice:example\.org|hunter2|private\.example|token=secret/
    );
  } finally {
    (globalThis as any).window = originalWindow;
    clearDesktopDiagnostics();
  }
});

test('loginPassword preserves only a static native diagnostic id', async () => {
  const invoke: PasswordLoginInvoke = async () => {
    throw {
      code: 'Unknown',
      diagnosticId: 'p3.2-login-http-api-response',
    };
  };

  await assert.rejects(
    () =>
      loginPassword(
        'https://matrix.example.org',
        {
          type: 'm.login.password',
          identifier: { type: 'm.id.user', user: '@alice:example.org' },
          password: 's3cret',
        },
        { isDesktop: () => true, invoke }
      ),
    (err: unknown) => {
      assert.ok(err instanceof PasswordLoginError);
      assert.equal(err.errcode, LoginError.Unknown);
      assert.equal(err.diagnosticId, 'p3.2-login-http-api-response');
      return true;
    }
  );
});

test('loginPassword drops unlisted and unsafe native diagnostic values', async () => {
  for (const diagnosticId of [
    'p3.2-login-unlisted-native-value',
    'https://private.example/token=secret',
  ]) {
    const invoke: PasswordLoginInvoke = async () => {
      throw { code: 'Unknown', diagnosticId };
    };

    await assert.rejects(
      () =>
        loginPassword(
          'https://matrix.example.org',
          {
            type: 'm.login.password',
            identifier: { type: 'm.id.user', user: '@alice:example.org' },
            password: 's3cret',
          },
          { isDesktop: () => true, invoke }
        ),
      (err: unknown) => {
        assert.ok(err instanceof PasswordLoginError);
        assert.equal(err.diagnosticId, undefined);
        return true;
      }
    );
  }
});

test('loginPassword preserves refined store/olm static diagnostic ids', async () => {
  for (const diagnosticId of [
    'p3.2-login-store-locked',
    'p3.2-login-store-open-failed',
    'p3.2-login-olm-unavailable',
    'p3.2-login-crypto-store', // legacy umbrella id remains accepted
  ]) {
    const invoke: PasswordLoginInvoke = async () => {
      throw { code: 'Unknown', diagnosticId };
    };

    await assert.rejects(
      () =>
        loginPassword(
          'https://matrix.example.org',
          {
            type: 'm.login.password',
            identifier: { type: 'm.id.user', user: '@alice:example.org' },
            password: 's3cret',
          },
          { isDesktop: () => true, invoke }
        ),
      (err: unknown) => {
        assert.ok(err instanceof PasswordLoginError);
        assert.equal(err.diagnosticId, diagnosticId);
        return true;
      }
    );
  }
});
