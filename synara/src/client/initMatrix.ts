import { clearSessionExpiryNotice } from '../app/utils/sessionExpiry';
import {
  createNativeMatrixClient,
  type NativeMatrixClient,
  type NativeSessionSnapshot,
} from '../app/features/native-client/nativeClientFacade';
import { clearNavToActivePathStore } from '../app/state/navToActivePath';
import {
  clearPendingFreshLoginIdentity,
  clearPersistedSessions,
  isPendingFreshLoginIdentity,
} from '../app/state/sessionPersistence';
import {
  clearSessionLocalStorage,
  type Session,
  type SessionLocalStorage,
} from '../app/state/sessions';
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
  startMatrixClient?: typeof startMatrixClient;
};

export const initClient = async (
  session: MatrixClientSession,
  {
    isPendingFreshLoginIdentity: isFreshLoginIdentity = isPendingFreshLoginIdentity,
    startMatrixClient: startClient = startMatrixClient,
  }: InitClientDeps = {}
): Promise<MatrixClient> => {
  const initStartedAtMs = performance.now();
  const freshLogin = isFreshLoginIdentity(session);
  if (freshLogin) clearSessionExpiryNotice();
  recordClientDiagnostic('session', 'matrix-client.bootstrap-decision', {
    freshLogin,
  });

  try {
    const client = await startClient();
    recordClientDiagnostic('session', 'matrix-client.bootstrap-completed', {
      outcome: 'initialized',
      durationMs: performance.now() - initStartedAtMs,
    });
    if (
      freshLogin &&
      client.getUserId() === session.userId &&
      client.getDeviceId() === session.deviceId &&
      client.getBaseUrl() === session.baseUrl &&
      client.getSessionGeneration() !== undefined &&
      String(client.getSessionGeneration()) === session.sessionGeneration
    ) {
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

/** Try every renderer cleanup step even if a listener or browser storage fails. */
const finishRendererCleanup = async (steps: Array<() => void | Promise<void>>) => {
  for (const step of steps) {
    try {
      await step();
    } catch {
      // Native session authority is independent; reload restores its actual state.
    }
  }
};

export const reloadApplication = async (mx: MatrixClient) => {
  const userId = mx.getSafeUserId();
  // stopClient only clears renderer cache/listeners; no native stop or wipe is invoked.
  await finishRendererCleanup([
    () => mx.stopClient(),
    () => clearNavToActivePathStore(userId),
    clearNotificationCaches,
  ]);
  if (typeof window !== 'undefined') window.location.reload();
};

export type PerformLogoutDeps = {
  clearPersistedSessions: () => Promise<void>;
  clearSessionLocalStorage: typeof clearSessionLocalStorage;
  logoutNativeSession: () => Promise<void>;
  reload: () => void;
};

const defaultPerformLogoutDeps = (): PerformLogoutDeps => ({
  clearPersistedSessions,
  clearSessionLocalStorage,
  logoutNativeSession: async () => {
    const result = await invokeDesktopWithAvailability<NativeSessionSnapshot>('matrix_logout');
    if (!result.available || result.value?.status !== 'logged_out') {
      throw new Error('Native logout did not complete. Retry before signing out.');
    }
  },
  reload: () => (typeof window !== 'undefined' ? window.location.reload() : undefined),
});

export const performLogout = async (
  mx?: MatrixClient,
  { storage, ...depsOverrides }: Partial<PerformLogoutDeps> & { storage?: SessionLocalStorage } = {}
): Promise<void> => {
  const deps = { ...defaultPerformLogoutDeps(), ...depsOverrides };

  // Native completion includes session and credential cleanup. Preserve renderer
  // state on failure so the user can retry; the native owner handles remote errors.
  if (mx) {
    await mx.logout();
  } else {
    await deps.logoutNativeSession();
  }

  // Once native confirms logout, every remaining renderer cleanup is attempted.
  // A storage/listener failure must not leave the document appearing signed in.
  await finishRendererCleanup([
    deps.clearPersistedSessions,
    () => deps.clearSessionLocalStorage(storage),
    clearNotificationCaches,
  ]);
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
