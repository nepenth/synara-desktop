import assert from 'node:assert/strict';
import test from 'node:test';
import {
  dismissDeviceVerificationPrompt,
  IDENTITY_RESET_COPY,
  IDENTITY_RESET_FAILED,
  identityResetApprovalUrl,
  identityResetErrorMessage,
  identityResetMayLeaveRecoveryOff,
  shouldShowDeviceVerificationPrompt,
  startIdentityReset,
} from '../nativeIdentityReset';

const withInvoke = async (
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>,
  run: () => Promise<void>
) => {
  const previousWindow = globalThis.window;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: { __TAURI_INTERNALS__: { invoke }, dispatchEvent: () => true },
  });
  try {
    await run();
  } finally {
    if (previousWindow === undefined) Reflect.deleteProperty(globalThis, 'window');
    else Object.defineProperty(globalThis, 'window', { configurable: true, value: previousWindow });
  }
};

test('typed reset failures map to fixed copy and never echo native text', () => {
  assert.equal(
    identityResetErrorMessage({ diagnosticId: 'v-crypto.2-identity-reset-password-rejected' }),
    'Identity reset authentication failed. Check your password and try again.'
  );
  assert.match(
    identityResetErrorMessage({ diagnosticId: 'v-crypto.2-identity-reset-approval-pending' }),
    /Approve the reset in your browser/
  );
  assert.match(
    identityResetErrorMessage({ diagnosticId: 'v-crypto.2-identity-reset-network' }),
    /Check your connection/
  );
  assert.equal(
    identityResetErrorMessage({ diagnosticId: 'v-crypto.2-identity-reset-failed', message: 'x' }),
    IDENTITY_RESET_FAILED
  );
  assert.equal(identityResetErrorMessage(new Error('https://hs/secret')), IDENTITY_RESET_FAILED);
});

test('the warning names every consequence before anything is reset', () => {
  const text = IDENTITY_RESET_COPY.warnings.join(' ');
  assert.match(text, /identity changed/);
  assert.match(text, /verify .* again/);
  assert.match(text, /may be lost, unless another device still has them/);
  assert.match(text, /removes your key backup and recovery key right away, even if you cancel/);
});

test('only failures that may have removed recovery point back to recovery setup', () => {
  const copy = (diagnosticId: string) => identityResetErrorMessage({ diagnosticId });
  assert.equal(
    identityResetMayLeaveRecoveryOff(copy('v-crypto.2-identity-reset-password-rejected')),
    false
  );
  assert.equal(
    identityResetMayLeaveRecoveryOff(copy('v-crypto.2-identity-reset-approval-pending')),
    false
  );
  assert.equal(
    identityResetMayLeaveRecoveryOff(copy('v-crypto.2-identity-reset-auth-unsupported')),
    true
  );
  assert.equal(identityResetMayLeaveRecoveryOff(IDENTITY_RESET_FAILED), true);
  assert.match(
    IDENTITY_RESET_COPY.cancelled,
    /Set up recovery in Settings once this device is verified/
  );
  assert.match(
    IDENTITY_RESET_COPY.recoveryMayBeOff,
    /Set up recovery in Settings once this device is verified/
  );
});

test('only an http(s) account page is opened for approval', () => {
  const state = (approvalUrl: string | null) => ({
    sessionGeneration: 2,
    step: 'approval_required' as const,
    approvalUrl,
  });
  assert.equal(
    identityResetApprovalUrl(state('https://account.example.org/reset')),
    'https://account.example.org/reset'
  );
  assert.equal(identityResetApprovalUrl(state('javascript:alert(1)')), undefined);
  assert.equal(identityResetApprovalUrl(state(null)), undefined);
  assert.equal(
    identityResetApprovalUrl({ sessionGeneration: 2, step: 'complete', approvalUrl: null }),
    undefined
  );
});

test('the prompt shows only when Core says so and Later dismisses it in Core', async () => {
  const calls: string[] = [];
  let show = true;
  await withInvoke(
    async (command) => {
      calls.push(command);
      if (command === 'matrix_device_verification_prompt') return { sessionGeneration: 3, show };
      if (command === 'matrix_device_verification_prompt_dismiss') {
        show = false;
        return { sessionGeneration: 3, show: false };
      }
      throw new Error(`unexpected ${command}`);
    },
    async () => {
      assert.equal(await shouldShowDeviceVerificationPrompt(), true);
      await dismissDeviceVerificationPrompt();
      assert.equal(await shouldShowDeviceVerificationPrompt(), false);
    }
  );
  assert.deepEqual(calls, [
    'matrix_device_verification_prompt',
    'matrix_device_verification_prompt_dismiss',
    'matrix_device_verification_prompt',
  ]);

  await withInvoke(
    async () => {
      throw { code: 'Unknown', diagnosticId: 'v-crypto.2-verify-prompt-failed' };
    },
    async () => {
      assert.equal(await shouldShowDeviceVerificationPrompt(), false, 'a failure stays hidden');
    }
  );
});

test('start returns the next step without any secret', async () => {
  await withInvoke(
    async (command) => {
      assert.equal(command, 'matrix_identity_reset_start');
      return { sessionGeneration: 5, step: 'password_required', approvalUrl: null };
    },
    async () => {
      const state = await startIdentityReset();
      assert.equal(state.step, 'password_required');
      assert.deepEqual(Object.keys(state).sort(), ['approvalUrl', 'sessionGeneration', 'step']);
    }
  );
});
