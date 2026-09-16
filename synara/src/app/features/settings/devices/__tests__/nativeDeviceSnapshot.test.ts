import assert from 'node:assert/strict';
import test from 'node:test';

import {
  parseNativeDeviceSnapshot,
  nativeDeviceTrustLabel,
  type NativeDeviceSnapshot,
} from '../nativeDevices';

test('device snapshot parser accepts additive trust fields and aliases unsupported', () => {
  const snapshot = parseNativeDeviceSnapshot({
    sessionGeneration: 7,
    ownVerification: 'verified',
    hasDevicesToVerifyAgainst: true,
    devices: [
      {
        deviceId: 'CURRENT',
        displayName: 'This Mac',
        lastSeenIp: '192.0.2.1',
        lastSeenTs: 1,
        trust: 'verified',
        isCurrent: true,
        isCrossSignedByOwner: true,
        firstSeenTs: 1,
        ed25519Fingerprint: 'ABCD EFGH',
      },
      {
        deviceId: 'OLD',
        trust: 'unsupported',
        isCurrent: false,
      },
      {
        deviceId: 'SAS',
        trust: 'verified_locally_only',
        isCurrent: false,
      },
      {
        deviceId: 'CERT',
        trust: 'verified_by_certificate',
        isCurrent: false,
      },
    ],
  });
  assert.ok(snapshot);
  assert.equal(snapshot.devices[0]?.ed25519Fingerprint, 'ABCD EFGH');
  assert.equal(snapshot.devices[0]?.isCrossSignedByOwner, true);
  assert.equal(snapshot.devices[1]?.trust, 'no_encryption');
  assert.equal(snapshot.devices[2]?.trust, 'verified_locally_only');
  assert.equal(snapshot.devices[3]?.trust, 'verified_by_certificate');
});

test('device trust labels distinguish backup devices from unencrypted sessions', () => {
  assert.equal(nativeDeviceTrustLabel('verified'), 'Verified');
  assert.equal(nativeDeviceTrustLabel('verified_locally_only'), 'Verified');
  assert.equal(nativeDeviceTrustLabel('verified_by_certificate'), 'Verified (certificate)');
  assert.equal(nativeDeviceTrustLabel('unverified'), 'Unverified');
  assert.equal(nativeDeviceTrustLabel('dehydrated'), 'Backup device');
  assert.equal(nativeDeviceTrustLabel('no_encryption'), 'Not encrypted');
});

test('device snapshot parser stays tolerant of missing optional fields', () => {
  const snapshot = parseNativeDeviceSnapshot({
    sessionGeneration: 3,
    ownVerification: 'unverified',
    hasDevicesToVerifyAgainst: null,
    devices: [{ deviceId: 'ONLY', trust: 'unverified', isCurrent: true }],
  });
  assert.deepEqual(snapshot, {
    sessionGeneration: 3,
    ownVerification: 'unverified',
    hasDevicesToVerifyAgainst: null,
    devices: [{ deviceId: 'ONLY', trust: 'unverified', isCurrent: true }],
  } satisfies NativeDeviceSnapshot);
});

test('device snapshot parser rejects unbounded and hostile strings', () => {
  assert.equal(
    parseNativeDeviceSnapshot({
      sessionGeneration: 3,
      ownVerification: 'verified',
      hasDevicesToVerifyAgainst: false,
      devices: [{ deviceId: 'X'.repeat(300), trust: 'verified', isCurrent: true }],
    }),
    undefined
  );

  const snapshot = parseNativeDeviceSnapshot({
    sessionGeneration: 3,
    ownVerification: 'unverified',
    hasDevicesToVerifyAgainst: false,
    devices: [
      {
        deviceId: 'ONLY',
        trust: 'unverified',
        isCurrent: true,
        displayName: 'n'.repeat(300),
        lastSeenIp: '1'.repeat(80),
        ed25519Fingerprint: '<script>alert(1)</script>',
      },
    ],
  });
  assert.ok(snapshot);
  assert.equal(snapshot.devices[0]?.displayName, undefined);
  assert.equal(snapshot.devices[0]?.lastSeenIp, undefined);
  assert.equal(snapshot.devices[0]?.ed25519Fingerprint, undefined);
});

test('device snapshot parser rejects malformed trust and missing identity', () => {
  assert.equal(
    parseNativeDeviceSnapshot({
      sessionGeneration: 3,
      ownVerification: 'verified',
      hasDevicesToVerifyAgainst: false,
      devices: [{ deviceId: 'X', trust: 'mystery', isCurrent: false }],
    }),
    undefined
  );
  assert.equal(
    parseNativeDeviceSnapshot({
      sessionGeneration: 3,
      ownVerification: 'verified',
      hasDevicesToVerifyAgainst: false,
      devices: [{ trust: 'verified', isCurrent: true }],
    }),
    undefined
  );
});
