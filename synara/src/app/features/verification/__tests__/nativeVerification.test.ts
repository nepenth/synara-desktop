import assert from 'node:assert/strict';
import test from 'node:test';
import {
  nativeVerificationErrorMessage,
  NativeVerificationRequest,
  verificationRequestNeedsSasStart,
} from '../nativeVerification';

const request = (
  direction: NativeVerificationRequest['direction'],
  phase: NativeVerificationRequest['phase']
): NativeVerificationRequest => ({
  flowId: 'flow',
  otherUserId: '@alice:example.org',
  direction,
  phase,
});

test('SAS start projection follows Matrix request ownership', () => {
  assert.equal(verificationRequestNeedsSasStart(request('outgoing', 'ready')), true);
  assert.equal(verificationRequestNeedsSasStart(request('incoming', 'started')), true);
  assert.equal(verificationRequestNeedsSasStart(request('incoming', 'ready')), false);
  assert.equal(verificationRequestNeedsSasStart(request('outgoing', 'requested')), false);
});

test('native verification failures use a fixed privacy-safe message', () => {
  const message = nativeVerificationErrorMessage().toLowerCase();
  for (const forbidden of ['token', 'key', 'mac', 'secret', 'ciphertext', 'recovery']) {
    assert.equal(message.includes(forbidden), false);
  }
});
