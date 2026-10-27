import assert from 'node:assert/strict';
import test from 'node:test';
import {
  nativeChangePassword,
  passwordFormProblem,
  PasswordChangeUnsupportedError,
} from '../nativePassword';

const input = { currentPassword: 'old secret', newPassword: 'new secret', logoutDevices: true };

test('password change sends both passwords as command arguments', async () => {
  const calls: unknown[] = [];
  await nativeChangePassword(input, async (command, args, options) => {
    calls.push({ command, args, options });
    return { available: true, value: null as never };
  });
  assert.deepEqual(calls, [
    {
      command: 'matrix_change_own_password',
      args: { currentPassword: 'old secret', newPassword: 'new secret', logoutDevices: true },
      options: { suppressErrorDiagnostic: true },
    },
  ]);
});

test('native rejections become fixed messages, never the raw error', async () => {
  const rejecting =
    (diagnosticId: string, message = 'raw server body old secret') =>
    async () => {
      throw { diagnosticId, message };
    };
  await assert.rejects(
    nativeChangePassword(input, rejecting('v-password.current-wrong')),
    /^Error: The current password is incorrect\.$/
  );
  await assert.rejects(nativeChangePassword(input, rejecting('v-password.weak')), /too weak/);
  await assert.rejects(nativeChangePassword(input, rejecting('anything-else')), (error: Error) => {
    assert.equal(error.message, 'The password could not be changed. Try again.');
    assert.ok(!error.message.includes('old secret'));
    return true;
  });
  await assert.rejects(
    nativeChangePassword(input, rejecting('v-password.auth-unsupported')),
    PasswordChangeUnsupportedError
  );
});

test('the web build without the desktop bridge cannot change the password', async () => {
  await assert.rejects(
    nativeChangePassword(input, async () => ({ available: false })),
    /desktop app/
  );
});

test('the form needs three fields, matching new passwords and a change', () => {
  assert.equal(passwordFormProblem('', 'a', 'a'), 'Fill in all three fields.');
  assert.equal(passwordFormProblem('old', 'new', 'nwe'), 'The new passwords do not match.');
  assert.equal(
    passwordFormProblem('same', 'same', 'same'),
    'The new password is the same as the current one.'
  );
  assert.equal(passwordFormProblem('old', 'new', 'new'), undefined);
});
