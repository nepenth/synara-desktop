import { clearLegacyRendererSessionCredentials, type Session } from './sessions';
import { recordClientDiagnostic } from '../utils/clientDiagnostics';

export type AsyncSessionStore = {
  getSession: () => Promise<Session | undefined>;
  setSession?: (session: Session) => Promise<boolean>;
  removeSession?: () => Promise<boolean>;
};

export type SessionBootstrapSource = 'native' | 'none';

export const NATIVE_SESSION_STORE_ERROR = 'native-session-store-error' as const;

export type NativeSessionStoreError = typeof NATIVE_SESSION_STORE_ERROR;

export type SessionBootstrapResult = {
  session?: Session;
  source: SessionBootstrapSource;
  nativeStoreError?: NativeSessionStoreError;
};

export type SessionBootstrapOptions = {
  nativeSessionStore?: AsyncSessionStore;
};

let activeSession: Session | undefined;
let activeSessionSource: SessionBootstrapSource = 'none';
let activeNativeStoreError: SessionBootstrapResult['nativeStoreError'];
let activeBootstrapPromise: Promise<SessionBootstrapResult> | undefined;

export const resolveSessionBootstrap = async ({
  nativeSessionStore,
}: SessionBootstrapOptions = {}): Promise<SessionBootstrapResult> => {
  const bootstrapStartedAtMs = performance.now();
  let nativeStoreError: SessionBootstrapResult['nativeStoreError'];

  recordClientDiagnostic('session', 'bootstrap.started', {
    nativeStoreConfigured: Boolean(nativeSessionStore),
  });
  clearLegacyRendererSessionCredentials();

  if (nativeSessionStore) {
    const nativeReadStartedAtMs = performance.now();
    try {
      const nativeSession = await nativeSessionStore.getSession();
      recordClientDiagnostic('session', 'bootstrap.native-read-completed', {
        outcome: nativeSession ? 'found' : 'missing',
        durationMs: performance.now() - nativeReadStartedAtMs,
        identityOnly: true,
      });
      if (nativeSession) {
        recordClientDiagnostic('session', 'bootstrap.completed', {
          source: 'native',
          durationMs: performance.now() - bootstrapStartedAtMs,
        });
        return { session: nativeSession, source: 'native' };
      }
    } catch (error) {
      nativeStoreError = NATIVE_SESSION_STORE_ERROR;
      recordClientDiagnostic('session', 'bootstrap.native-read-completed', {
        outcome: 'error',
        durationMs: performance.now() - nativeReadStartedAtMs,
        errorType: error instanceof Error ? error.name : typeof error,
      });
    }
  }

  recordClientDiagnostic('session', 'bootstrap.completed', {
    source: 'none',
    durationMs: performance.now() - bootstrapStartedAtMs,
    nativeStoreError: Boolean(nativeStoreError),
  });
  return { source: 'none', nativeStoreError };
};

const cacheSessionBootstrapResult = (result: SessionBootstrapResult): SessionBootstrapResult => {
  activeSession = result.session;
  activeSessionSource = result.source;
  activeNativeStoreError = result.nativeStoreError;
  return result;
};

export const initializeSessionBootstrap = (
  options?: SessionBootstrapOptions
): Promise<SessionBootstrapResult> => {
  if (!activeBootstrapPromise) {
    activeBootstrapPromise = resolveSessionBootstrap(options)
      .then(cacheSessionBootstrapResult)
      .catch((error) => {
        recordClientDiagnostic('session', 'bootstrap.failed', {
          errorType: error instanceof Error ? error.name : typeof error,
        });
        return cacheSessionBootstrapResult({ source: 'none' });
      });
  }
  return activeBootstrapPromise;
};

export const getActiveSession = (): Session | undefined => activeSession;

export const getSessionBootstrapResult = (): SessionBootstrapResult => ({
  session: activeSession,
  source: activeSessionSource,
  nativeStoreError: activeNativeStoreError,
});

export const shouldSurfaceNativeStoreErrorWarning = (
  nativeStoreError: SessionBootstrapResult['nativeStoreError'],
  isDesktop: boolean
): boolean => isDesktop && nativeStoreError === NATIVE_SESSION_STORE_ERROR;

export const getNativeStoreErrorWarningMessage = (): string =>
  'The native credential store could not be used. Synara did not restore a session; unlock native credential storage and sign in again.';

export const setSessionBootstrapResult = (
  result: SessionBootstrapResult
): SessionBootstrapResult => {
  activeBootstrapPromise = Promise.resolve(result);
  return cacheSessionBootstrapResult(result);
};

export const clearSessionBootstrap = () => {
  activeSession = undefined;
  activeSessionSource = 'none';
  activeNativeStoreError = undefined;
  activeBootstrapPromise = undefined;
};

export const resetSessionBootstrapForTests = clearSessionBootstrap;
