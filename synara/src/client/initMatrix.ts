import {
  createClient,
  MatrixClient,
  IndexedDBStore,
  IndexedDBCryptoStore,
  MatrixError,
  type IRefreshTokenResponse,
} from 'matrix-js-sdk';
import type { AccessTokens, TokenRefreshFunction } from 'matrix-js-sdk/lib/http-api/interface';

import {
  clearMatrixLocalStores,
  isCryptoAccountMismatchError,
  MATRIX_LEGACY_CRYPTO_STORE_NAME,
  MATRIX_SYNC_STORE_NAME,
} from './matrixLocalStores';
import { clearSecretStorageKeys, cryptoCallbacks } from './secretStorageKeys';
import { clearNavToActivePathStore } from '../app/state/navToActivePath';
import { pushSessionToSW } from '../sw-session';
import {
  clearMatrixStoresForIdentityChange,
  clearPersistedSessions,
  persistAuthenticatedSession,
  setLastBootstrappedMatrixIdentity,
  type SessionPersistenceOptions,
} from '../app/state/sessionPersistence';
import { clearSessionLocalStorage, type Session, type SessionLocalStorage } from '../app/state/sessions';
import { platformSessionStore } from '../app/platform';
import { clearNotificationCaches } from '../app/notifications/notificationCaches';

export const REFRESH_BEFORE_EXPIRY_MS = 60_000;

export type MatrixClientSession = Session;

export type RefreshAndPersistSessionDeps = {
  persistAuthenticatedSession: typeof persistAuthenticatedSession;
  pushSessionToSW: typeof pushSessionToSW;
  nativeSessionStore?: SessionPersistenceOptions['nativeSessionStore'];
};

export const toRefreshedSession = (
  session: MatrixClientSession,
  response: IRefreshTokenResponse
): MatrixClientSession => ({
  baseUrl: session.baseUrl,
  userId: session.userId,
  deviceId: session.deviceId,
  accessToken: response.access_token,
  refreshToken: response.refresh_token,
  expiresInMs: response.expires_in_ms,
});

export const toAccessTokens = (response: IRefreshTokenResponse): AccessTokens => ({
  accessToken: response.access_token,
  refreshToken: response.refresh_token,
  expiry: new Date(Date.now() + response.expires_in_ms),
});

export const refreshAndPersistSession = async (
  mx: MatrixClient,
  session: MatrixClientSession,
  refreshToken: string,
  {
    persistAuthenticatedSession: persistSession,
    pushSessionToSW: pushSession,
    nativeSessionStore,
  }: RefreshAndPersistSessionDeps
): Promise<AccessTokens> => {
  const response = await mx.refreshToken(refreshToken);
  const refreshedSession = toRefreshedSession(session, response);
  await persistSession(refreshedSession, { nativeSessionStore });
  pushSession(refreshedSession.baseUrl, refreshedSession.accessToken);
  return toAccessTokens(response);
};

export const createTokenRefreshFunction = (
  getClient: () => MatrixClient,
  session: MatrixClientSession,
  deps: RefreshAndPersistSessionDeps
): TokenRefreshFunction => {
  return async (refreshToken: string) => {
    try {
      return await refreshAndPersistSession(getClient(), session, refreshToken, deps);
    } catch (error) {
      if (error instanceof MatrixError) {
        throw error;
      }
      throw new MatrixError({
        errcode: 'M_UNKNOWN',
        error: 'Token refresh failed',
      });
    }
  };
};

export type ProactiveTokenRefreshHandle = {
  dispose: () => void;
};

export type CreateMatrixClientOptions = {
  refreshDeps?: RefreshAndPersistSessionDeps;
};

const createMatrixClient = (
  session: MatrixClientSession,
  { refreshDeps }: CreateMatrixClientOptions = {}
): MatrixClient => {
  const indexedDBStore = new IndexedDBStore({
    indexedDB: global.indexedDB,
    localStorage: global.localStorage,
    dbName: MATRIX_SYNC_STORE_NAME,
  });

  const legacyCryptoStore = new IndexedDBCryptoStore(
    global.indexedDB,
    MATRIX_LEGACY_CRYPTO_STORE_NAME
  );

  let mx!: MatrixClient;
  const clientOptions = {
    baseUrl: session.baseUrl,
    accessToken: session.accessToken,
    userId: session.userId,
    store: indexedDBStore,
    cryptoStore: legacyCryptoStore,
    deviceId: session.deviceId,
    timelineSupport: true,
    cryptoCallbacks: cryptoCallbacks as any,
    verificationMethods: ['m.sas.v1'] as const,
  };

  if (session.refreshToken && refreshDeps) {
    Object.assign(clientOptions, {
      refreshToken: session.refreshToken,
      tokenRefreshFunction: createTokenRefreshFunction(() => mx, session, refreshDeps),
    });
  }

  mx = createClient(clientOptions);
  mx.setMaxListeners(50);
  return mx;
};

const defaultRefreshDeps = (): RefreshAndPersistSessionDeps => ({
  persistAuthenticatedSession,
  pushSessionToSW,
  nativeSessionStore: platformSessionStore,
});

