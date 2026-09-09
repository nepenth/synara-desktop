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
  assert.match(devices, /VerifyCurrentDeviceTile/);
  assert.doesNotMatch(devices, /resolveDeviceVerificationStatus\(\s*currentDevice\?\.trust/);
});

test('Devices only offers current-device verification from a loaded snapshot and surfaces load failures', () => {
  // A missing snapshot must render as loading or as a retryable failure, never
  // as the "could not check eligible verified sessions" verification prompt.
  assert.match(devices, /deviceSnapshot !== undefined &&\s*canOfferNativeDeviceVerification/);
  assert.match(devices, /hasDevicesToVerifyAgainst=\{deviceSnapshot\.hasDevicesToVerifyAgainst\}/);
  assert.doesNotMatch(devices, /deviceSnapshot\?\.hasDevicesToVerifyAgainst \?\? null/);
  assert.match(devices, /snapshotFailed && \(/);
  assert.match(devices, /title="Device list unavailable"/);
  assert.match(devices, /description=\{deviceLoadState\.error\}/);
  assert.match(devices, /snapshotPending && <DevicesPlaceholder \/>/);
});

test('useDeviceList is gated on session presence, not the optional bootstrap generation marker', () => {
  const hook = readFileSync(join(process.cwd(), 'src/app/hooks/useDeviceList.ts'), 'utf8');
  const platformSessions = readFileSync(
    join(process.cwd(), 'src/app/platform/sessions.ts'),
    'utf8'
  );
  // The desktop platform store never populates `Session.sessionGeneration`,
  // so gating the query on it disabled the Devices page entirely and turned
  // every retry into a no-op.
  assert.doesNotMatch(platformSessions, /sessionGeneration/);
  assert.doesNotMatch(hook, /enabled: sessionGeneration !== undefined/);
  assert.doesNotMatch(hook, /getActiveSession\(\)\?\.sessionGeneration/);
  assert.match(hook, /enabled: sessionKey !== undefined/);
  assert.match(hook, /retry: false/);
  assert.match(hook, /error: error \? describeDeviceListError\(error\) : undefined/);
});
