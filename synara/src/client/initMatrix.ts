import {
  createClient,
  ClientEvent,
  MatrixClient,
  IndexedDBStore,
  IndexedDBCryptoStore,
  SyncState,
  type ICreateClientOpts,
  type IRefreshTokenResponse,
} from 'matrix-js-sdk';
import type { AccessTokens, TokenRefreshFunction } from 'matrix-js-sdk/lib/http-api/interface';

import {
  isCryptoAccountMismatchError,
  MATRIX_LEGACY_CRYPTO_STORE_NAME,
  MATRIX_SYNC_STORE_NAME,
} from './matrixLocalStores';
import { clearNavToActivePathStore } from '../app/state/navToActivePath';
import { pushSessionToSW } from '../sw-session';
import {
  clearMatrixStoresForIdentityChange,
  clearPendingFreshLoginIdentity,
  clearPersistedSessions,
  isPendingFreshLoginIdentity,
  persistAuthenticatedSession,
  setLastBootstrappedMatrixIdentity,
  type SessionPersistenceOptions,
} from '../app/state/sessionPersistence';
import {
  clearSessionLocalStorage,
  type Session,
  type SessionLocalStorage,
} from '../app/state/sessions';
import { platformSessionStore } from '../app/platform';
import { clearNotificationCaches } from '../app/notifications/notificationCaches';
import { assertCryptoStoreContinuity, CryptoStoreContinuityError } from './cryptoStoreContinuity';
import { recordClientDiagnostic } from '../app/utils/clientDiagnostics';
import { getSessionBootstrapResult } from '../app/state/sessionBootstrap';
import { isSynaraDesktop } from '../app/utils/desktop';

/** Duck-typed js-sdk MatrixError: an Error carrying a string `errcode`. */
export const isMatrixErrorLike = (error: unknown): error is Error & { errcode?: string } =>
  error instanceof Error && typeof (error as { errcode?: unknown } | null)?.errcode === 'string';

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
  ...(session.sessionGeneration ? { sessionGeneration: session.sessionGeneration } : {}),
  refreshToken: response.refresh_token,
  expiresInMs: response.expires_in_ms,
});

export const toAccessTokens = (response: IRefreshTokenResponse): AccessTokens => ({
  accessToken: response.access_token,
  refreshToken: response.refresh_token,
  expiry: new Date(Date.now() + response.expires_in_ms),
});

type MatrixClientWithRefreshToken = MatrixClient & {
  http: {
    opts: {
      refreshToken?: string;
    };
  };
};

export const applyRefreshedCredentialsToClient = (
  mx: MatrixClient,
  response: IRefreshTokenResponse
): void => {
  mx.setAccessToken(response.access_token);
  (mx as MatrixClientWithRefreshToken).http.opts.refreshToken = response.refresh_token;
};

export type RefreshAndPersistResult = {
  tokens: AccessTokens;
  session: MatrixClientSession;
};

export const refreshAndPersistSession = async (
  mx: MatrixClient,
  session: MatrixClientSession,
  refreshToken: string,
  {
    persistAuthenticatedSession: persistSession,
    pushSessionToSW: pushSession,
    nativeSessionStore,
  }: RefreshAndPersistSessionDeps
): Promise<RefreshAndPersistResult> => {
  const startedAtMs = performance.now();
  let refreshPhase = 'request';
  recordClientDiagnostic('session', 'token-refresh.started');
  try {
    const response = await mx.refreshToken(refreshToken);
    const refreshedAtMs = performance.now();
    refreshPhase = 'apply-credentials';
    applyRefreshedCredentialsToClient(mx, response);
    const refreshedSession: MatrixClientSession = {
      ...toRefreshedSession(session, response),
      storedAtMs: Date.now(),
    };
    refreshPhase = 'persist';
    await persistSession(refreshedSession, { nativeSessionStore });
    refreshPhase = 'service-worker';
    pushSession(refreshedSession.baseUrl, refreshedSession.accessToken);
    recordClientDiagnostic('session', 'token-refresh.completed', {
      outcome: 'persisted',
      requestDurationMs: refreshedAtMs - startedAtMs,
      durationMs: performance.now() - startedAtMs,
    });
    return {
      tokens: toAccessTokens(response),
      session: refreshedSession,
    };
  } catch (error) {
    recordClientDiagnostic('session', 'token-refresh.completed', {
      outcome: 'error',
      phase: refreshPhase,
      durationMs: performance.now() - startedAtMs,
      errorType: error instanceof Error ? error.name : typeof error,
    });
    throw error;
  }
};

