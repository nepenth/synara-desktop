import {
  clearSessionBootstrap,
  getSessionBootstrapResult,
  NATIVE_SESSION_STORE_ERROR,
  setSessionBootstrapResult,
  type AsyncSessionStore,
  type NativeSessionStoreError,
  type SessionBootstrapResult,
} from './sessionBootstrap';
import { clearMatrixLocalStores } from '../../client/matrixLocalStores';
import {
  fallbackSessionStore,
  type FallbackSessionInput,
  type Session,
  type SessionStore,
} from './sessions';

export type { NativeSessionStoreError };

export type PersistedSessionResult = {
  session: Session;
  source: 'native' | 'legacy-fallback';
  nativeStoreError?: NativeSessionStoreError;
};

export type LegacySessionMigrationResult =
  | { status: 'skipped' }
  | { status: 'migrated'; session: Session }
  | { status: 'native-unavailable'; session: Session }
  | { status: 'failed'; session: Session; nativeStoreError: NativeSessionStoreError };

export type SessionPersistenceOptions = {
  nativeSessionStore?: Pick<AsyncSessionStore, 'setSession' | 'removeSession'>;
  fallbackStore?: Pick<SessionStore, 'setFallbackSession' | 'removeFallbackSession'>;
};

export type LegacySessionMigrationOptions = SessionPersistenceOptions & {
  bootstrapResult?: SessionBootstrapResult;
};

const toFallbackSessionInput = (session: Session): FallbackSessionInput => ({
  accessToken: session.accessToken,
  baseUrl: session.baseUrl,
  deviceId: session.deviceId,
  userId: session.userId,
});

const toNativeSession = (session: Session): Session => {
  const nativeSession: Session = {
    accessToken: session.accessToken,
    baseUrl: session.baseUrl,
    deviceId: session.deviceId,
    userId: session.userId,
  };

  if (session.refreshToken) nativeSession.refreshToken = session.refreshToken;
  if (typeof session.expiresInMs === 'number' && Number.isFinite(session.expiresInMs)) {
    nativeSession.expiresInMs = session.expiresInMs;
  }

  return nativeSession;
};

const toLegacyFallbackSession = (session: Session): Session => ({
  ...toFallbackSessionInput(session),
  fallbackSdkStores: true,
});

export const persistAuthenticatedSession = async (
  session: Session,
  { nativeSessionStore, fallbackStore = fallbackSessionStore }: SessionPersistenceOptions = {}
): Promise<PersistedSessionResult> => {
  const nativeSession = toNativeSession(session);
  let nativeStoreError: NativeSessionStoreError | undefined;

  if (nativeSessionStore?.setSession) {
    try {
      if (await nativeSessionStore.setSession(nativeSession)) {
        fallbackStore.removeFallbackSession();
        setSessionBootstrapResult({ session: nativeSession, source: 'native' });
        return {
          session: nativeSession,
          source: 'native',
        };
      }
    } catch {
      nativeStoreError = NATIVE_SESSION_STORE_ERROR;
    }
  }

  fallbackStore.setFallbackSession(toFallbackSessionInput(nativeSession));
  const fallbackSession = toLegacyFallbackSession(nativeSession);
  setSessionBootstrapResult({
    session: fallbackSession,
    source: 'legacy-fallback',
    nativeStoreError,
  });

  return {
    session: fallbackSession,
    source: 'legacy-fallback',
    nativeStoreError,
  };
};

export const migrateLegacySessionToNativeAfterClientInit = async ({
  nativeSessionStore,
  fallbackStore = fallbackSessionStore,
  bootstrapResult = getSessionBootstrapResult(),
}: LegacySessionMigrationOptions = {}): Promise<LegacySessionMigrationResult> => {
  if (bootstrapResult.source !== 'legacy-fallback' || !bootstrapResult.session) {
    return { status: 'skipped' };
  }

  const nativeSession = toNativeSession(bootstrapResult.session);
  if (!nativeSessionStore?.setSession) {
    return { status: 'native-unavailable', session: bootstrapResult.session };
  }

  try {
    if (!(await nativeSessionStore.setSession(nativeSession))) {
      return { status: 'native-unavailable', session: bootstrapResult.session };
    }
  } catch {
    return {
      status: 'failed',
      session: bootstrapResult.session,
      nativeStoreError: NATIVE_SESSION_STORE_ERROR,
    };
  }

  fallbackStore.removeFallbackSession();
  setSessionBootstrapResult({ session: nativeSession, source: 'native' });
  return { status: 'migrated', session: nativeSession };
};

export const clearPersistedSessions = async ({
  nativeSessionStore,
  fallbackStore = fallbackSessionStore,
}: SessionPersistenceOptions = {}): Promise<void> => {
  clearSessionBootstrap();

  try {
    await nativeSessionStore?.removeSession?.();
  } catch {
    // Logout and local data reset must continue even if the native store is unavailable.
  }

  fallbackStore.removeFallbackSession();
  try {
    await clearMatrixLocalStores();
  } catch {
    // Logout must continue even if IndexedDB cleanup is unavailable.
  }
  clearSessionBootstrap();
};
