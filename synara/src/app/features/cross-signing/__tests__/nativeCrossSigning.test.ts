import assert from 'node:assert/strict';
import test from 'node:test';
import {
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