const startMatrixClient = async (
  session: MatrixClientSession,
  options: CreateMatrixClientOptions = {}
): Promise<MatrixClient> => {
  const refreshDeps = session.refreshToken
    ? { ...defaultRefreshDeps(), ...options.refreshDeps }
    : undefined;
  const mx = createMatrixClient(session, { refreshDeps });
  await mx.store.startup();
  await mx.initRustCrypto();
  return mx;
};

export type InitClientDeps = {
  clearMatrixStoresForIdentityChange?: typeof clearMatrixStoresForIdentityChange;
  clearMatrixLocalStores?: typeof clearMatrixLocalStores;
  setLastBootstrappedMatrixIdentity?: typeof setLastBootstrappedMatrixIdentity;
  startMatrixClient?: typeof startMatrixClient;
};

const recordBootstrappedMatrixIdentity = (
  session: MatrixClientSession,
  setLastBootstrapped: InitClientDeps['setLastBootstrappedMatrixIdentity'] = setLastBootstrappedMatrixIdentity
): void => {
  setLastBootstrapped?.({
    userId: session.userId,
    deviceId: session.deviceId,
  });
};

export const initClient = async (
  session: MatrixClientSession,
  {
    clearMatrixStoresForIdentityChange: clearStoresForIdentityChange = clearMatrixStoresForIdentityChange,
    clearMatrixLocalStores: clearStores = clearMatrixLocalStores,
    setLastBootstrappedMatrixIdentity: setLastBootstrapped = setLastBootstrappedMatrixIdentity,
    startMatrixClient: startClient = startMatrixClient,
  }: InitClientDeps = {}
): Promise<MatrixClient> => {
  const identityCleared = await clearStoresForIdentityChange(session);
  if (identityCleared) {
    clearNotificationCaches();
  }

  try {
    const client = await startClient(session);
    recordBootstrappedMatrixIdentity(session, setLastBootstrapped);
    return client;
  } catch (error) {
    if (!isCryptoAccountMismatchError(error)) {
      throw error;
    }

    await clearStores();
    const client = await startClient(session);
    recordBootstrappedMatrixIdentity(session, setLastBootstrapped);
    return client;
  }
};

export const startClient = async (mx: MatrixClient) => {
  await mx.startClient({
    lazyLoadMembers: true,
  });
};

export const clearCacheAndReload = async (mx: MatrixClient) => {
  mx.stopClient();
  clearNavToActivePathStore(mx.getSafeUserId());
  clearNotificationCaches();
  await mx.store.deleteAllData();
  window.location.reload();
};

export type PerformLogoutDeps = {
  clearPersistedSessions: (options?: SessionPersistenceOptions) => Promise<void>;
  pushSessionToSW: typeof pushSessionToSW;
  clearSessionLocalStorage: typeof clearSessionLocalStorage;
  nativeSessionStore: SessionPersistenceOptions['nativeSessionStore'];
  reload: () => void;
};

const defaultPerformLogoutDeps = (): PerformLogoutDeps => ({
  clearPersistedSessions,
  pushSessionToSW,
  clearSessionLocalStorage,
  nativeSessionStore: platformSessionStore,
  reload: () => window.location.reload(),
});

export const performLogout = async (
  mx?: MatrixClient,
  {
    storage,
    ...depsOverrides
  }: Partial<PerformLogoutDeps> & { storage?: SessionLocalStorage } = {}
): Promise<void> => {
  const deps = { ...defaultPerformLogoutDeps(), ...depsOverrides };

  await deps.clearPersistedSessions({ nativeSessionStore: deps.nativeSessionStore });
  deps.pushSessionToSW();

  if (mx) {
    mx.stopClient();
    try {
      await mx.logout();
    } catch {
      // ignore if failed to logout
    }
    await mx.clearStores();
  }

  deps.clearSessionLocalStorage(storage);
  clearSecretStorageKeys();
  clearNotificationCaches();
  deps.reload();
};

export const logoutClient = async (mx: MatrixClient) => performLogout(mx);

export const clearLoginData = async (
  storage = typeof window === 'undefined' ? undefined : window.localStorage
) => performLogout(undefined, { storage: storage as SessionLocalStorage });

export const scheduleProactiveTokenRefresh = (
  mx: MatrixClient,
  session: MatrixClientSession,
  deps: Partial<RefreshAndPersistSessionDeps> = {},
  nowMs = Date.now()
): ProactiveTokenRefreshHandle => {
  const resolvedDeps = { ...defaultRefreshDeps(), ...deps };

  if (!session.refreshToken || typeof session.expiresInMs !== 'number' || typeof session.storedAtMs !== 'number') {
    return { dispose: () => undefined };
  }

  const refreshAtMs = session.storedAtMs + session.expiresInMs - REFRESH_BEFORE_EXPIRY_MS;
  const delayMs = Math.max(0, refreshAtMs - nowMs);
  let disposed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const runRefresh = async () => {
    if (disposed) return;

    try {
      await refreshAndPersistSession(mx, session, session.refreshToken!, resolvedDeps);
    } catch {
      await performLogout(mx, { nativeSessionStore: resolvedDeps.nativeSessionStore });
    }
  };

  timer = setTimeout(() => {
    void runRefresh();
  }, delayMs);

  return {
    dispose: () => {
      disposed = true;
      if (timer !== undefined) {
        clearTimeout(timer);
      }
    },
  };
};