export const createTokenRefreshFunction = (
  getClient: () => MatrixClient,
  session: MatrixClientSession,
  deps: RefreshAndPersistSessionDeps
): TokenRefreshFunction => {
  return async (refreshToken: string) => {
    try {
      const { tokens } = await refreshAndPersistSession(getClient(), session, refreshToken, deps);
      return tokens;
    } catch (error) {
      if (isMatrixErrorLike(error)) {
        throw error;
      }
      throw Object.assign(new Error('Token refresh failed'), { errcode: 'M_UNKNOWN' });
    }
  };
};

export type ProactiveTokenRefreshHandle = {
  dispose: () => void;
};

export type CreateMatrixClientOptions = {
  refreshDeps?: RefreshAndPersistSessionDeps;
  allowMissingServerDevice?: boolean;
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

  const mxHolder: { client?: MatrixClient } = {};
  const clientOptions: ICreateClientOpts = {
    baseUrl: session.baseUrl,
    accessToken: session.accessToken,
    userId: session.userId,
    store: indexedDBStore,
    cryptoStore: legacyCryptoStore,
    deviceId: session.deviceId,
    timelineSupport: true,
    verificationMethods: ['m.sas.v1'],
  };

  if (session.refreshToken && refreshDeps) {
    Object.assign(clientOptions, {
      refreshToken: session.refreshToken,
      tokenRefreshFunction: createTokenRefreshFunction(
        () => mxHolder.client!,
        session,
        refreshDeps
      ),
    });
  }

  const mx = createClient(clientOptions);
  mxHolder.client = mx;
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
  const startupStartedAtMs = performance.now();
  let initializationPhase = 'store-startup';
  const refreshDeps = session.refreshToken
    ? { ...defaultRefreshDeps(), ...options.refreshDeps }
    : undefined;
  const nativeCryptoOwner = isSynaraDesktop() && getSessionBootstrapResult().source === 'native';
  const mx = createMatrixClient(session, { refreshDeps });
  recordClientDiagnostic('session', 'matrix-client.initialization-started', {
    hasRefreshToken: Boolean(session.refreshToken),
    fallbackSdkStores: Boolean(session.fallbackSdkStores),
  });
  try {
    const storeStartedAtMs = performance.now();
    await mx.store.startup();
    recordClientDiagnostic('session', 'matrix-store.startup-completed', {
      durationMs: performance.now() - storeStartedAtMs,
    });
    if (nativeCryptoOwner) {
      recordClientDiagnostic('session', 'matrix-crypto.initialization-skipped', {
        owner: 'matrix-rust-sdk',
      });
    } else {
      initializationPhase = 'crypto-initialization';
      const cryptoStartedAtMs = performance.now();
      await mx.initRustCrypto();
      recordClientDiagnostic('session', 'matrix-crypto.initialization-completed', {
        durationMs: performance.now() - cryptoStartedAtMs,
      });
      initializationPhase = 'crypto-continuity';
      const continuityStartedAtMs = performance.now();
      const continuity = await assertCryptoStoreContinuity(mx, {
        userId: session.userId,
        deviceId: session.deviceId,
        allowMissingServerDevice: options.allowMissingServerDevice,
      });
      recordClientDiagnostic('session', 'crypto-continuity.completed', {
        outcome: continuity,
        durationMs: performance.now() - continuityStartedAtMs,
      });
      initializationPhase = 'continuity-finalization';
      if (continuity === 'matched') {
        clearPendingFreshLoginIdentity(session);
      } else {
        pendingFreshLoginContinuity.set(mx, session);
      }
    }
    recordClientDiagnostic('session', 'matrix-client.initialization-completed', {
      outcome: 'ready-to-start',
      durationMs: performance.now() - startupStartedAtMs,
    });
    return mx;
  } catch (error) {
    // Closes the Rust OlmMachine/IndexedDB handle without deleting any data.
    // matrix-js-sdk exposes only IndexedDBStore.destroy(), which deletes the
    // sync cache; it has no supported non-destructive close API. Do not reach
    // into its private backend or delete either crypto store on a safety error.
    mx.stopClient();
    recordClientDiagnostic('session', 'matrix-client.initialization-completed', {
      outcome: 'error',
      phase: initializationPhase,
      durationMs: performance.now() - startupStartedAtMs,
      errorType: error instanceof Error ? error.name : typeof error,
    });
    throw error;
  }
};

