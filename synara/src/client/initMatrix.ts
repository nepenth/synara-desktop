import { clearSessionExpiryNotice } from '../app/utils/sessionExpiry';
import { nativeSession, type NativeSession } from '../app/native/nativeSession';
import type { NativeSessionSnapshot } from '../app/native/nativeWire';
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
import { invokeDesktopWithAvailability } from '../app/utils/desktop';

import { getSafeMyUserId } from '../app/state/nativeIdentity';
/**
 * Renderer session boot. Native owns the Matrix client, token refresh, sync,
 * session and crypto; the renderer hydrates `native/nativeSession` from native
 * commands and reads through it.
 */

export type MatrixClient = NativeSession;
export type MatrixClientSession = Session;

/** Duck-typed MatrixError: an Error carrying a string `errcode`. */
export const isMatrixErrorLike = (error: unknown): error is Error & { errcode?: string } =>
  error instanceof Error && typeof (error as { errcode?: unknown } | null)?.errcode === 'string';

/** D1C: proactive renderer token refresh is abolished — native owns refresh. */
export const REFRESH_BEFORE_EXPIRY_MS = 0;

export type ProactiveTokenRefreshHandle = {
  dispose: () => void;
};

const noOpRefreshHandle: ProactiveTokenRefreshHandle = { dispose: () => undefined };

const startMatrixClient = async (): Promise<MatrixClient> => {
  const startupStartedAtMs = performance.now();
  const mx = nativeSession();
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
      client.getIdentity().userId === session.userId &&
      client.getIdentity().deviceId === session.deviceId &&
      client.getIdentity().homeserverUrl === session.baseUrl &&
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

/** Native sync is already live; this hydrates the renderer's cached reads. */
export const startClient = async (session: MatrixClient): Promise<void> => {
  await session.start();
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

export const reloadApplication = async () => {
  const userId = getSafeMyUserId();
  // stop() only clears renderer caches; no native stop or wipe is invoked.
  await finishRendererCleanup([
    () => nativeSession().stop(),
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

/** Fixed dialog copy when native logout rejects or does not report `logged_out`. */
export const LOGOUT_RETRY_COPY = 'Local sign out did not complete. Retry to finish local cleanup.';

export type LogoutAttemptOutcome = 'logged_out' | 'retry';

/**
 * One user-initiated Sign Out. A rejected or incomplete native logout keeps
 * the signed-in screen and resolves `retry`; it is never swallowed silently.
 * Native error text is not surfaced, only the fixed retry outcome.
 */
export const attemptLogout = async (
  mx?: MatrixClient,
  options?: Parameters<typeof performLogout>[1]
): Promise<LogoutAttemptOutcome> => {
  try {
    await performLogout(mx, options);
    return 'logged_out';
  } catch {
    return 'retry';
  }
};

export const logoutClient = async (mx: MatrixClient) => performLogout(mx);

export const clearLoginData = async (
  storage: SessionLocalStorage | undefined = typeof window === 'undefined'
    ? undefined
    : window.localStorage
) => performLogout(undefined, { storage });

/** D1C: native owns token refresh; renderer shelf is a no-op handle. */
export const scheduleProactiveTokenRefresh = (): ProactiveTokenRefreshHandle => noOpRefreshHandle;
