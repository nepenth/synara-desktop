import { synaraDeviceDisplayName } from '../utils/user-agent';

export type PlatformDeviceRecord = {
  display_name?: string;
};

export type PlatformDeviceNameClient = {
  getDeviceId: () => string | null | undefined;
  getDevice: (deviceId: string) => Promise<PlatformDeviceRecord | undefined>;
  setDeviceDetails: (deviceId: string, details: { display_name: string }) => Promise<unknown>;
};

export const getPlatformDeviceDisplayName = synaraDeviceDisplayName;

export const repairPlatformDeviceDisplayName = async (
  mx: PlatformDeviceNameClient,
  displayName = getPlatformDeviceDisplayName()
): Promise<boolean> => {
  const deviceId = mx.getDeviceId();
  if (!deviceId) return false;

  const currentDevice = await mx.getDevice(deviceId).catch(() => undefined);
  if (currentDevice?.display_name === displayName) return false;

  await mx.setDeviceDetails(deviceId, { display_name: displayName });
  return true;
};
