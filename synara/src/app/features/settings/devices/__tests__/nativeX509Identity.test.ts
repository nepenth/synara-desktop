import assert from 'node:assert/strict';
import test from 'node:test';

import { parseNativeX509IdentityStatus } from '../nativeX509Identity';

test('x509 status parser accepts fingerprints and mxids without PEM', () => {
  const parsed = parseNativeX509IdentityStatus({
    enabled: true,
    hasCa: true,
    cas: [{ fingerprint: 'sha256:abcdef12', label: 'Imported CA abcdef12' }],
    signerImported: false,
    verifierConfigured: true,
    reloadRequired: false,
    certificateVerifiedIdentities: ['@bot:example.org'],
  });
  assert.deepEqual(parsed, {
    enabled: true,
    hasCa: true,
    cas: [{ fingerprint: 'sha256:abcdef12', label: 'Imported CA abcdef12' }],
    signerImported: false,
    verifierConfigured: true,
    reloadRequired: false,
    certificateVerifiedIdentities: ['@bot:example.org'],
  });
});

test('x509 status parser defaults off and rejects PEM-shaped fields', () => {
  const off = parseNativeX509IdentityStatus({
    enabled: false,
    hasCa: false,
    cas: [],
    signerImported: false,
    verifierConfigured: false,
    reloadRequired: false,
  });
  assert.equal(off?.enabled, false);
  assert.equal(off?.certificateVerifiedIdentities.length, 0);

  assert.equal(
    parseNativeX509IdentityStatus({
      enabled: true,
      hasCa: true,
      cas: [
        {
          fingerprint: '-----BEGIN CERTIFICATE-----',
          label: 'CA',
        },
      ],
      signerImported: false,
      verifierConfigured: false,
      reloadRequired: false,
    }),
    undefined
  );
  assert.equal(
    parseNativeX509IdentityStatus({
      enabled: true,
      hasCa: false,
      cas: [],
      signerImported: false,
      verifierConfigured: false,
      reloadRequired: false,
      certificateVerifiedIdentities: ['not-an-mxid'],
    }),
    undefined
  );
});
