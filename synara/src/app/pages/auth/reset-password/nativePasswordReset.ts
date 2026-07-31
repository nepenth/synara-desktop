/**
 * V-AUTH.4a — native desktop password-reset IPC owner.
 * No matrix-js-sdk. Secrets are never logged here.
 */

import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativePasswordEmailTokenResult = {
  sid: string;
  submitUrl?: string | null;
};

export type NativePasswordResetOutcome =
  | { status: 'complete' }
  | {
      status: 'email_not_verified';
      session?: string | null;
      errorCode?: string | null;
      errorMessage?: string | null;
    };

export type NativePasswordResetCommandError = {
  code?: string;
  message?: string;
  diagnosticId?: string;
};

export type NativePasswordResetInvoke = <T>(
  command: string,
  args?: Record<string, unknown>
) => Promise<{ available: false } | { available: true; value: T | undefined }>;

const defaultInvoke: NativePasswordResetInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);

const mapNativeError = (error: unknown): Error => {
  if (error instanceof Error) return error;
  if (typeof error === 'object' && error !== null) {
    const e = error as NativePasswordResetCommandError;
    const code = typeof e.code === 'string' ? e.code : 'Unknown';
    const message =
      typeof e.message === 'string' && e.message.trim()
        ? e.message
        : 'Native password reset failed.';
    // Never append diagnostic payloads that might echo request fields.
    return new Error(`${code}: ${message}`);
  }
  return new Error('Native password reset failed.');
};

const invokeNative = async <T>(
  command: string,
  args: Record<string, unknown>,
  invoke: NativePasswordResetInvoke = defaultInvoke
): Promise<T> => {
  try {
    const result = await invoke<T>(command, args);
    if (!result.available || result.value === undefined) {
      throw new Error('Native password reset is unavailable.');
    }
    return result.value;
  } catch (error) {
    if (error instanceof Error && error.message.includes('unavailable')) {
      throw error;
    }
    throw mapNativeError(error);
  }
};

/** Generate a Matrix client secret (UUID without hyphens). */
export const generatePasswordResetClientSecret = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID().replace(/-/g, '');
  }
  // Fallback for non-browser test harnesses.
  const bytes = new Uint8Array(16);
  if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
    crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i += 1) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
};

export const requestPasswordResetEmailToken = (
  homeserverUrl: string,
  email: string,
  clientSecret: string,
  sendAttempt: number,
  invoke: NativePasswordResetInvoke = defaultInvoke
): Promise<NativePasswordEmailTokenResult> =>
  invokeNative(
    'matrix_password_reset_request_email_token',
    {
      homeserverUrl,
      email,
      clientSecret,
      sendAttempt,
    },
    invoke
  );

export const completePasswordReset = (
  homeserverUrl: string,
  email: string,
  newPassword: string,
  clientSecret: string,
  sid: string,
  invoke: NativePasswordResetInvoke = defaultInvoke
): Promise<NativePasswordResetOutcome> =>
  invokeNative(
    'matrix_password_reset_complete',
    {
      homeserverUrl,
      email,
      newPassword,
      clientSecret,
      sid,
    },
    invoke
  );
