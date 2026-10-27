import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type ChangePasswordInput = {
  currentPassword: string;
  newPassword: string;
  logoutDevices: boolean;
};

/** Thrown when the homeserver manages passwords on its own account page. */
export class PasswordChangeUnsupportedError extends Error {}

const MESSAGES: Record<string, string> = {
  'v-password.current-wrong': 'The current password is incorrect.',
  'v-password.weak': 'The homeserver rejected the new password as too weak. Try a longer one.',
  'v-password.rate-limited': 'Too many attempts. Wait a moment and try again.',
  'v-password.current-empty': 'Enter your current password.',
  'v-password.new-empty': 'Enter a new password.',
  'v-password.current-too-long': 'The password is too long.',
  'v-password.new-too-long': 'The password is too long.',
  'd0.4-send-requires-session': 'Sign in again to change your password.',
};

const diagnosticIdOf = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object') return undefined;
  const id = (error as Record<string, unknown>).diagnosticId;
  return typeof id === 'string' ? id : undefined;
};

/**
 * Change the signed-in user's password. Rejections carry only a fixed
 * product message: native error values are never shown or logged as-is.
 */
export async function nativeChangePassword(
  input: ChangePasswordInput,
  invoke: typeof invokeDesktopWithAvailability = invokeDesktopWithAvailability
): Promise<void> {
  let result;
  try {
    result = await invoke<null>(
      'matrix_change_own_password',
      {
        currentPassword: input.currentPassword,
        newPassword: input.newPassword,
        logoutDevices: input.logoutDevices,
      },
      // The rejection is mapped to a fixed message below.
      { suppressErrorDiagnostic: true }
    );
  } catch (error) {
    const id = diagnosticIdOf(error);
    if (id === 'v-password.auth-unsupported') {
      throw new PasswordChangeUnsupportedError(
        'This homeserver changes passwords on its own account page.',
        { cause: error }
      );
    }
    throw new Error((id && MESSAGES[id]) ?? 'The password could not be changed. Try again.', {
      cause: error,
    });
  }
  if (!result.available) throw new Error('Password change needs the Synara desktop app.');
}

/** Why the form cannot be sent yet, or `undefined` when it can. */
export const passwordFormProblem = (
  currentPassword: string,
  newPassword: string,
  confirmPassword: string
): string | undefined => {
  if (!currentPassword || !newPassword || !confirmPassword) return 'Fill in all three fields.';
  if (newPassword !== confirmPassword) return 'The new passwords do not match.';
  if (newPassword === currentPassword) return 'The new password is the same as the current one.';
  return undefined;
};
