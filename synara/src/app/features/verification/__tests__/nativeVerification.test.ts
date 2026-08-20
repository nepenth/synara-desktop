import assert from 'node:assert/strict';
import test from 'node:test';
import {
  isNativeVerificationTerminal,
  nativeVerificationErrorMessage,
  NativeVerificationRequest,
  selectNativeVerificationRequest,
  verificationRequestHasSasCodes,
  verificationRequestNeedsSasStart,
} from '../nativeVerification';

const request = (
  direction: NativeVerificationRequest['direction'],
  phase: NativeVerificationRequest['phase'],
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
  assert.equal(verificationRequestNeedsSasStart(request('outgoing', 'sas_ready')), true);
  assert.equal(
    verificationRequestNeedsSasStart({
      ...request('outgoing', 'sas_ready'),
      sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
    }),
    false,
  );
});

test('SAS compare requires emoji or decimal codes before confirm', () => {
  assert.equal(verificationRequestHasSasCodes(request('outgoing', 'sas_ready')), false);
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('outgoing', 'sas_ready'),
      sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
    }),
    true,
  );
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('incoming', 'sas_ready'),
      sas: { decimals: [11, 22, 33] },
    }),
    true,
  );
  assert.equal(
    verificationRequestHasSasCodes({
      ...request('outgoing', 'sas_ready'),
      sas: { decimals: [11, 22] as unknown as [number, number, number] },
    }),
    false,
  );
});

test('native verification failures use a fixed privacy-safe message', () => {
  const message = nativeVerificationErrorMessage().toLowerCase();
  for (const forbidden of ['token', 'key', 'mac', 'secret', 'ciphertext', 'recovery']) {
    assert.equal(message.includes(forbidden), false);
  }
});

test('inbox keeps the in-progress flow instead of always taking requests[0]', () => {
  const incoming = request('incoming', 'requested');
  incoming.flowId = 'incoming';
  const sas = {
    ...request('outgoing', 'sas_ready'),
    flowId: 'sas',
    sas: { emoji: [{ symbol: '🐶', description: 'Dog' }] },
  };
  const done = { ...request('outgoing', 'done'), flowId: 'done' };
  assert.equal(isNativeVerificationTerminal('done'), true);
  assert.equal(isNativeVerificationTerminal('sas_ready'), false);
  assert.equal(selectNativeVerificationRequest([incoming, sas])?.flowId, 'incoming');
  assert.equal(selectNativeVerificationRequest([incoming, sas], 'sas')?.flowId, 'sas');
  assert.equal(selectNativeVerificationRequest([done, incoming])?.flowId, 'incoming');
  assert.equal(selectNativeVerificationRequest([done])?.flowId, 'done');
  assert.equal(selectNativeVerificationRequest([]), undefined);
});
