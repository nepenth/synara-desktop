import assert from 'node:assert/strict';
import test from 'node:test';

import { parseNativeDeviceSnapshot, type NativeDeviceSnapshot } from '../nativeDevices';

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
    ],
  });
  assert.ok(snapshot);
  assert.equal(snapshot.devices[0]?.ed25519Fingerprint, 'ABCD EFGH');
  assert.equal(snapshot.devices[0]?.isCrossSignedByOwner, true);
  assert.equal(snapshot.devices[1]?.trust, 'no_encryption');
  assert.equal(snapshot.devices[2]?.trust, 'verified_locally_only');
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