export type InitClientDeps = {
  clearMatrixStoresForIdentityChange?: typeof clearMatrixStoresForIdentityChange;
  isPendingFreshLoginIdentity?: typeof isPendingFreshLoginIdentity;
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
    clearMatrixStoresForIdentityChange:
      clearStoresForIdentityChange = clearMatrixStoresForIdentityChange,
    isPendingFreshLoginIdentity: isFreshLoginIdentity = isPendingFreshLoginIdentity,
    setLastBootstrappedMatrixIdentity: setLastBootstrapped = setLastBootstrappedMatrixIdentity,
    startMatrixClient: startClient = startMatrixClient,
  }: InitClientDeps = {}
): Promise<MatrixClient> => {
  const initStartedAtMs = performance.now();
  const freshLogin = isFreshLoginIdentity(session);
  const identityCleared = freshLogin ? await clearStoresForIdentityChange(session) : false;
  recordClientDiagnostic('session', 'matrix-client.bootstrap-decision', {
    freshLogin,
    identityStoresCleared: identityCleared,
  });
  if (identityCleared) {
    clearNotificationCaches();
  }

  try {
    const client = await startClient(session, { allowMissingServerDevice: freshLogin });
    recordBootstrappedMatrixIdentity(session, setLastBootstrapped);
    recordClientDiagnostic('session', 'matrix-client.bootstrap-completed', {
      outcome: 'initialized',
      durationMs: performance.now() - initStartedAtMs,
    });
    return client;
  } catch (error) {
    recordClientDiagnostic('session', 'matrix-client.bootstrap-completed', {
      outcome: 'error',
      durationMs: performance.now() - initStartedAtMs,
      errorType: error instanceof Error ? error.name : typeof error,
    });
    if (isCryptoAccountMismatchError(error)) {
      throw new CryptoStoreContinuityError(
        session.userId,
        session.deviceId,
        'identity-key-mismatch'
      );
    }
    throw error;
  }
};

const pendingFreshLoginContinuity = new WeakMap<MatrixClient, MatrixClientSession>();

export const POST_START_CRYPTO_CONTINUITY_TIMEOUT_MS = 30_000;
const POST_START_CRYPTO_QUERY_RETRY_DELAYS_MS = [0, 250, 1_000, 2_500] as const;

export const waitForInitialSyncPrepared = async (
  mx: MatrixClient,
  timeoutMs = POST_START_CRYPTO_CONTINUITY_TIMEOUT_MS
): Promise<void> => {
  const startedAtMs = performance.now();
  if (mx.getSyncState() === SyncState.Prepared) {
    recordClientDiagnostic('session', 'initial-sync.prepared', {
      outcome: 'already-prepared',
      durationMs: 0,
    });
    return;
  }

  await new Promise<void>((resolve, reject) => {
    const handleSync = (state: SyncState) => {
      if (state !== SyncState.Prepared) return;
      cleanup();
      recordClientDiagnostic('session', 'initial-sync.prepared', {
        outcome: 'prepared',
        durationMs: performance.now() - startedAtMs,
      });
      resolve();
    };
    const cleanup = () => {
      clearTimeout(timer);
      mx.removeListener(ClientEvent.Sync, handleSync);
    };

    const timer = setTimeout(() => {
      cleanup();
      recordClientDiagnostic('session', 'initial-sync.prepared', {
        outcome: 'timeout',
        durationMs: performance.now() - startedAtMs,
      });
      reject(new Error('Initial sync did not become ready for the crypto continuity check.'));
    }, timeoutMs);
    mx.on(ClientEvent.Sync, handleSync);
  });
};

const waitForRetryDelay = (delayMs: number): Promise<void> =>
  delayMs === 0 ? Promise.resolve() : new Promise((resolve) => setTimeout(resolve, delayMs));

export const confirmFreshLoginCryptoContinuity = async (
  mx: MatrixClient,
  session: MatrixClientSession,
  {
    assertContinuity = assertCryptoStoreContinuity,
    clearPendingIdentity = clearPendingFreshLoginIdentity,
    retryDelaysMs = POST_START_CRYPTO_QUERY_RETRY_DELAYS_MS,
  }: {
    assertContinuity?: typeof assertCryptoStoreContinuity;
    clearPendingIdentity?: typeof clearPendingFreshLoginIdentity;
    retryDelaysMs?: readonly number[];
  } = {}
): Promise<void> => {
  let lastError: unknown;
  for (const delayMs of retryDelaysMs) {
    await waitForRetryDelay(delayMs);
    try {
      await assertContinuity(mx, {
        userId: session.userId,
        deviceId: session.deviceId,
        allowMissingServerDevice: false,
      });
      clearPendingIdentity(session);
      pendingFreshLoginContinuity.delete(mx);
      return;
    } catch (error) {
      lastError = error;
      if (
        error instanceof CryptoStoreContinuityError &&
        (error.reason === 'identity-key-mismatch' || error.reason === 'crypto-unavailable')
      ) {
        break;
      }
    }
  }
  throw lastError ?? new Error('Crypto continuity confirmation failed.');
};

export type StartClientContinuityDeps = {
  pendingSession?: MatrixClientSession;
  waitForPrepared?: typeof waitForInitialSyncPrepared;
  confirmContinuity?: typeof confirmFreshLoginCryptoContinuity;
};

