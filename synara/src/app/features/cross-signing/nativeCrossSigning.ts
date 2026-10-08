import { invokeDesktopWithAvailability } from '../../utils/desktop';
import type {
  NativeCrossSigningKeyPublication,
  NativeCrossSigningPrivateIdentity,
  NativeCrossSigningReadiness,
  NativeCrossSigningSetupResult,
  NativeCrossSigningStatus,
  NativeOwnIdentityVerification,
} from '../matrix-dto/generated';

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

export type {
  NativeCrossSigningKeyPublication,
  NativeCrossSigningPrivateIdentity,
  NativeCrossSigningReadiness,
  NativeCrossSigningSetupResult,
  NativeCrossSigningStatus,
  NativeOwnIdentityVerification,
};
