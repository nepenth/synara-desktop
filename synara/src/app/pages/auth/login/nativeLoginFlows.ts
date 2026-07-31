/**
 * V-AUTH.3 — native desktop login-flow discovery IPC owner.
 * No matrix-js-sdk. Fail-closed when the native command is unavailable.
 */

import { invokeDesktopWithAvailability } from '../../../utils/desktop';

/** Synara login-flow kinds (match Rust `LoginFlowKind::as_str`). */
export type LoginFlowKind = 'password' | 'token' | 'application_service' | 'unknown' | string;

/** One homeserver login flow (matches `matrix_login_flows` DTO). */
export type LoginFlowDto = {
  kind: LoginFlowKind;
  matrixType: string;
  getLoginToken?: boolean | null;
};

/** Response of `matrix_login_flows`. */
export type LoginFlowsDto = {
  flows: LoginFlowDto[];
};

export type NativeLoginFlowsCommandError = {
  code?: string;
  message?: string;
  diagnosticId?: string;
};

export type NativeLoginFlowsInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<{ available: false } | { available: true; value?: unknown }>;

const defaultInvoke: NativeLoginFlowsInvoke = (command, args) =>
  invokeDesktopWithAvailability(command, args);

export class NativeLoginFlowsError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'NativeLoginFlowsError';
    this.code = code;
  }
}

const mapNativeError = (error: unknown): NativeLoginFlowsError => {
  if (error instanceof NativeLoginFlowsError) return error;
  if (typeof error === 'object' && error !== null) {
    const e = error as NativeLoginFlowsCommandError;
    const code = typeof e.code === 'string' ? e.code : 'Unknown';
    const message =
      typeof e.message === 'string' && e.message.trim()
        ? e.message
        : 'Native login-flow discovery failed.';
    return new NativeLoginFlowsError(code, message);
  }
  if (error instanceof Error) {
    return new NativeLoginFlowsError('Unknown', error.message);
  }
  return new NativeLoginFlowsError('Unknown', 'Native login-flow discovery failed.');
};

const invokeNative = async <T>(
  command: string,
  args: Record<string, unknown>,
  invoke: NativeLoginFlowsInvoke = defaultInvoke
): Promise<T> => {
  try {
    const result = await invoke(command, args);
    if (!result.available || result.value === undefined) {
      throw new NativeLoginFlowsError('Unknown', 'Native login-flow discovery is unavailable.');
    }
    return result.value as T;
  } catch (error) {
    if (error instanceof NativeLoginFlowsError) throw error;
    throw mapNativeError(error);
  }
};

/** Discover login flows for a resolved homeserver base URL (fail-closed). */
export const discoverLoginFlows = (
  homeserverUrl: string,
  invoke: NativeLoginFlowsInvoke = defaultInvoke
): Promise<LoginFlowsDto> => invokeNative('matrix_login_flows', { homeserverUrl }, invoke);

/** Select the password login flow from a discovery result (if advertised). */
export const getPasswordFlow = (loginFlows: LoginFlowDto[]): LoginFlowDto | undefined =>
  loginFlows.find((flow) => flow.kind === 'password' || flow.matrixType === 'm.login.password');
