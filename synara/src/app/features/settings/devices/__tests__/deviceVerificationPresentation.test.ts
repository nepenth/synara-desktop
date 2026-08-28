import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import {
  canStartCurrentDeviceVerification,
  currentDeviceVerificationAvailabilityMessage,
  resolveDeviceVerificationStatus,
} from '../deviceVerificationStatus';
import type { NativeDeviceSnapshot } from '../nativeDevices';

const devices = readFileSync(
  join(process.cwd(), 'src/app/features/settings/devices/Devices.tsx'),
  'utf8'
);

const snapshot = (
  ownVerification: NativeDeviceSnapshot['ownVerification'],
  hasDevicesToVerifyAgainst: boolean | null
): NativeDeviceSnapshot => ({
  sessionGeneration: 7,
  ownVerification,
  hasDevicesToVerifyAgainst,
  devices: [
    {
      deviceId: 'CURRENT',
      trust: ownVerification === 'verified' ? 'unverified' : 'verified',
      isCurrent: true,
    },
  ],
});

test('current-device verification uses authoritative snapshot metadata, never peer row trust', () => {
  assert.equal(resolveDeviceVerificationStatus(snapshot('unverified', true)), 'unverified');
  assert.equal(resolveDeviceVerificationStatus(snapshot('verified', true)), 'verified');
  assert.equal(resolveDeviceVerificationStatus(undefined), 'unknown');
  assert.equal(canStartCurrentDeviceVerification(snapshot('unverified', true)), true);
  assert.equal(canStartCurrentDeviceVerification(snapshot('unverified', false)), false);
  assert.equal(canStartCurrentDeviceVerification(snapshot('unverified', null)), false);
  assert.equal(canStartCurrentDeviceVerification(snapshot('verified', true)), false);
});

test('verification availability copy distinguishes ready, absent, and unknown peer authority', () => {
  assert.match(currentDeviceVerificationAvailabilityMessage(true), /Compare emoji/);
  assert.match(currentDeviceVerificationAvailabilityMessage(false), /No eligible verified session/);
  assert.match(currentDeviceVerificationAvailabilityMessage(null), /could not check/);
});

test('Devices does not leave Device Verification spinning after identity is known', () => {
  assert.match(devices, /resolveDeviceVerificationStatus\(/);
  assert.match(devices, /offerCurrentVerification/);
  assert.match(devices, /deviceSnapshot\?\.hasDevicesToVerifyAgainst/);
  assert.match(devices, /VerifyCurrentDeviceTile/);
  assert.doesNotMatch(devices, /resolveDeviceVerificationStatus\(\s*currentDevice\?\.trust/);
});
