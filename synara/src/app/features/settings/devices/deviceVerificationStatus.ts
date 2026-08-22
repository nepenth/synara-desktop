import type { NativeCrossSigningStatus } from '../../cross-signing/nativeCrossSigning';
import type { VerificationStatus } from './nativeDevices';

export function resolveDeviceVerificationStatus(
  currentTrust: VerificationStatus | undefined,
  ownIdentityVerification: NativeCrossSigningStatus['ownIdentityVerification'] | undefined,
  loading: boolean
): VerificationStatus {
  if (currentTrust && currentTrust !== 'unknown') {
    return currentTrust;
  }
  if (ownIdentityVerification === 'verified') {
    return 'verified';
  }
  if (ownIdentityVerification === 'unverified') {
    return 'unverified';
  }
  return loading ? 'unknown' : 'unverified';
}
