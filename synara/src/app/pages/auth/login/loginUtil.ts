/**
 * Desktop password login owner.
 *
 * Product path is native-only via `matrix_login_password` (fail-closed).
 * No matrix-js-sdk client construction and no JS password-login fallback.
 */

import to from 'await-to-js';
import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { ClientConfig, clientAllowedServer } from '../../../hooks/useClientConfig';
import { autoDiscovery, specVersions } from '../../../cs-api';
import {
  deleteAfterLoginRedirectPath,
  getAfterLoginRedirectPath,
} from '../../afterLoginRedirectPath';
import { getHomePath } from '../../pathUtils';
import { platformSessionStore } from '../../../platform';
import {
  clearSessionBootstrap,
  resolveSessionBootstrap,
  setSessionBootstrapResult,
  type AsyncSessionStore,
} from '../../../state/sessionBootstrap';
import { recordClientDiagnostic } from '../../../utils/clientDiagnostics';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';
import { recordDesktopDiagnostic } from '../../../utils/desktopDiagnostics';

export enum GetBaseUrlError {
  NotAllow = 'NotAllow',
  NotFound = 'NotFound',
}
export const factoryGetBaseUrl = (clientConfig: ClientConfig, server: string) => {
  const getBaseUrl = async (): Promise<string> => {
    if (!clientAllowedServer(clientConfig, server)) {
      throw new Error(GetBaseUrlError.NotAllow);
    }

    const [, discovery] = await to(autoDiscovery(fetch, server));

    let mxIdBaseUrl: string | undefined;
    const [, discoveryInfo] = discovery ?? [];

    if (discoveryInfo) {
      mxIdBaseUrl = discoveryInfo['m.homeserver'].base_url;
    }

    if (!mxIdBaseUrl) {
      throw new Error(GetBaseUrlError.NotFound);
    }
    const [, versions] = await to(specVersions(fetch, mxIdBaseUrl));
    if (!versions) {
      throw new Error(GetBaseUrlError.NotFound);
    }
    return mxIdBaseUrl;
  };
  return getBaseUrl;
};

export enum LoginError {
  ServerNotAllowed = 'ServerNotAllowed',
  InvalidServer = 'InvalidServer',
  Forbidden = 'Forbidden',
  UserDeactivated = 'UserDeactivated',
  InvalidRequest = 'InvalidRequest',
  RateLimited = 'RateLimited',
  Unknown = 'Unknown',
}

/** SDK-neutral login error with product errcode (replaces matrix-js-sdk MatrixError). */
export class PasswordLoginError extends Error {
  readonly errcode: LoginError;
  /** Static native diagnostic only; never a server body, URL, password, or token. */
  readonly diagnosticId?: string;

  constructor(errcode: LoginError, message?: string, diagnosticId?: string) {
    super(message ?? errcode);
    this.name = 'PasswordLoginError';
    this.errcode = errcode;
    this.diagnosticId = diagnosticId;
  }
}

export type NativeLoginIdentity = {
  userId: string;
  deviceId: string;
  homeserverUrl: string;
};

export type NativeLoginResponse = {
  native: true;
  identity: NativeLoginIdentity;
};

/** Desktop password login always returns a native identity outcome. */
export type PasswordLoginResponse = NativeLoginResponse;

/** Minimal password login request shape (UI → native IPC). */
export type PasswordLoginRequest = {
  type?: string;
  password?: string;
  identifier?: {
    type?: string;
    user?: string;
    address?: string;
    medium?: string;
  };
  initial_device_display_name?: string;
};

export type PasswordLoginInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<{ available: false } | { available: true; value?: unknown }>;

export type LoginPasswordOptions = {
  isDesktop?: () => boolean;
  invoke?: PasswordLoginInvoke;
};

