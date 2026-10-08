import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { isLastSignedInDevice, LAST_DEVICE_LOGOUT_WARNING } from '../logoutLastDevice';
import type { NativeDevice } from '../../features/settings/devices/nativeDevices';

const device = (overrides: Partial<NativeDevice>): NativeDevice => ({
  deviceId: 'DEVICE',
  trust: 'verified',
  isCurrent: false,
  ...overrides,
});

test('the only signed-in device is the last device', () => {
  assert.equal(isLastSignedInDevice({ devices: [device({ isCurrent: true })] }), true);
});

test('another signed-in device means this is not the last one', () => {
  assert.equal(
    isLastSignedInDevice({
      devices: [device({ isCurrent: true }), device({ deviceId: 'PHONE', trust: 'unverified' })],
    }),
    false
  );
});

test('a dehydrated device does not count as another signed-in device', () => {
  assert.equal(
    isLastSignedInDevice({
      devices: [
        device({ isCurrent: true }),
        device({ deviceId: 'DEHYDRATED', trust: 'dehydrated' }),
      ],
    }),
    true
  );
});

test('an unknown device list gives no verdict', () => {
  assert.equal(isLastSignedInDevice(undefined), undefined);
  assert.equal(isLastSignedInDevice({ devices: [] }), undefined);
});

test('the logout dialog shows the last-device warning', () => {
  const dialog = readFileSync('src/app/components/LogoutDialog.tsx', 'utf8');
  assert.match(dialog, /isLastSignedInDevice\(deviceSnapshot\)/);
  assert.match(dialog, /LAST_DEVICE_LOGOUT_WARNING/);
  assert.match(LAST_DEVICE_LOGOUT_WARNING, /recovery key/);
});
