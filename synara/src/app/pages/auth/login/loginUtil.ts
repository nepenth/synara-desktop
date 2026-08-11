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
import { persistAuthenticatedSession } from '../../../state/sessionPersistence';
import { pushSessionToSW } from '../../../../sw-session';
import { recordClientDiagnostic } from '../../../utils/clientDiagnostics';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../../utils/desktop';

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

/** Local session fields used by completeAuthenticatedLogin (no matrix-js-sdk types). */
export type SessionLoginResponse = {
  access_token: string;
  device_id: string;
  user_id: string;
  refresh_token?: string;
  expires_in_ms?: number;
};

export type CustomLoginResponse = {
  baseUrl: string;
  response: SessionLoginResponse;
};

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

const SAFE_NATIVE_DIAGNOSTIC_ID = /^[a-z0-9][a-z0-9.-]{0,127}$/;

const nativeDiagnosticId = (error: unknown): string | undefined => {
  if (!error || typeof error !== 'object') return undefined;
  const candidate = error as NativeCommandError;
  const diagnosticId = candidate.diagnosticId ?? candidate.diagnostic_id;
  return typeof diagnosticId === 'string' && SAFE_NATIVE_DIAGNOSTIC_ID.test(diagnosticId)
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
    options.invoke ?? ((command, args) => invokeDesktopWithAvailability(command, args));

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
    throw mapNativeLoginError(error);
  }
};

export type CompleteAuthenticatedLoginDeps = {
  persistAuthenticatedSession: typeof persistAuthenticatedSession;
  pushSessionToSW: typeof pushSessionToSW;
  nativeSessionStore: typeof platformSessionStore;
};

export const completeAuthenticatedLogin = async (
  data: CustomLoginResponse,
  {
    persistAuthenticatedSession: persistSession,
    pushSessionToSW: pushSession,
    nativeSessionStore,
  }: CompleteAuthenticatedLoginDeps = {
    persistAuthenticatedSession,
    pushSessionToSW,
    nativeSessionStore: platformSessionStore,
  }
) => {
  const { response: loginRes, baseUrl: loginBaseUrl } = data;
  const session = {
    accessToken: loginRes.access_token,
    deviceId: loginRes.device_id,
    userId: loginRes.user_id,
    baseUrl: loginBaseUrl,
    ...(loginRes.refresh_token ? { refreshToken: loginRes.refresh_token } : {}),
    ...(typeof loginRes.expires_in_ms === 'number' ? { expiresInMs: loginRes.expires_in_ms } : {}),
  };

  await persistSession(session, { nativeSessionStore, freshLogin: true });
  pushSession(loginBaseUrl, loginRes.access_token);
};

/**
 * Rehydrate the frontend bootstrap from the host-side desktop session envelope
 * after a native password login / register. The login IPC never returns tokens;
 * route guards and ClientRoot gate on this in-memory bootstrap (`source:
 * 'native'`). Fail-closed: a missing envelope throws (no JS session fallback).
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
      'Native login succeeded but the desktop session envelope is missing.'
    );
  }
  setSessionBootstrapResult({
    session: bootstrap.session,
    source: 'native',
    nativeStoreError: bootstrap.nativeStoreError,
  });
};

export const useLoginComplete = (data?: CustomLoginResponse | NativeLoginResponse) => {
  const navigate = useNavigate();

  useEffect(() => {
    if (!data) return undefined;

    let active = true;
    const persistAndNavigate = async () => {
      // Native password login already persists the session host-side (vault +
      // desktop session envelope). Rehydrate the frontend bootstrap from the
      // envelope so route guards / ClientRoot see the active native session
      // (tokens never return on the login IPC).
      if ('native' in data) {
        await completeNativeLoginBootstrap();
      } else {
        await completeAuthenticatedLogin(data);
      }
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
