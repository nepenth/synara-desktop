import assert from 'node:assert/strict';
import test from 'node:test';
import {
  nativeVerificationErrorMessage,
  NativeVerificationRequest,
  verificationRequestHasSasCodes,
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

test('SAS compare requires emoji or decimal codes before confirm', () => {
  assert.equal(verificationRequestHasSasCodes(request('outgoing', 'sas_ready')), false);
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('outgoing', 'sas_ready'),
      sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
    }),
    true
  );
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('incoming', 'sas_ready'),
      sas: { decimals: [11, 22, 33] },
    }),
    true
  );
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('outgoing', 'sas_ready'),
      sas: { decimals: [11, 22] as unknown as [number, number, number] },
    }),
    false
  );
});

test('native verification failures use a fixed privacy-safe message', () => {
  const message = nativeVerificationErrorMessage().toLowerCase();
  for (const forbidden of ['token', 'key', 'mac', 'secret', 'ciphertext', 'recovery']) {
    assert.equal(message.includes(forbidden), false);
  }
});