const resolveLoginBaseUrl = async (
  serverBaseUrl: string | (() => Promise<string>)
): Promise<string> => {
  if (typeof serverBaseUrl === 'string') return serverBaseUrl;

  const [urlError, url] = await to(serverBaseUrl());
  if (urlError) {
    throw new PasswordLoginError(
      urlError.message === GetBaseUrlError.NotAllow
        ? LoginError.ServerNotAllowed
        : LoginError.InvalidServer
    );
  }
  if (!url) {
    throw new PasswordLoginError(LoginError.InvalidServer);
  }
  return url;
};

type NativeCommandError = {
  code?: string;
  diagnosticId?: string;
  diagnostic_id?: string;
};

// Closed allowlist: native login diagnostics are static identifiers defined in
// `src-tauri/src/matrix/auth/login.rs`. Do not accept arbitrary native text here:
// this is the renderer's final privacy boundary before an identifier is displayed.
const SAFE_NATIVE_LOGIN_DIAGNOSTIC_IDS = new Set([
  'p3.2-device-display-name-too-long',
  'p3.2-device-id-invalid-chars',
  'p3.2-device-id-too-long',
  'p3.2-empty-device-display-name',
  'p3.2-empty-device-id',
  'p3.2-empty-password',
  'p3.2-empty-user-id',
  'p3.2-login-connectivity',
  // Legacy umbrella id for pre-fix builds; refined ids add store-locked / store-open-failed / olm-unavailable
  'p3.2-login-crypto-store',
  'p3.2-login-endpoint-not-found',
  'p3.2-login-homeserver-unavailable',
  'p3.2-login-http-api-response',
  'p3.2-login-http-cached',
  'p3.2-login-http-connect',
  'p3.2-login-http-request',
  'p3.2-login-http-request-build',
  'p3.2-login-http-timeout',
  'p3.2-login-http-verifier',
  'p3.2-login-local-io',
  'p3.2-login-rate-limited',
  'p3.2-login-refresh-token',
  'p3.2-login-rejected',
  'p3.2-login-response-decode',
  'p3.2-login-sdk-timeout',
  'p3.2-login-store-locked',
  'p3.2-login-store-migration-failed',
  'p3.2-login-store-migration-required',
  'p3.2-login-store-open-failed',
  'p3.2-login-store-reset-required',
  'p3.2-login-olm-unavailable',
  'p3.2-login-uiaa-required',
  'p3.2-login-unknown',
  'p3.2-login-unknown-rejected',
  'p3.2-login-unknown-token',
  'p3.2-login-unrecognized',
  'p3.2-login-url-parse',
  'p3.2-login-user-deactivated',
  'p3.2-user-id-invalid-chars',
  'p3.2-user-id-too-long',
]);

const nativeDiagnosticId = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object') return undefined;
  const candidate = error as NativeCommandError;
  const diagnosticId = candidate.diagnosticId ?? candidate.diagnostic_id;
  return typeof diagnosticId === 'string' && SAFE_NATIVE_LOGIN_DIAGNOSTIC_IDS.has(diagnosticId)
    ? diagnosticId
    : undefined;
};

const mapNativeLoginError = (error: unknown): PasswordLoginError => {
  const code =
    typeof error === 'object' && error !== null ? (error as NativeCommandError).code : undefined;
  const errcode = Object.values(LoginError).includes(code as LoginError)
    ? (code as LoginError)
    : LoginError.Unknown;
  return new PasswordLoginError(errcode, undefined, nativeDiagnosticId(error));
};

// Only a native login failure can reveal the archive-and-rebuild affordance.
// This stays separate from generic store errors: locked/unavailable storage is
// not automatically treated as data corruption or a reset candidate.
const RECOVERABLE_NATIVE_STORE_DIAGNOSTIC_IDS = new Set([
  'p3.2-login-store-migration-required',
  'p3.2-login-store-reset-required',
]);

export const canOfferNativeStoreRecovery = (diagnosticId: string | undefined): boolean =>
  diagnosticId !== undefined && RECOVERABLE_NATIVE_STORE_DIAGNOSTIC_IDS.has(diagnosticId);

