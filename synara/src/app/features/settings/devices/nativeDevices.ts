import { invokeDesktopWithAvailability, listen } from '../../../utils/desktop';

// The shared native owner observes device lists, verification, recovery and
// backup state. Signals invalidate status reads; they never carry secrets.
export const subscribeNativeDeviceUpdates = (
  onUpdate: (sessionGeneration: number) => void
): (() => void) => {
  let disposed = false;
  let unlisten: (() => void | Promise<void>) | undefined;
  void listen<{ sessionGeneration: number }>('matrix-device-list-updated', (event) => {
    if (!disposed) onUpdate(event.payload.sessionGeneration);
  })
    .then((cleanup) => {
      if (disposed) void cleanup?.();
      else unlisten = cleanup;
    })
    .catch(() => undefined);
  return () => {
    disposed = true;
    void unlisten?.();
  };
};

export type NativeDeviceTrust =
  | 'verified'
  | 'verified_locally_only'
  | 'unverified'
  | 'no_encryption'
  | 'dehydrated';

export type NativeOwnDeviceVerification = 'unknown' | 'unverified' | 'verified';
export type VerificationStatus = NativeOwnDeviceVerification;

export type NativeDevice = {
  deviceId: string;
  displayName?: string;
  lastSeenIp?: string;
  lastSeenTs?: number;
  trust: NativeDeviceTrust;
  isCurrent: boolean;
  isCrossSignedByOwner?: boolean;
  firstSeenTs?: number;
  ed25519Fingerprint?: string;
};

export type NativeDeviceSnapshot = {
  sessionGeneration: number;
  ownVerification: NativeOwnDeviceVerification;
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

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isSafeCounter = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const optionalBoundedString = (value: unknown): string | undefined => {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  return trimmed.length === 0 ? undefined : trimmed;
};

const optionalTimestamp = (value: unknown): number | undefined => {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) return undefined;
  return value;
};

const parseTrust = (value: unknown): NativeDeviceTrust | undefined => {
  if (value === 'verified' || value === 'unverified' || value === 'dehydrated') return value;
  if (value === 'verified_locally_only') return value;
  if (value === 'no_encryption' || value === 'unsupported') return 'no_encryption';
  return undefined;
};

const parseOwnVerification = (value: unknown): NativeOwnDeviceVerification | undefined => {
  if (value === 'unknown' || value === 'unverified' || value === 'verified') return value;
  return undefined;
};

const parseDevice = (value: unknown): NativeDevice | undefined => {
  if (!isRecord(value)) return undefined;
  const deviceId = optionalBoundedString(value.deviceId);
  const trust = parseTrust(value.trust);
  if (!deviceId || !trust) return undefined;
  const isCurrent = value.isCurrent === true;
  const device: NativeDevice = {
    deviceId,
    trust,
    isCurrent,
  };
  const displayName = optionalBoundedString(value.displayName);
  if (displayName) device.displayName = displayName;
  const lastSeenIp = optionalBoundedString(value.lastSeenIp);
  if (lastSeenIp) device.lastSeenIp = lastSeenIp;
  const lastSeenTs = optionalTimestamp(value.lastSeenTs);
  if (lastSeenTs !== undefined) device.lastSeenTs = lastSeenTs;
  if (typeof value.isCrossSignedByOwner === 'boolean') {
    device.isCrossSignedByOwner = value.isCrossSignedByOwner;
  }
  const firstSeenTs = optionalTimestamp(value.firstSeenTs);
  if (firstSeenTs !== undefined) device.firstSeenTs = firstSeenTs;
  const fingerprint = optionalBoundedString(value.ed25519Fingerprint);
  if (fingerprint) device.ed25519Fingerprint = fingerprint;
  return device;
};

export const parseNativeDeviceSnapshot = (value: unknown): NativeDeviceSnapshot | undefined => {
  if (!isRecord(value)) return undefined;
  if (!isSafeCounter(value.sessionGeneration)) return undefined;
  const ownVerification = parseOwnVerification(value.ownVerification);
  if (!ownVerification) return undefined;
  if (
    value.hasDevicesToVerifyAgainst !== null &&
    typeof value.hasDevicesToVerifyAgainst !== 'boolean'
  ) {
    return undefined;
  }
  if (!Array.isArray(value.devices)) return undefined;
  const devices: NativeDevice[] = [];
  for (const device of value.devices) {
    const parsed = parseDevice(device);
    if (!parsed) return undefined;
    devices.push(parsed);
  }
  return {
    sessionGeneration: value.sessionGeneration,
    ownVerification,
    hasDevicesToVerifyAgainst: value.hasDevicesToVerifyAgainst,
    devices,
  };
};

const parseNativeDeviceDeleteResult = (value: unknown): NativeDeviceDeleteResult | undefined => {
  if (!isRecord(value)) return undefined;
  if (value.outcome === 'complete') {
    const snapshot = parseNativeDeviceSnapshot(value.snapshot);
    if (!snapshot) return undefined;
    return { outcome: 'complete', snapshot };
  }
  if (value.outcome === 'authentication_required' && isRecord(value.challenge)) {
    const { operationId, sessionGeneration, authentication, authenticationFailed } =
      value.challenge;
    if (
      !isSafeCounter(operationId) ||
      !isSafeCounter(sessionGeneration) ||
      authentication !== 'password' ||
      typeof authenticationFailed !== 'boolean'
    ) {
      return undefined;
    }
    return {
      outcome: 'authentication_required',
      challenge: { operationId, sessionGeneration, authentication, authenticationFailed },
    };
  }
  return undefined;
};

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

const requireSnapshot = (value: unknown): NativeDeviceSnapshot => {
  const snapshot = parseNativeDeviceSnapshot(value);
  if (!snapshot) {
    throw new Error('Native Matrix device management is unavailable.');
  }
  return snapshot;
};

const requireDeleteResult = (value: unknown): NativeDeviceDeleteResult => {
  const result = parseNativeDeviceDeleteResult(value);
  if (!result) {
    throw new Error('Native Matrix device management is unavailable.');
  }
  return result;
};

export const getNativeDeviceSnapshot = async (): Promise<NativeDeviceSnapshot> =>
  requireSnapshot(await invokeNativeDevices('matrix_device_snapshot'));

export const renameNativeDevice = async (
  deviceId: string,
  displayName: string
): Promise<NativeDeviceSnapshot> =>
  requireSnapshot(await invokeNativeDevices('matrix_device_rename', { deviceId, displayName }));

export const startNativeDeviceDelete = async (
  deviceIds: string[]
): Promise<NativeDeviceDeleteResult> =>
  requireDeleteResult(await invokeNativeDevices('matrix_device_delete_start', { deviceIds }));

export const authenticateNativeDeviceDeletePassword = async (
  operationId: number,
  sessionGeneration: number,
  password: string
): Promise<NativeDeviceDeleteResult> =>
  requireDeleteResult(
    await invokeNativeDevices('matrix_device_delete_password', {
      operationId,
      sessionGeneration,
      password,
    })
  );

export const cancelNativeDeviceDelete = (
  operationId: number,
  sessionGeneration: number
): Promise<void> =>
  invokeNativeDevices('matrix_device_delete_cancel', { operationId, sessionGeneration });
