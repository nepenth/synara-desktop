import test from 'node:test';
import assert from 'node:assert/strict';
import { VerificationPhase } from 'matrix-js-sdk/lib/crypto-api';
import {
  phaseFromVerifierCancellation,
  shouldCancelActiveVerificationRequest,
  verificationErrorMessage,
} from '../verification';

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