/** The visible acknowledgement required before the one-use native confirmation is requested. */
export const STORE_RECOVERY_CONFIRMATION_TEXT = 'ARCHIVE';

/** Static-only error for the archive-and-rebuild IPC flow. */
export class StoreRecoveryError extends Error {
  readonly diagnosticId: string;

  constructor(diagnosticId: string) {
    super('Local Matrix store recovery could not be completed.');
    this.name = 'StoreRecoveryError';
    this.diagnosticId = diagnosticId;
  }
}

const SAFE_NATIVE_STORE_RECOVERY_DIAGNOSTIC_IDS = new Set([
  'd0.1-session-already-active',
  'p3.2-login-store-recovery-confirmation-required',
  'p3.2-login-store-recovery-confirmation-unavailable',
  'p3.2-login-store-recovery-failed',
  'p3.2-login-store-recovery-not-pending',
  'p3.2-login-store-recovery-unavailable',
]);

const nativeStoreRecoveryDiagnosticId = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object') return undefined;
  const candidate = error as NativeCommandError;
  const diagnosticId = candidate.diagnosticId ?? candidate.diagnostic_id;
  return typeof diagnosticId === 'string' &&
    SAFE_NATIVE_STORE_RECOVERY_DIAGNOSTIC_IDS.has(diagnosticId)
    ? diagnosticId
    : undefined;
};

const asStoreRecoveryConfirmationId = (value: unknown): string | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const confirmationId = (value as { confirmationId?: unknown }).confirmationId;
  return typeof confirmationId === 'string' && /^[a-f0-9]{64}$/.test(confirmationId)
    ? confirmationId
    : undefined;
};

const isStoreRecoverySuccess = (value: unknown): boolean =>
  !!value &&
  typeof value === 'object' &&
  (value as { status?: unknown }).status === 'archived_and_rebuilt';

export type StoreRecoveryOptions = {
  invoke?: PasswordLoginInvoke;
};

/**
 * Complete only the already-user-confirmed archive-and-rebuild action.
 *
 * `confirmationText` is the exact text typed in the recovery dialog. The
 * helper refuses it before even preparing a native capability unless it is the
 * visible acknowledgement; the native host validates the same value again.
 * The host accepts no account identity or credential here. It first issues an
 * opaque CSPRNG one-use confirmation identifier bound to the failed login
 * target, then consumes it immediately. Neither it nor raw IPC errors are
 * rendered or logged.
 */
export const archiveAndRebuildNativeStore = async (
  confirmationText: string,
  options: StoreRecoveryOptions = {}
): Promise<void> => {
  const invoke: PasswordLoginInvoke =
    options.invoke ??
    ((command, args) =>
      invokeDesktopWithAvailability(command, args, { suppressErrorDiagnostic: true }));

  try {
    if (confirmationText !== STORE_RECOVERY_CONFIRMATION_TEXT) {
      throw new StoreRecoveryError('p3.2-login-store-recovery-confirmation-required');
    }
    const prepared = await invoke('matrix_store_recovery_prepare');
    const confirmationId = prepared.available
      ? asStoreRecoveryConfirmationId(prepared.value)
      : undefined;
    if (!confirmationId) {
      throw new StoreRecoveryError('p3.2-login-store-recovery-unavailable');
    }
    const confirmed = await invoke('matrix_store_recovery_confirm', {
      confirmationId,
      confirmationText,
    });
    if (!confirmed.available || !isStoreRecoverySuccess(confirmed.value)) {
      throw new StoreRecoveryError('p3.2-login-store-recovery-unavailable');
    }
  } catch (error) {
    const mapped =
      error instanceof StoreRecoveryError
        ? error
        : new StoreRecoveryError(
            nativeStoreRecoveryDiagnosticId(error) ?? 'p3.2-login-store-recovery-failed'
          );
    // A fixed allowlisted identifier is the only recovery detail written to
    // desktop diagnostics; confirmation IDs, identities, paths, and native
    // errors never reach logs.
    recordDesktopDiagnostic(`matrix_store_recovery failed: ${mapped.diagnosticId}`);
    throw mapped;
  }
};

