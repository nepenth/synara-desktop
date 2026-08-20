import { invokeDesktopWithAvailability } from '../../utils/desktop';

export type NativeCrossSigningKeyPublication = 'missing' | 'published';
export type NativeCrossSigningPrivateIdentity = 'missing' | 'partial' | 'complete';
export type NativeOwnIdentityVerification = 'missing' | 'unverified' | 'verified';
export type NativeCrossSigningReadiness =
  | 'unavailable'
  | 'setup_required'
  | 'recovery_required'
  | 'verification_required'
  | 'ready';

export type NativeCrossSigningStatus = {
  sessionGeneration: number;
  readiness: NativeCrossSigningReadiness;
  masterSigning: NativeCrossSigningKeyPublication;
  selfSigning: NativeCrossSigningKeyPublication;
  userSigning: NativeCrossSigningKeyPublication;
  privateIdentity: NativeCrossSigningPrivateIdentity;
  ownIdentityVerification: NativeOwnIdentityVerification;
  bootstrap: 'needed' | 'not_needed';
};

export type NativeCrossSigningSetupResult = {
  outcome: 'complete' | 'already_configured' | 'authentication_required';
  status: NativeCrossSigningStatus;
};

export const NATIVE_CROSS_SIGNING_CHANGED = 'synara-native-cross-signing-changed';

export const nativeCrossSigningErrorMessage = (): string =>
  'Native cross-signing is unavailable. Restart Synara and try again.';

const invokeNativeCrossSigning = async <T>(
  command: string,
  args?: Record<string, unknown>,
  errorMessage = nativeCrossSigningErrorMessage()
): Promise<T> => {
  try {
    const result = await invokeDesktopWithAvailability<T>(command, args);
    if (!result.available || result.value === undefined) {
      throw new Error(errorMessage);
    }
    return result.value;
  } catch {
    throw new Error(errorMessage);
  }
};

const announceStatusChange = (): void => {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new Event(NATIVE_CROSS_SIGNING_CHANGED));
  }
};

export const getNativeCrossSigningStatus = (): Promise<NativeCrossSigningStatus> =>
  invokeNativeCrossSigning('matrix_cross_signing_status');

export const startNativeCrossSigningSetup = async (): Promise<NativeCrossSigningSetupResult> => {
  const result = await invokeNativeCrossSigning<NativeCrossSigningSetupResult>(
    'matrix_cross_signing_setup'
  );
  announceStatusChange();
  return result;
};

export const authenticateNativeCrossSigningSetup = async (
  password: string
): Promise<NativeCrossSigningSetupResult> => {
  const result = await invokeNativeCrossSigning<NativeCrossSigningSetupResult>(
    'matrix_cross_signing_setup_password',
    { password },
    'Cross-signing authentication failed. Check your account password and try again.'
  );
  announceStatusChange();
  return result;
};

export const isNativeCrossSigningPublished = (status: NativeCrossSigningStatus): boolean =>
  status.masterSigning === 'published' &&
  status.selfSigning === 'published' &&
  status.userSigning === 'published';

export const canOfferNativeDeviceVerification = (status?: NativeCrossSigningStatus): boolean => {
  if (!status) return false;
  return (
    isNativeCrossSigningPublished(status) ||
    status.readiness === 'verification_required' ||
    status.bootstrap === 'not_needed'
  );
};
