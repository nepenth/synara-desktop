import test from 'node:test';
import assert from 'node:assert/strict';
import {
  type ShowSasCallbacks,
  VerificationPhase,
  type VerificationRequest,
  type Verifier,
} from 'matrix-js-sdk/lib/crypto-api';
import {
  cancelVerificationRequestForExit,
  getInitialSasCallbacks,
  phaseFromVerifierCancellation,
  shouldCancelActiveVerificationRequest,
  verificationErrorMessage,
} from '../verification';
import {
  ensureVerificationRequestInbox,
  mergeSelfVerificationRequests,
} from '../../../client/verificationRequestInbox';
import type { MatrixClient } from 'matrix-js-sdk';

test('verification user exits only cancel active requests', () => {
  assert.equal(shouldCancelActiveVerificationRequest(VerificationPhase.Requested), true);
  assert.equal(shouldCancelActiveVerificationRequest(VerificationPhase.Ready), true);
  assert.equal(shouldCancelActiveVerificationRequest(VerificationPhase.Started), true);
  assert.equal(shouldCancelActiveVerificationRequest(VerificationPhase.Cancelled), false);
  assert.equal(shouldCancelActiveVerificationRequest(VerificationPhase.Done), false);
});

test('verification verifier cancellation does not override completed requests', () => {
  assert.equal(
    phaseFromVerifierCancellation(VerificationPhase.Started, true),
    VerificationPhase.Cancelled
  );
  assert.equal(phaseFromVerifierCancellation(VerificationPhase.Done, true), VerificationPhase.Done);
  assert.equal(
    phaseFromVerifierCancellation(VerificationPhase.Started, false),
    VerificationPhase.Started
  );
});

test('verification error messages are safe fallbacks', () => {
  assert.equal(
    verificationErrorMessage(new Error('MAC could not be sent')),
    'MAC could not be sent'
  );
  assert.equal(verificationErrorMessage('Verification timed out'), 'Verification timed out');
  assert.equal(verificationErrorMessage(undefined), 'Verification failed.');
});

const verificationRequest = (
  transactionId: string,
  isSelfVerification = true
): VerificationRequest => ({ transactionId, isSelfVerification } as unknown as VerificationRequest);

test('verification inbox filters non-self requests and deduplicates transaction IDs', () => {
  const first = verificationRequest('txn-1');
  const duplicate = verificationRequest('txn-1');
  const second = verificationRequest('txn-2');
  const otherUser = verificationRequest('txn-other', false);

  assert.deepEqual(mergeSelfVerificationRequests([first], [duplicate, otherUser, second]), [
    first,
    second,
  ]);
});

test('verification inbox queues requests received before the UI subscribes', () => {
  let receive: ((request: VerificationRequest) => void) | undefined;
  const mx = {
    on: (_event: string, listener: (request: VerificationRequest) => void) => {
      receive = listener;
    },
  } as unknown as MatrixClient;
  const first = verificationRequest('txn-early');
  const second = verificationRequest('txn-hydrated');

  const inbox = ensureVerificationRequestInbox(mx);
  receive?.(first);
  inbox.hydrate([first, second]);

  assert.deepEqual(inbox.getSnapshot(), [first, second]);
  inbox.dismiss(first);
  assert.deepEqual(inbox.getSnapshot(), [second]);
});

test('SAS verification seeds callbacks that existed before the React listener', () => {
  const callbacks = {
    sas: { emoji: [['🐶', 'Dog']] },
    confirm: async () => undefined,
    mismatch: () => undefined,
    cancel: () => undefined,
  } as ShowSasCallbacks;
  const verifier = {
    getShowSasCallbacks: () => callbacks,
  } as unknown as Verifier;

  assert.equal(getInitialSasCallbacks(verifier), callbacks);
});

test('verification exit awaits cancellation and preserves failures for the caller', async () => {
  const calls: string[] = [];
  const request = {
    phase: VerificationPhase.Started,
    cancel: async () => {
      calls.push('cancel');
      throw new Error('transport unavailable');
    },
  } as unknown as VerificationRequest;

  await assert.rejects(() => cancelVerificationRequestForExit(request), /transport unavailable/);
  assert.deepEqual(calls, ['cancel']);
});

test('verification exit does not cancel terminal requests', async () => {
  let cancelled = false;
  const request = {
    phase: VerificationPhase.Done,
    cancel: async () => {
      cancelled = true;
    },
  } as unknown as VerificationRequest;

  await cancelVerificationRequestForExit(request);
  assert.equal(cancelled, false);
});
