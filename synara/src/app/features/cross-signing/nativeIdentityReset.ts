import { invokeDesktopWithAvailability } from '../../utils/desktop';
import { coreDiagnosticId } from '../room/composerSendError';
import type {
  NativeDeviceVerificationPrompt,
  NativeIdentityResetState,
  NativeIdentityResetStep,
} from '../matrix-dto/generated';
import { NATIVE_CROSS_SIGNING_CHANGED } from './nativeCrossSigning';

/**
 * S5 "Reset identity" and the one-time "Verify This Device" prompt. Core
 * keeps the reset handle for the session; the password is only a command
 * argument. iOS shows the same copy (synara-ios `IdentityResetCopy`).
 */
export const IDENTITY_RESET_COPY = {
  title: 'Reset Identity',
  settingsDescription:
    'If you have lost your recovery key and all your other devices, start over with a new identity.',
  warningIntro:
    'Only reset your identity if you have lost your recovery key and are not signed in on any other device you can verify with.',
  warnings: [
    'People you talk to will see that your identity changed.',
    'You will need to verify your other devices and contacts again.',
    'Messages that only your old keys can read may be lost, unless another device still has them.',
    'Starting the reset removes your key backup and recovery key right away, even if you cancel before it finishes.',
  ],
  password: 'Enter your account password to confirm the reset.',
  approval:
    'Your homeserver asks you to approve the reset on your account page. Approve it in your browser, then select Continue.',
  openAccountPage: 'Open Account Page',
  recovery: 'Your identity was reset. Set up recovery now to get a new recovery key.',
  cancelled:
    'Reset cancelled. Your key backup and recovery key were removed when it started. Set up recovery in Settings once this device is verified, or reset your identity to finish.',
  recoveryMayBeOff:
    'Your key backup and recovery key may already be removed. Set up recovery in Settings once this device is verified, or reset your identity to finish.',
} as const;

export const VERIFY_DEVICE_PROMPT_COPY = {
  title: 'Verify This Device',
  description:
    'Your account already has a cross-signing identity. Verify this device to read your encrypted messages and so others can trust it.',
  recoveryKey: 'Use Recovery Key',
  noRecovery:
    'This account has no recovery key set up. Verify from another device or reset your identity.',
  cantVerify: 'Lost your recovery key and all your other devices?',
  verified: 'This device is verified.',
  later: 'Later',
} as const;

export const IDENTITY_RESET_FAILED = 'Synara could not reset your identity. Try again.';

const PASSWORD_REJECTED =
  'Identity reset authentication failed. Check your password and try again.';
const APPROVAL_PENDING =
  'Synara has not seen your approval yet. Approve the reset in your browser, then select Continue.';

const IDENTITY_RESET_ERRORS: Record<string, string> = {
  'v-crypto.2-identity-reset-password-rejected': PASSWORD_REJECTED,
  'v-crypto.2-identity-reset-approval-pending': APPROVAL_PENDING,
  'v-crypto.2-identity-reset-network':
    'Synara could not reach your homeserver. Check your connection and try again.',
  'v-crypto.2-identity-reset-auth-unsupported':
    'This homeserver asks for a sign-in step Synara cannot complete for an identity reset.',
  'v-crypto.2-identity-reset-requires-session':
    'Synara is not signed in to your Matrix account. Sign in again, then try again.',
};

/** User copy for a failed reset step; never the native message. */
export const identityResetErrorMessage = (error: unknown): string => {
  const diagnosticId = coreDiagnosticId(error);
  return (diagnosticId && IDENTITY_RESET_ERRORS[diagnosticId]) || IDENTITY_RESET_FAILED;
};

/**
 * Whether a failed step may have left the account without key backup and
 * recovery: the SDK removes both before it asks for authentication. A wrong
 * password or a missing approval keeps the reset pending instead.
 */
export const identityResetMayLeaveRecoveryOff = (message: string): boolean =>
  message !== PASSWORD_REJECTED && message !== APPROVAL_PENDING;

const invokeIdentityReset = async <T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> => {
  let result;
  // Only the fixed copy leaves here; the native rejection is not attached.
  let failure: string | undefined;
  try {
    result = await invokeDesktopWithAvailability<T>(command, args);
  } catch (error) {
    failure = identityResetErrorMessage(error);
  }
  if (failure !== undefined) throw new Error(failure);
  if (!result || !result.available || result.value === undefined) {
    throw new Error(IDENTITY_RESET_FAILED);
  }
  return result.value;
};

const announceIdentityChange = (state: NativeIdentityResetState): NativeIdentityResetState => {
  if (state.step === 'complete' && typeof window !== 'undefined') {
    window.dispatchEvent(new Event(NATIVE_CROSS_SIGNING_CHANGED));
  }
  return state;
};

export const startIdentityReset = async (): Promise<NativeIdentityResetState> =>
  announceIdentityChange(
    await invokeIdentityReset<NativeIdentityResetState>('matrix_identity_reset_start')
  );

export const authenticateIdentityReset = async (
  password: string
): Promise<NativeIdentityResetState> =>
  announceIdentityChange(
    await invokeIdentityReset<NativeIdentityResetState>('matrix_identity_reset_password', {
      password,
    })
  );

export const continueIdentityReset = async (): Promise<NativeIdentityResetState> =>
  announceIdentityChange(
    await invokeIdentityReset<NativeIdentityResetState>('matrix_identity_reset_continue')
  );

export const cancelIdentityReset = async (): Promise<void> => {
  try {
    await invokeDesktopWithAvailability<boolean>('matrix_identity_reset_cancel');
  } catch {
    // Nothing is left to undo; a later Start replaces any pending reset.
  }
};

/** Only an http(s) account page is opened in the browser. */
export const identityResetApprovalUrl = (state: NativeIdentityResetState): string | undefined => {
  if (state.step !== 'approval_required' || !state.approvalUrl) return undefined;
  try {
    const url = new URL(state.approvalUrl);
    return url.protocol === 'https:' || url.protocol === 'http:' ? url.href : undefined;
  } catch {
    return undefined;
  }
};

/** Whether Core wants the prompt now. Any failure keeps it hidden. */
export const shouldShowDeviceVerificationPrompt = async (): Promise<boolean> => {
  try {
    const result = await invokeDesktopWithAvailability<NativeDeviceVerificationPrompt>(
      'matrix_device_verification_prompt',
      undefined,
      { suppressErrorDiagnostic: true }
    );
    return result.available && result.value?.show === true;
  } catch {
    return false;
  }
};

/** "Later" (or any choice): Core hides the prompt for this session. */
export const dismissDeviceVerificationPrompt = async (): Promise<void> => {
  try {
    await invokeDesktopWithAvailability<NativeDeviceVerificationPrompt>(
      'matrix_device_verification_prompt_dismiss'
    );
  } catch {
    // Hidden locally either way; Settings keeps the entry.
  }
};

export type { NativeDeviceVerificationPrompt, NativeIdentityResetState, NativeIdentityResetStep };