export const startClient = async (
  mx: MatrixClient,
  {
    pendingSession: pendingSessionOverride,
    waitForPrepared = waitForInitialSyncPrepared,
    confirmContinuity = confirmFreshLoginCryptoContinuity,
  }: StartClientContinuityDeps = {}
) => {
  const startCallStartedAtMs = performance.now();
  const pendingSession = pendingSessionOverride ?? pendingFreshLoginContinuity.get(mx);
  let clientStartCompleted = false;
  recordClientDiagnostic('session', 'matrix-client.start-requested', {
    continuityConfirmationPending: Boolean(pendingSession),
  });
  try {
    await mx.startClient({
      lazyLoadMembers: true,
    });
    clientStartCompleted = true;
    recordClientDiagnostic('session', 'matrix-client.start-call-completed', {
      outcome: 'started',
      durationMs: performance.now() - startCallStartedAtMs,
      continuityConfirmationPending: Boolean(pendingSession),
    });
    if (!pendingSession) return;
    await waitForPrepared(mx);
    const continuityStartedAtMs = performance.now();
    await confirmContinuity(mx, pendingSession);
    recordClientDiagnostic('session', 'crypto-continuity.post-sync-completed', {
      outcome: 'matched',
      durationMs: performance.now() - continuityStartedAtMs,
    });
  } catch (error) {
    recordClientDiagnostic(
      'session',
      clientStartCompleted
        ? 'crypto-continuity.post-sync-completed'
        : 'matrix-client.start-call-completed',
      {
        outcome: 'error',
        durationMs: performance.now() - startCallStartedAtMs,
        continuityConfirmationPending: Boolean(pendingSession),
        errorType: error instanceof Error ? error.name : typeof error,
      }
    );
    if (!pendingSession) throw error;
    mx.stopClient();
    throw error instanceof CryptoStoreContinuityError
      ? error
      : new CryptoStoreContinuityError(
          pendingSession.userId,
          pendingSession.deviceId,
          'server-query-incomplete'
        );
  }
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
  { storage, ...depsOverrides }: Partial<PerformLogoutDeps> & { storage?: SessionLocalStorage } = {}
): Promise<void> => {
  const deps = { ...defaultPerformLogoutDeps(), ...depsOverrides };

  if (mx) {
    mx.stopClient();
    try {
      await mx.logout();
    } catch {
      // ignore if failed to logout
    }
  }

  await deps.clearPersistedSessions({ nativeSessionStore: deps.nativeSessionStore });
  deps.pushSessionToSW();

  deps.clearSessionLocalStorage(storage);
  clearNotificationCaches();
  deps.reload();
};

export const logoutClient = async (mx: MatrixClient) => performLogout(mx);

export const clearLoginData = async (
  storage: SessionLocalStorage | undefined = typeof window === 'undefined'
    ? undefined
    : window.localStorage
) => performLogout(undefined, { storage });

const canScheduleProactiveTokenRefresh = (session: MatrixClientSession): boolean =>
  Boolean(session.refreshToken) &&
  typeof session.expiresInMs === 'number' &&
  typeof session.storedAtMs === 'number' &&
  session.expiresInMs >= REFRESH_BEFORE_EXPIRY_MS;

export const scheduleProactiveTokenRefresh = (
  mx: MatrixClient,
  session: MatrixClientSession,
  deps: Partial<RefreshAndPersistSessionDeps> = {},
  nowMs = Date.now()
): ProactiveTokenRefreshHandle => {
  const resolvedDeps = { ...defaultRefreshDeps(), ...deps };

  if (!canScheduleProactiveTokenRefresh(session)) {
    return { dispose: () => undefined };
  }

  let disposed = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const clearScheduledRefresh = (): void => {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  };

  const scheduleRefreshForSession = (
    activeSession: MatrixClientSession,
    scheduleAtMs = Date.now()
  ): void => {
    clearScheduledRefresh();
    if (disposed || !canScheduleProactiveTokenRefresh(activeSession)) return;

    const refreshAtMs =
      activeSession.storedAtMs! + activeSession.expiresInMs! - REFRESH_BEFORE_EXPIRY_MS;
    const delayMs = Math.max(0, refreshAtMs - scheduleAtMs);

    timer = setTimeout(() => {
      void runRefresh(activeSession);
    }, delayMs);
  };

  const runRefresh = async (activeSession: MatrixClientSession) => {
    if (disposed || !activeSession.refreshToken) return;

    try {
      const { session: refreshedSession } = await refreshAndPersistSession(
        mx,
        activeSession,
        activeSession.refreshToken,
        resolvedDeps
      );
      scheduleRefreshForSession(refreshedSession);
    } catch {
      clearScheduledRefresh();
      await performLogout(mx, { nativeSessionStore: resolvedDeps.nativeSessionStore });
    }
  };

  scheduleRefreshForSession(session, nowMs);

  return {
    dispose: () => {
      disposed = true;
      clearScheduledRefresh();
    },
  };
};
