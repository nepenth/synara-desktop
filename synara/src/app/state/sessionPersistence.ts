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
  type SessionStorage,
  type SessionStore,
} from './sessions';

export type { NativeSessionStoreError };

/**
 * Non-secret Matrix account identity tracked across session persistence and client bootstrap.
 * Synara uses fixed IndexedDB store names, so only one Matrix account's local data can occupy
 * those stores at a time. Multi-account support remains a non-goal.
 */
export type MatrixSessionIdentity = Pick<Session, 'userId' | 'deviceId'>;

export const LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY =
  'synara_last_bootstrapped_matrix_identity' as const;
export const LAST_PERSISTED_MATRIX_IDENTITY_KEY = 'synara_last_persisted_matrix_identity' as const;

const getDefaultSessionStorage = (): SessionStorage | undefined =>
  typeof localStorage === 'undefined' ? undefined : localStorage;

const parseMatrixSessionIdentity = (value: string | null): MatrixSessionIdentity | undefined => {
  if (!value) {
    return undefined;
  }

  try {
    const parsed = JSON.parse(value) as Partial<MatrixSessionIdentity>;
    if (typeof parsed.userId === 'string' && typeof parsed.deviceId === 'string') {
      return { userId: parsed.userId, deviceId: parsed.deviceId };
    }
  } catch {
    // Ignore invalid metadata.
  }

  return undefined;
};

export const getLastBootstrappedMatrixIdentity = (
  storage?: SessionStorage
): MatrixSessionIdentity | undefined => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) {
    return undefined;
  }

  return parseMatrixSessionIdentity(
    resolvedStorage.getItem(LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY)
  );
};

export const setLastBootstrappedMatrixIdentity = (
  identity: MatrixSessionIdentity,
  storage?: SessionStorage
): void => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) {
    return;
  }

  resolvedStorage.setItem(
    LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY,
    JSON.stringify(identity)
  );
};

export const getLastPersistedMatrixIdentity = (
  storage?: SessionStorage
): MatrixSessionIdentity | undefined => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) {
    return undefined;
  }

  return parseMatrixSessionIdentity(resolvedStorage.getItem(LAST_PERSISTED_MATRIX_IDENTITY_KEY));
};

export const setLastPersistedMatrixIdentity = (
  identity: MatrixSessionIdentity,
  storage?: SessionStorage
): void => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) {
    return;
  }

  resolvedStorage.setItem(LAST_PERSISTED_MATRIX_IDENTITY_KEY, JSON.stringify(identity));
};

export const matrixSessionIdentitiesMatch = (
  left?: MatrixSessionIdentity,
  right?: MatrixSessionIdentity
): boolean => {
  if (!left || !right) {
    return false;
  }

  return left.userId === right.userId && left.deviceId === right.deviceId;
};

export const shouldClearMatrixStoresBeforeInit = (
  session: MatrixSessionIdentity,
  lastBootstrapped: MatrixSessionIdentity | undefined = getLastBootstrappedMatrixIdentity()
): boolean => {
  if (!lastBootstrapped) {
    return false;
  }

  return !matrixSessionIdentitiesMatch(session, lastBootstrapped);
};

export type ClearMatrixStoresForIdentityChangeOptions = {
  storage?: SessionStorage;
  clearStores?: () => Promise<void>;
};

export const clearMatrixStoresForIdentityChange = async (
  session: MatrixSessionIdentity,
  {
    storage,
    clearStores = clearMatrixLocalStores,
  }: ClearMatrixStoresForIdentityChangeOptions = {}
): Promise<boolean> => {
  if (!shouldClearMatrixStoresBeforeInit(session, getLastBootstrappedMatrixIdentity(storage))) {
    return false;
  }

  await clearStores();
  return true;
};

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
  storage?: SessionStorage;
};

export type LegacySessionMigrationOptions = SessionPersistenceOptions & {
  bootstrapResult?: SessionBootstrapResult;
};

export const SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS = 60_000;

export const isPersistedSessionExpired = (session: Session, nowMs = Date.now()): boolean => {
  const { expiresInMs, storedAtMs } = session;
  if (typeof expiresInMs !== 'number' || !Number.isFinite(expiresInMs)) {
    return false;
  }
  if (typeof storedAtMs !== 'number' || !Number.isFinite(storedAtMs)) {
    return false;
  }

  return nowMs > storedAtMs + expiresInMs + SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS;
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
  nativeSession.storedAtMs = Date.now();

  return nativeSession;
};

const toLegacyFallbackSession = (session: Session): Session => ({
  ...toFallbackSessionInput(session),
  fallbackSdkStores: true,
});

export const persistAuthenticatedSession = async (
  session: Session,
  {
    nativeSessionStore,
    fallbackStore = fallbackSessionStore,
    storage = getDefaultSessionStorage(),
  }: SessionPersistenceOptions = {}
): Promise<PersistedSessionResult> => {
  const nativeSession = toNativeSession(session);
  let nativeStoreError: NativeSessionStoreError | undefined;
  const persistedIdentity = {
    userId: nativeSession.userId,
    deviceId: nativeSession.deviceId,
  };

  if (nativeSessionStore?.setSession) {
    try {
      if (await nativeSessionStore.setSession(nativeSession)) {
        fallbackStore.removeFallbackSession();
        setSessionBootstrapResult({ session: nativeSession, source: 'native' });
        setLastPersistedMatrixIdentity(persistedIdentity, storage);
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
  setLastPersistedMatrixIdentity(persistedIdentity, storage);

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

export const reconcileExpiredPersistedSession = async (
  options: SessionPersistenceOptions = {}
): Promise<SessionBootstrapResult> => {
  const bootstrap = getSessionBootstrapResult();
  if (!bootstrap.session || !isPersistedSessionExpired(bootstrap.session)) {
    return bootstrap;
  }

  await clearPersistedSessions(options);
  return { source: 'none' };
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
