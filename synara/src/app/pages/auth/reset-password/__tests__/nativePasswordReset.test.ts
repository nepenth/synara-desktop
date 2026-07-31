import assert from 'node:assert/strict';
import test from 'node:test';
import {
  completePasswordReset,
  generatePasswordResetClientSecret,
  requestPasswordResetEmailToken,
  type NativePasswordResetInvoke,
} from '../nativePasswordReset';

test('generatePasswordResetClientSecret produces Matrix-safe secret shape', () => {
  const secret = generatePasswordResetClientSecret();
  assert.ok(secret.length >= 16);
  assert.match(secret, /^[0-9a-zA-Z._=-]+$/);
  assert.equal(secret.includes('-'), false);
});

test('requestPasswordResetEmailToken invokes native command with privacy-safe args shape', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativePasswordResetInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_password_reset_request_email_token') {
      return { available: true, value: { sid: 'sid123', submitUrl: null } };
    }
    return { available: false };
  };

  const result = await requestPasswordResetEmailToken(
    'https://matrix.example.org',
    'user@example.org',
    'clientsecret001',
    1,
    invoke
  );

  assert.deepEqual(result, { sid: 'sid123', submitUrl: null });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].command, 'matrix_password_reset_request_email_token');
  assert.deepEqual(calls[0].args, {
    homeserverUrl: 'https://matrix.example.org',
    email: 'user@example.org',
    clientSecret: 'clientsecret001',
    sendAttempt: 1,
  });
});

test('completePasswordReset invokes native complete command', async () => {
  const invoke: NativePasswordResetInvoke = async (command, args) => {
    if (command === 'matrix_password_reset_complete') {
      assert.equal(args?.homeserverUrl, 'https://matrix.example.org');
      assert.equal(args?.email, 'user@example.org');
      assert.equal(args?.newPassword, 'new-secret');
      assert.equal(args?.clientSecret, 'clientsecret001');
      assert.equal(args?.sid, 'sid123');
      return { available: true, value: { status: 'complete' } };
    }
    return { available: false };
  };

  const result = await completePasswordReset(
    'https://matrix.example.org',
    'user@example.org',
    'new-secret',
    'clientsecret001',
    'sid123',
    invoke
  );
  assert.deepEqual(result, { status: 'complete' });
});

test('completePasswordReset surfaces email_not_verified without throwing', async () => {
  const invoke: NativePasswordResetInvoke = async () => ({
    available: true,
    value: {
      status: 'email_not_verified',
      session: 'uia-sess',
      errorCode: 'M_UNAUTHORIZED',
      errorMessage: 'Email has not been verified yet, or the verification expired.',
    },
  });

  const result = await completePasswordReset(
    'https://matrix.example.org',
    'user@example.org',
    'new-secret',
    'clientsecret001',
    'sid123',
    invoke
  );
  assert.equal(result.status, 'email_not_verified');
  if (result.status === 'email_not_verified') {
    assert.equal(result.errorCode, 'M_UNAUTHORIZED');
  }
});

test('native password reset fails closed when desktop IPC unavailable', async () => {
  const invoke: NativePasswordResetInvoke = async () => ({ available: false });
  await assert.rejects(
    () =>
      requestPasswordResetEmailToken(
        'https://matrix.example.org',
        'user@example.org',
        'clientsecret001',
        1,
        invoke
      ),
    /unavailable/
  );
});
