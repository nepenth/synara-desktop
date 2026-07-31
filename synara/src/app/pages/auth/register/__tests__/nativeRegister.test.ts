import assert from 'node:assert/strict';
import test from 'node:test';
import {
  generateRegisterClientSecret,
  NativeRegisterError,
  probeRegisterFlows,
  submitRegister,
  uiaAuthDataFromChallenge,
  uiaAuthDataFromProbe,
  type NativeRegisterInvoke,
} from '../nativeRegister';

test('generateRegisterClientSecret returns hex-like secret without hyphens', () => {
  const secret = generateRegisterClientSecret();
  assert.equal(typeof secret, 'string');
  assert.ok(secret.length >= 16);
  assert.equal(secret.includes('-'), false);
});

test('probeRegisterFlows invokes matrix_register_flows', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeRegisterInvoke = async (command, args) => {
    calls.push({ command, args });
    return {
      available: true,
      value: {
        status: 'flow_required',
        session: 'sess',
        flows: [{ stages: ['m.login.dummy'] }],
        completed: [],
        params: {},
      },
    };
  };
  const probe = await probeRegisterFlows('https://hs.example.org', invoke);
  assert.equal(calls[0]?.command, 'matrix_register_flows');
  assert.equal(calls[0]?.args?.homeserverUrl, 'https://hs.example.org');
  assert.equal(probe.status, 'flow_required');
  if (probe.status === 'flow_required') {
    const auth = uiaAuthDataFromProbe(probe);
    assert.equal(auth.session, 'sess');
    assert.deepEqual(auth.flows?.[0]?.stages, ['m.login.dummy']);
  }
});

test('submitRegister maps complete identity without token fields', async () => {
  const invoke: NativeRegisterInvoke = async (command, args) => {
    assert.equal(command, 'matrix_register');
    assert.equal(args?.username, 'alice');
    assert.equal(args?.password, 'secret');
    assert.ok(args?.auth);
    return {
      available: true,
      value: {
        status: 'complete',
        identity: {
          userId: '@alice:example.org',
          deviceId: 'DEVICE',
          homeserverUrl: 'https://hs.example.org',
        },
      },
    };
  };
  const outcome = await submitRegister(
    'https://hs.example.org',
    'alice',
    'secret',
    { type: 'session_only', session: 's' },
    'Synara Test',
    invoke
  );
  assert.equal(outcome.status, 'complete');
  if (outcome.status === 'complete') {
    assert.equal(outcome.identity.userId, '@alice:example.org');
    assert.equal('accessToken' in outcome.identity, false);
  }
});

test('submitRegister maps uia_required challenge', async () => {
  const invoke: NativeRegisterInvoke = async () => ({
    available: true,
    value: {
      status: 'uia_required',
      session: 's2',
      flows: [{ stages: ['m.login.recaptcha', 'm.login.terms'] }],
      completed: ['m.login.terms'],
      params: { 'm.login.recaptcha': { public_key: 'pk' } },
      errorCode: null,
      errorMessage: null,
    },
  });
  const outcome = await submitRegister(
    'https://hs.example.org',
    'bob',
    'pw',
    { type: 'terms', session: 's1' },
    undefined,
    invoke
  );
  assert.equal(outcome.status, 'uia_required');
  if (outcome.status === 'uia_required') {
    const auth = uiaAuthDataFromChallenge(outcome);
    assert.equal(auth.session, 's2');
    assert.deepEqual(auth.completed, ['m.login.terms']);
    assert.equal(auth.params?.['m.login.recaptcha']?.public_key, 'pk');
  }
});

test('submitRegister maps structured native errors without secret leakage', async () => {
  const invoke: NativeRegisterInvoke = async () => {
    throw { code: 'UserTaken', message: 'This username is already taken.', diagnosticId: 'x' };
  };
  await assert.rejects(
    () =>
      submitRegister(
        'https://hs.example.org',
        'alice',
        'password-should-not-appear',
        { type: 'session_only' },
        undefined,
        invoke
      ),
    (err: unknown) => {
      assert.ok(err instanceof NativeRegisterError);
      assert.equal(err.code, 'UserTaken');
      assert.equal(err.message.includes('password-should-not-appear'), false);
      return true;
    }
  );
});

test('unavailable native register fails closed', async () => {
  const invoke: NativeRegisterInvoke = async () => ({ available: false });
  await assert.rejects(
    () => probeRegisterFlows('https://hs.example.org', invoke),
    (err: unknown) => {
      assert.ok(err instanceof NativeRegisterError);
      assert.match(err.message, /unavailable/i);
      return true;
    }
  );
});
