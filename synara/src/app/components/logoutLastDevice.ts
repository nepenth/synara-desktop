import type { NativeDeviceSnapshot } from '../features/settings/devices/nativeDevices';

export const LAST_DEVICE_LOGOUT_TITLE = 'Last Signed-In Device';

export const LAST_DEVICE_LOGOUT_WARNING =
  'This is your last signed-in device. Save your recovery key before logging out, or you will lose access to your encrypted messages.';

/**
 * Whether this is the account's only signed-in device, matching the SDK's
 * `Recovery::is_last_device`: a dehydrated device cannot sign back in, so it
 * does not count. `undefined` while the device list is unknown.
 */
export const isLastSignedInDevice = (
  snapshot: Pick<NativeDeviceSnapshot, 'devices'> | null | undefined
): boolean | undefined => {
  if (!snapshot || snapshot.devices.length === 0) return undefined;
  return !snapshot.devices.some((device) => !device.isCurrent && device.trust !== 'dehydrated');
};
