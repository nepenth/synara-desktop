import to from 'await-to-js';
import { LoginRequest, LoginResponse, MatrixError, createClient } from 'matrix-js-sdk';
import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { ClientConfig, clientAllowedServer } from '../../../hooks/useClientConfig';
import { autoDiscovery, specVersions } from '../../../cs-api';
import { ErrorCode } from '../../../cs-errorcode';
import {
  deleteAfterLoginRedirectPath,
  getAfterLoginRedirectPath,
} from '../../afterLoginRedirectPath';
import { getHomePath } from '../../pathUtils';
import { platformSessionStore } from '../../../platform';
import { persistAuthenticatedSession } from '../../../state/sessionPersistence';
import { pushSessionToSW } from '../../../../sw-session';
import {
  invokeDesktopWithAvailability,
  isSynaraDesktop,
} from '../../../utils/desktop';

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

export type CustomLoginResponse = {
  baseUrl: string;
  response: LoginResponse;
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

export type PasswordLoginResponse = CustomLoginResponse | NativeLoginResponse;

const resolveLoginBaseUrl = async (
  serverBaseUrl: string | (() => Promise<string>)
): Promise<string> => {
  if (typeof serverBaseUrl === 'string') return serverBaseUrl;

  const [urlError, url] = await to(serverBaseUrl());
  if (urlError) {
    throw new MatrixError({
      errcode:
        urlError.message === GetBaseUrlError.NotAllow
          ? LoginError.ServerNotAllowed
          : LoginError.InvalidServer,
    });
  }
  if (!url) {
    throw new MatrixError({ errcode: LoginError.InvalidServer });
  }
  return url;
};

export const login = async (
  serverBaseUrl: string | (() => Promise<string>),
  data: LoginRequest
): Promise<CustomLoginResponse> => {
  const url = await resolveLoginBaseUrl(serverBaseUrl);

  const mx = createClient({ baseUrl: url });
  const [err, res] = await to<LoginResponse, MatrixError>(mx.loginRequest(data));

  if (err) {
    if (err.httpStatus === 400) {
      throw new MatrixError({
        errcode: LoginError.InvalidRequest,
      });
    }
    if (err.httpStatus === 429) {
      throw new MatrixError({
        errcode: LoginError.RateLimited,
      });
    }
    if (err.errcode === ErrorCode.M_USER_DEACTIVATED) {
      throw new MatrixError({
        errcode: LoginError.UserDeactivated,
      });
    }

    if (err.httpStatus === 403) {
      throw new MatrixError({
        errcode: LoginError.Forbidden,
      });
    }

    throw new MatrixError({
      errcode: LoginError.Unknown,
    });
  }
  return {
    baseUrl: url,
    response: res,
  };
};

type NativeCommandError = {
  code?: string;
};

const mapNativeLoginError = (error: unknown): MatrixError => {
  const code =
    typeof error === 'object' && error !== null ? (error as NativeCommandError).code : undefined;
  const errcode = Object.values(LoginError).includes(code as LoginError)
    ? (code as LoginError)
    : LoginError.Unknown;
  return new MatrixError({ errcode });
};

const passwordLoginUser = (data: LoginRequest): string | undefined => {
  const request = data as LoginRequest & {
    password?: string;
    identifier?: {
      user?: string;
      address?: string;
    };
  };
  return request.identifier?.user ?? request.identifier?.address;
};

export const loginPassword = async (
  serverBaseUrl: string | (() => Promise<string>),
  data: LoginRequest
): Promise<PasswordLoginResponse> => {
  if (!isSynaraDesktop()) {
    return login(serverBaseUrl, data);
  }

  const url = await resolveLoginBaseUrl(serverBaseUrl);
  const user = passwordLoginUser(data);
  const password = (data as LoginRequest & { password?: string }).password;
  if (!user || !password) {
    throw new MatrixError({ errcode: LoginError.InvalidRequest });
  }

  try {
    const result = await invokeDesktopWithAvailability<NativeLoginIdentity>(
      'matrix_login_password',
      {
        homeserverUrl: url,
        user,
        password,
      }
    );
    if (!result.available || !result.value) {
      throw new MatrixError({ errcode: LoginError.Unknown });
    }
    return {
      native: true,
      identity: result.value,
    };
  } catch (error) {
    if (error instanceof MatrixError) throw error;
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

export const useLoginComplete = (data?: CustomLoginResponse | NativeLoginResponse) => {
  const navigate = useNavigate();

  useEffect(() => {
    if (!data) return undefined;

    let active = true;
    const persistAndNavigate = async () => {
      if (!('native' in data)) {
        await completeAuthenticatedLogin(data);
      }
      if (!active) return;
      const afterLoginRedirectUrl = getAfterLoginRedirectPath();
      deleteAfterLoginRedirectPath();
      navigate(afterLoginRedirectUrl ?? getHomePath(), { replace: true });
    };

    void persistAndNavigate();

    return () => {
      active = false;
    };
  }, [data, navigate]);
};