const passwordLoginUser = (data: PasswordLoginRequest): string | undefined =>
  data.identifier?.user ?? data.identifier?.address;

/**
 * Desktop product password login — native `matrix_login_password` only.
 * Fail-closed when not on Synara desktop or when the native command is unavailable.
 * No matrix-js-sdk password-login path.
 */
export const loginPassword = async (
  serverBaseUrl: string | (() => Promise<string>),
  data: PasswordLoginRequest,
  options: LoginPasswordOptions = {}
): Promise<PasswordLoginResponse> => {
  const isDesktop = options.isDesktop ?? isSynaraDesktop;
  const invoke: PasswordLoginInvoke =
    options.invoke ??
    ((command, args) =>
      invokeDesktopWithAvailability(command, args, { suppressErrorDiagnostic: true }));

  if (!isDesktop()) {
    throw new PasswordLoginError(
      LoginError.Unknown,
      'Password login requires the native desktop Matrix runtime.'
    );
  }

  const url = await resolveLoginBaseUrl(serverBaseUrl);
  const user = passwordLoginUser(data);
  const password = data.password;
  if (!user || !password) {
    throw new PasswordLoginError(LoginError.InvalidRequest);
  }

  try {
    const result = await invoke('matrix_login_password', {
      homeserverUrl: url,
      user,
      password,
    });
    if (!result.available || !result.value) {
      throw new PasswordLoginError(LoginError.Unknown, 'Native password login is unavailable.');
    }
    return {
      native: true,
      identity: result.value as NativeLoginIdentity,
    };
  } catch (error) {
    if (error instanceof PasswordLoginError) throw error;
    const mappedError = mapNativeLoginError(error);
    // This is deliberately a static, allowlisted code rather than the native
    // rejection: login errors can carry server-controlled text or credentials.
    recordDesktopDiagnostic(
      `matrix_login_password failed: ${mappedError.diagnosticId ?? 'p3.2-login-unknown'}`
    );
    throw mappedError;
  }
};

/**
 * Rehydrate the frontend bootstrap from host-owned identity metadata after a
 * native password login or registration. Credentials never cross IPC.
 */
export const completeNativeLoginBootstrap = async (
  nativeSessionStore: AsyncSessionStore = platformSessionStore
): Promise<void> => {
  clearSessionBootstrap();
  const bootstrap = await resolveSessionBootstrap({ nativeSessionStore });
  if (!bootstrap.session) {
    recordClientDiagnostic('session', 'bootstrap.completed', {
      source: 'none',
      outcome: 'native-login-missing-desktop-session',
    });
    throw new PasswordLoginError(
      LoginError.Unknown,
      'Native login succeeded but the desktop session identity is missing.'
    );
  }
  setSessionBootstrapResult({
    session: bootstrap.session,
    source: 'native',
    nativeStoreError: bootstrap.nativeStoreError,
  });
};

export const useLoginComplete = (data?: NativeLoginResponse) => {
  const navigate = useNavigate();

  useEffect(() => {
    if (!data) return undefined;

    let active = true;
    const persistAndNavigate = async () => {
      // Native password login persists credentials host-side. Rehydrate only
      // identity metadata so route guards can enter the client.
      await completeNativeLoginBootstrap();
      if (!active) return;
      const afterLoginRedirectUrl = getAfterLoginRedirectPath();
      deleteAfterLoginRedirectPath();
      navigate(afterLoginRedirectUrl ?? getHomePath(), { replace: true });
    };

    void persistAndNavigate().catch((error) => {
      recordClientDiagnostic('session', 'bootstrap.failed', {
        errorType: error instanceof Error ? error.name : typeof error,
        outcome: 'native-login-handoff-failed',
      });
    });

    return () => {
      active = false;
    };
  }, [data, navigate]);
};
