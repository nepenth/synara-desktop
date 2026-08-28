import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativeDeviceTrust = 'verified' | 'unverified' | 'unsupported';
export type VerificationStatus = NativeDeviceTrust | 'unknown';

export type NativeDevice = {
  deviceId: string;
  displayName?: string;
  lastSeenIp?: string;
  lastSeenTs?: number;
  trust: NativeDeviceTrust;
  isCurrent: boolean;
};

export type NativeDeviceSnapshot = {
  sessionGeneration: number;
  ownVerification: 'unknown' | 'unverified' | 'verified';
  hasDevicesToVerifyAgainst: boolean | null;
  devices: NativeDevice[];
};

export type NativeDeviceDeleteAuthentication = 'password';

export type NativeDeviceDeleteChallenge = {
  operationId: number;
  sessionGeneration: number;
  authentication: NativeDeviceDeleteAuthentication;
  authenticationFailed: boolean;
};

export type NativeDeviceDeleteResult =
  | { outcome: 'complete'; snapshot: NativeDeviceSnapshot }
  | { outcome: 'authentication_required'; challenge: NativeDeviceDeleteChallenge };

const invokeNativeDevices = async <T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> => {
  const result = await invokeDesktopWithAvailability<T>(command, args);
  if (!result.available || result.value === undefined) {
    throw new Error('Native Matrix device management is unavailable.');
  }
  return result.value;
};

export const getNativeDeviceSnapshot = (): Promise<NativeDeviceSnapshot> =>
  invokeNativeDevices('matrix_device_snapshot');

export const renameNativeDevice = (
  deviceId: string,
  displayName: string
): Promise<NativeDeviceSnapshot> =>
  invokeNativeDevices('matrix_device_rename', { deviceId, displayName });

export const startNativeDeviceDelete = (deviceIds: string[]): Promise<NativeDeviceDeleteResult> =>
  invokeNativeDevices('matrix_device_delete_start', { deviceIds });

export const authenticateNativeDeviceDeletePassword = (
  operationId: number,
  sessionGeneration: number,
  password: string
): Promise<NativeDeviceDeleteResult> =>
  invokeNativeDevices('matrix_device_delete_password', {
    operationId,
    sessionGeneration,
    password,
  });

export const cancelNativeDeviceDelete = (
  operationId: number,
  sessionGeneration: number
): Promise<void> =>
  invokeNativeDevices('matrix_device_delete_cancel', { operationId, sessionGeneration });
