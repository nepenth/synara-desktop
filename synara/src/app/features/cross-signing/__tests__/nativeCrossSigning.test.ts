import assert from 'node:assert/strict';
import test from 'node:test';
import {
  canOfferNativeDeviceVerification,
  isNativeCrossSigningPublished,
  nativeCrossSigningErrorMessage,
  NativeCrossSigningStatus,
} from '../nativeCrossSigning';

const status = (publication: 'missing' | 'published'): NativeCrossSigningStatus => ({
  sessionGeneration: 4,
  readiness: publication === 'published' ? 'ready' : 'setup_required',
  masterSigning: publication,
  selfSigning: publication,
  userSigning: publication,
  privateIdentity: publication === 'published' ? 'complete' : 'missing',
  ownIdentityVerification: publication === 'published' ? 'verified' : 'missing',
  bootstrap: publication === 'missing' ? 'needed' : 'not_needed',
});

test('published readiness requires the complete public identity projection', () => {
  assert.equal(isNativeCrossSigningPublished(status('published')), true);
  assert.equal(isNativeCrossSigningPublished(status('missing')), false);
  assert.equal(
    isNativeCrossSigningPublished({
      ...status('published'),
      selfSigning: 'missing',
    }),
    false
  );
});

test('device verification remains offered when identity only needs verification', () => {
  assert.equal(canOfferNativeDeviceVerification(status('published')), true);
  assert.equal(canOfferNativeDeviceVerification(status('missing')), false);
  assert.equal(canOfferNativeDeviceVerification(undefined), false);
  assert.equal(
    canOfferNativeDeviceVerification({
      ...status('missing'),
      readiness: 'verification_required',
      bootstrap: 'not_needed',
    }),
    true
  );
});

test('native failures use a fixed privacy-safe message', () => {
  const message = nativeCrossSigningErrorMessage().toLowerCase();
  for (const forbidden of [
    'token',
    'private',
    'ciphertext',
    'recovery',
    'passphrase',
    'password',
  ]) {
    assert.equal(message.includes(forbidden), false);
  }
});
