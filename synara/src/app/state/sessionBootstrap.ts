import { fallbackSessionStore, type Session, type SessionStore } from './sessions';

export type AsyncSessionStore = {
  getSession: () => Promise<Session | undefined>;
  setSession?: (session: Session) => Promise<boolean>;
  removeSession?: () => Promise<boolean>;
};

export type SessionBootstrapSource = 'native' | 'legacy-fallback' | 'none';

export const NATIVE_SESSION_STORE_ERROR = 'native-session-store-error' as const;

export type NativeSessionStoreError = typeof NATIVE_SESSION_STORE_ERROR;

export type SessionBootstrapResult = {
  session?: Session;
  source: SessionBootstrapSource;
  nativeStoreError?: NativeSessionStoreError;
};

export type SessionBootstrapOptions = {
  nativeSessionStore?: AsyncSessionStore;
  fallbackStore?: Pick<SessionStore, 'getFallbackSession'>;
};

let activeSession: Session | undefined;
let activeSessionSource: SessionBootstrapSource = 'none';
let activeNativeStoreError: SessionBootstrapResult['nativeStoreError'];
let activeBootstrapPromise: Promise<SessionBootstrapResult> | undefined;

export const resolveSessionBootstrap = async ({
  nativeSessionStore,
  fallbackStore = fallbackSessionStore,
}: SessionBootstrapOptions = {}): Promise<SessionBootstrapResult> => {
  let nativeStoreError: SessionBootstrapResult['nativeStoreError'];

  if (nativeSessionStore) {
    try {
      const nativeSession = await nativeSessionStore.getSession();
      if (nativeSession) {
        return { session: nativeSession, source: 'native' };
      }
    } catch {
      nativeStoreError = NATIVE_SESSION_STORE_ERROR;
    }
  }

  const fallbackSession = fallbackStore.getFallbackSession();
  if (fallbackSession) {
    return {
      session: fallbackSession,
      source: 'legacy-fallback',
      nativeStoreError,
    };
  }

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
      .catch(() => cacheSessionBootstrapResult({ source: 'none' }));
  }
  return activeBootstrapPromise;
};

export const getActiveSession = (): Session | undefined =>
  activeSession ?? fallbackSessionStore.getFallbackSession();

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
  'The native credential store could not be used. Your session is stored using legacy browser storage instead. Restart the app or sign in again to retry native storage.';

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
