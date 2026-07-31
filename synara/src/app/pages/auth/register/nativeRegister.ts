/**
 * V-AUTH.4b — native desktop registration IPC owner.
 * No matrix-js-sdk. Secrets are never logged here.
 */

import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export const SUPPORTED_REGISTER_STAGES = [
  'm.login.registration_token',
  'm.login.terms',
  'm.login.recaptcha',
  'm.login.email.identity',
  'm.login.dummy',
] as const;

export type NativeRegisterUiaFlow = {
  stages: string[];
};

export type NativeRegisterFlowsProbe =
  | {
      status: 'flow_required';
      session?: string | null;
      flows: NativeRegisterUiaFlow[];
      completed?: string[];
      params?: Record<string, Record<string, unknown>> | null;
    }
  | { status: 'registration_disabled' }
  | { status: 'rate_limited' }
  | { status: 'invalid_request' };

export type NativeRegisterIdentity = {
  userId: string;
  deviceId: string;
  homeserverUrl: string;
};

export type NativeRegisterOutcome =
  | { status: 'complete'; identity: NativeRegisterIdentity }
  | {
      status: 'uia_required';
      session?: string | null;
      flows: NativeRegisterUiaFlow[];
      completed?: string[];
      params?: Record<string, Record<string, unknown>> | null;
      errorCode?: string | null;
      errorMessage?: string | null;
    };

export type NativeRegisterEmailTokenResult = {
  sid: string;
  submitUrl?: string | null;
};

export type NativeRegisterAuthStage =
  | { type: 'session_only'; session?: string | null }
  | { type: 'dummy'; session?: string | null }
  | { type: 'terms'; session?: string | null }
  | { type: 'registration_token'; token: string; session?: string | null }
  | { type: 'recaptcha'; response: string; session?: string | null }
  | {
      type: 'email_identity';
      sid: string;
      clientSecret: string;
      session?: string | null;
    };

export type NativeRegisterCommandError = {
  code?: string;
  message?: string;
  diagnosticId?: string;
};

export type NativeRegisterInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<{ available: false } | { available: true; value?: unknown }>;

const defaultInvoke: NativeRegisterInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);

export enum RegisterErrorCode {
  UserTaken = 'UserTaken',
  UserInvalid = 'UserInvalid',
  UserExclusive = 'UserExclusive',
  PasswordWeak = 'PasswordWeak',
  PasswordShort = 'PasswordShort',
  InvalidRequest = 'InvalidRequest',
  Forbidden = 'Forbidden',
  RateLimited = 'RateLimited',
  Unsupported = 'Unsupported',
  Unknown = 'Unknown',
}

export class NativeRegisterError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'NativeRegisterError';
    this.code = code;
  }
}

const mapNativeError = (error: unknown): NativeRegisterError => {
  if (error instanceof NativeRegisterError) return error;
  if (typeof error === 'object' && error !== null) {
    const e = error as NativeRegisterCommandError;
    const code = typeof e.code === 'string' ? e.code : RegisterErrorCode.Unknown;
    const message =
      typeof e.message === 'string' && e.message.trim()
        ? e.message
        : 'Native registration failed.';
    return new NativeRegisterError(code, message);
  }
  if (error instanceof Error) {
    return new NativeRegisterError(RegisterErrorCode.Unknown, error.message);
  }
  return new NativeRegisterError(RegisterErrorCode.Unknown, 'Native registration failed.');
};

const invokeNative = async <T>(
  command: string,
  args: Record<string, unknown>,
  invoke: NativeRegisterInvoke = defaultInvoke
): Promise<T> => {
  try {
    const result = await invoke(command, args);
    if (!result.available || result.value === undefined) {
      throw new NativeRegisterError(
        RegisterErrorCode.Unknown,
        'Native registration is unavailable.'
      );
    }
    return result.value as T;
  } catch (error) {
    if (error instanceof NativeRegisterError) throw error;
    throw mapNativeError(error);
  }
};

/** Generate a Matrix client secret (UUID without hyphens). */
export const generateRegisterClientSecret = (): string => {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID().replace(/-/g, '');
  }
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

export const probeRegisterFlows = (
  homeserverUrl: string,
  invoke: NativeRegisterInvoke = defaultInvoke
): Promise<NativeRegisterFlowsProbe> =>
  invokeNative('matrix_register_flows', { homeserverUrl }, invoke);

export const requestRegisterEmailToken = (
  homeserverUrl: string,
  email: string,
  clientSecret: string,
  sendAttempt: number,
  invoke: NativeRegisterInvoke = defaultInvoke
): Promise<NativeRegisterEmailTokenResult> =>
  invokeNative(
    'matrix_register_request_email_token',
    {
      homeserverUrl,
      email,
      clientSecret,
      sendAttempt,
    },
    invoke
  );

export const submitRegister = (
  homeserverUrl: string,
  username: string,
  password: string,
  auth: NativeRegisterAuthStage,
  deviceDisplayName?: string,
  invoke: NativeRegisterInvoke = defaultInvoke
): Promise<NativeRegisterOutcome> =>
  invokeNative(
    'matrix_register',
    {
      homeserverUrl,
      username,
      password,
      deviceDisplayName: deviceDisplayName ?? null,
      auth,
    },
    invoke
  );

export type UiaAuthData = {
  session?: string;
  flows?: NativeRegisterUiaFlow[];
  completed?: string[];
  params?: Record<string, Record<string, unknown>>;
  errcode?: string;
  error?: string;
};

export const uiaAuthDataFromChallenge = (outcome: {
  session?: string | null;
  flows?: NativeRegisterUiaFlow[];
  completed?: string[];
  params?: Record<string, Record<string, unknown>> | null;
  errorCode?: string | null;
  errorMessage?: string | null;
}): UiaAuthData => ({
  session: outcome.session ?? undefined,
  flows: outcome.flows ?? [],
  completed: outcome.completed ?? [],
  params: outcome.params ?? undefined,
  errcode: outcome.errorCode ?? undefined,
  error: outcome.errorMessage ?? undefined,
});

export const uiaAuthDataFromProbe = (
  probe: Extract<NativeRegisterFlowsProbe, { status: 'flow_required' }>
): UiaAuthData => ({
  session: probe.session ?? undefined,
  flows: probe.flows ?? [],
  completed: probe.completed ?? [],
  params: probe.params ?? undefined,
});
