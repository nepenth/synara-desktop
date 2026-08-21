import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { resolveDeviceVerificationStatus } from '../deviceVerificationStatus';

const devices = readFileSync(
  join(process.cwd(), 'src/app/features/settings/devices/Devices.tsx'),
  'utf8'
);

test('Devices maps verification status from identity when the current device row is missing', () => {
  assert.equal(resolveDeviceVerificationStatus(undefined, 'unverified', false), 'unverified');
  assert.equal(resolveDeviceVerificationStatus(undefined, 'verified', false), 'verified');
  assert.equal(resolveDeviceVerificationStatus('verified', 'unverified', false), 'verified');
  assert.equal(resolveDeviceVerificationStatus(undefined, undefined, true), 'unknown');
  assert.equal(resolveDeviceVerificationStatus(undefined, undefined, false), 'unverified');
});

test('Devices does not leave Device Verification spinning after identity is known', () => {
  assert.match(devices, /resolveDeviceVerificationStatus\(/);
  assert.match(devices, /offerCurrentVerification/);
  assert.match(devices, /VerifyCurrentDeviceTile/);
  assert.doesNotMatch(devices, /currentDevice\?\.trust \?\? 'unknown'/);
});
