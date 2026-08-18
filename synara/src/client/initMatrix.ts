import {
  createNativeMatrixClient,
  type NativeMatrixClient,
} from '../app/features/native-client/nativeClientFacade';
import { clearNavToActivePathStore } from '../app/state/navToActivePath';
import {
  clearPendingFreshLoginIdentity,
  clearPersistedSessions,
  isPendingFreshLoginIdentity,
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
import { recordClientDiagnostic } from '../app/utils/clientDiagnostics';
import { invokeDesktopWithAvailability, type DesktopInvokeResult } from '../app/utils/desktop';

/**
 * F6c — renderer client boot on the native facade (Option A + D1C).
 * The js-sdk `createClient`/IndexedDB stores/token refresh are gone: the
 * renderer cedes token custody entirely to native (D1C), which owns refresh,
 * sync, session, and crypto. `initClient` returns the facade; all renderer
 * reads flow through the injected command bridge.
 *
 * The operator dropped the web fallback (native macOS/Linux + iOS only), so
 * the facade is the sole client construction path.
 */

export type MatrixClient = NativeMatrixClient;
export type MatrixClientSession = Session;

/** Matches the facade's NativeInvoke (DesktopInvokeResult-shaped). */
type NativeInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<unknown>>;

/** Duck-typed MatrixError: an Error carrying a string `errcode`. */
export const isMatrixErrorLike = (error: unknown): error is Error & { errcode?: string } =>
  error instanceof Error && typeof (error as { errcode?: unknown } | null)?.errcode === 'string';

/** D1C: proactive renderer token refresh is abolished — native owns refresh. */
export const REFRESH_BEFORE_EXPIRY_MS = 0;

export type ProactiveTokenRefreshHandle = {
  dispose: () => void;
};

const noOpRefreshHandle: ProactiveTokenRefreshHandle = { dispose: () => undefined };

/** Native invoke for the facade (fail-closed to unavailable off-desktop). */
const nativeInvoke: NativeInvoke = (command, args) => invokeDesktopWithAvailability(command, args);

const createMatrixClient = (): MatrixClient => createNativeMatrixClient(nativeInvoke);

const startMatrixClient = async (): Promise<MatrixClient> => {
  const startupStartedAtMs = performance.now();
  const mx = createMatrixClient();
  recordClientDiagnostic('session', 'matrix-client.initialization-started', {
    hasRefreshToken: false,
    fallbackSdkStores: false,
  });
  try {
    await mx.refresh();
    recordClientDiagnostic('session', 'matrix-client.initialization-completed', {
      outcome: 'ready-to-start',
      durationMs: performance.now() - startupStartedAtMs,
    });
    return mx;
  } catch (error) {
    recordClientDiagnostic('session', 'matrix-client.initialization-completed', {
      outcome: 'error',
      durationMs: performance.now() - startupStartedAtMs,
      errorType: error instanceof Error ? error.name : typeof error,
    });
    throw error;
  }
};

export type InitClientDeps = {
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
    isPendingFreshLoginIdentity: isFreshLoginIdentity = isPendingFreshLoginIdentity,
    setLastBootstrappedMatrixIdentity: setLastBootstrapped = setLastBootstrappedMatrixIdentity,
    startMatrixClient: startClient = startMatrixClient,
  }: InitClientDeps = {}
): Promise<MatrixClient> => {
  const initStartedAtMs = performance.now();
  const freshLogin = isFreshLoginIdentity(session);
  recordClientDiagnostic('session', 'matrix-client.bootstrap-decision', {
    freshLogin,
    identityStoresCleared: false,
  });

  try {
    const client = await startClient();
    recordBootstrappedMatrixIdentity(session, setLastBootstrapped);
    recordClientDiagnostic('session', 'matrix-client.bootstrap-completed', {
      outcome: 'initialized',
      durationMs: performance.now() - initStartedAtMs,
    });
    if (freshLogin) {
      clearPendingFreshLoginIdentity(session);
    }
    return client;
  } catch (error) {
    recordClientDiagnostic('session', 'matrix-client.bootstrap-completed', {
      outcome: 'error',
      durationMs: performance.now() - initStartedAtMs,
      errorType: error instanceof Error ? error.name : typeof error,
    });
    throw error;
  }
};

/** Start sync on the facade (native sync is already live; this hydrates reads). */
export const startClient = async (mx: MatrixClient): Promise<void> => {
  await mx.startClient();
};

export const clearCacheAndReload = async (mx: MatrixClient) => {
  await mx.stopClient();
  clearNavToActivePathStore(mx.getSafeUserId());
  clearNotificationCaches();
  await invokeDesktopWithAvailability<boolean>('matrix_clear_session');
  if (typeof window !== 'undefined') window.location.reload();
};

export type PerformLogoutDeps = {
  clearPersistedSessions: (options?: SessionPersistenceOptions) => Promise<void>;
  clearSessionLocalStorage: typeof clearSessionLocalStorage;
  logoutNativeSession: () => Promise<void>;
  nativeSessionStore: SessionPersistenceOptions['nativeSessionStore'];
  reload: () => void;
};

const defaultPerformLogoutDeps = (): PerformLogoutDeps => ({
  clearPersistedSessions,
  clearSessionLocalStorage,
  logoutNativeSession: async () => {
    await invokeDesktopWithAvailability('matrix_logout');
  },
  nativeSessionStore: platformSessionStore,
  reload: () => (typeof window !== 'undefined' ? window.location.reload() : undefined),
});

export const performLogout = async (
  mx?: MatrixClient,
  { storage, ...depsOverrides }: Partial<PerformLogoutDeps> & { storage?: SessionLocalStorage } = {}
): Promise<void> => {
  const deps = { ...defaultPerformLogoutDeps(), ...depsOverrides };

  if (mx) {
    try {
      await mx.stopClient();
      await mx.logout();
    } catch {
      // ignore if failed to logout
    }
  } else {
    try {
      await deps.logoutNativeSession();
    } catch {
      // Renderer cleanup and reload still run if native cleanup reports an error.
    }
  }

  await deps.clearPersistedSessions({ nativeSessionStore: deps.nativeSessionStore });

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

/** D1C: native owns token refresh; renderer shelf is a no-op handle. */
export const scheduleProactiveTokenRefresh = (): ProactiveTokenRefreshHandle => noOpRefreshHandle;
