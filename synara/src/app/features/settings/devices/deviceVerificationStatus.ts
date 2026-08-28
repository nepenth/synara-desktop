import type { NativeDeviceSnapshot, VerificationStatus } from './nativeDevices';

export function resolveDeviceVerificationStatus(
  snapshot: NativeDeviceSnapshot | undefined
): VerificationStatus {
  return snapshot?.ownVerification ?? 'unknown';
}

export const canStartCurrentDeviceVerification = (
  snapshot: NativeDeviceSnapshot | undefined
): boolean =>
  snapshot?.ownVerification !== 'verified' && snapshot?.hasDevicesToVerifyAgainst === true;

export const currentDeviceVerificationAvailabilityMessage = (
  hasDevicesToVerifyAgainst: boolean | null | undefined
): string => {
  if (hasDevicesToVerifyAgainst === true) {
    return 'Compare emoji or number codes with another verified session. Synara does not mark this device verified until both sides confirm.';
  }
  if (hasDevicesToVerifyAgainst === false) {
    return 'No eligible verified session is available yet. Open Synara or Element on a device that already verified this account, then refresh.';
  }
  return 'Synara could not check eligible verified sessions. Check your connection and retry.';
};
