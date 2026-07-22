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
  PENDING_FRESH_LOGIN_IDENTITY_KEY,
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

export type FreshLoginBootstrapIdentity = Pick<
  Session,
  'userId' | 'deviceId' | 'baseUrl' | 'sessionGeneration'
>;

export type FreshLoginBootstrapMarker = Required<FreshLoginBootstrapIdentity> & {
  issuedAtMs: number;
};

/** A fresh-device bootstrap should never survive beyond the login hand-off window. */
export const FRESH_LOGIN_BOOTSTRAP_TTL_MS = 10 * 60 * 1000;

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

const parseFreshLoginBootstrapMarker = (
  value: string | null
): FreshLoginBootstrapMarker | undefined => {
  if (!value) return undefined;
  try {
    const parsed = JSON.parse(value) as Partial<FreshLoginBootstrapMarker>;
    if (
      typeof parsed.userId === 'string' &&
      typeof parsed.deviceId === 'string' &&
      typeof parsed.baseUrl === 'string' &&
      typeof parsed.sessionGeneration === 'string' &&
      typeof parsed.issuedAtMs === 'number' &&
      Number.isFinite(parsed.issuedAtMs)
    ) {
      return parsed as FreshLoginBootstrapMarker;
    }
  } catch {
    // Invalid bootstrap metadata is fail-closed below.
  }
  return undefined;
};

export const createFreshLoginSessionGeneration = (): string => {
  if (typeof globalThis.crypto?.randomUUID === 'function') {
    return globalThis.crypto.randomUUID();
  }
  const bytes = new Uint8Array(16);
  globalThis.crypto?.getRandomValues?.(bytes);
  if (bytes.some((byte) => byte !== 0)) {
    return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
};

export const getPendingFreshLoginIdentity = (
  storage?: SessionStorage,
  nowMs = Date.now()
): FreshLoginBootstrapMarker | undefined => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) return undefined;
  const marker = parseFreshLoginBootstrapMarker(
    resolvedStorage.getItem(PENDING_FRESH_LOGIN_IDENTITY_KEY)
  );
  if (
    !marker ||
    marker.issuedAtMs > nowMs ||
    nowMs - marker.issuedAtMs > FRESH_LOGIN_BOOTSTRAP_TTL_MS
  ) {
    resolvedStorage.removeItem(PENDING_FRESH_LOGIN_IDENTITY_KEY);
    return undefined;
  }
  return marker;
};

export const markPendingFreshLoginIdentity = (
  identity: FreshLoginBootstrapIdentity,
  storage?: SessionStorage,
  issuedAtMs = Date.now()
): void => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage || !identity.sessionGeneration) return;
  const marker: FreshLoginBootstrapMarker = {
    userId: identity.userId,
    deviceId: identity.deviceId,
    baseUrl: identity.baseUrl,
    sessionGeneration: identity.sessionGeneration,
    issuedAtMs,
  };
  resolvedStorage.setItem(PENDING_FRESH_LOGIN_IDENTITY_KEY, JSON.stringify(marker));
};

export const isPendingFreshLoginIdentity = (
  identity: FreshLoginBootstrapIdentity,
  storage?: SessionStorage,
  nowMs = Date.now()
): boolean => {
  const marker = getPendingFreshLoginIdentity(storage, nowMs);
  return Boolean(
    marker &&
      identity.sessionGeneration &&
      marker.userId === identity.userId &&
      marker.deviceId === identity.deviceId &&
      marker.baseUrl === identity.baseUrl &&
      marker.sessionGeneration === identity.sessionGeneration
  );
};

export const clearPendingFreshLoginIdentity = (
  identity: FreshLoginBootstrapIdentity,
  storage?: SessionStorage,
  nowMs = Date.now()
): void => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage || !isPendingFreshLoginIdentity(identity, resolvedStorage, nowMs)) return;
  resolvedStorage.removeItem(PENDING_FRESH_LOGIN_IDENTITY_KEY);
};

export const getLastBootstrappedMatrixIdentity = (
  storage?: SessionStorage
): MatrixSessionIdentity | undefined => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) {
    return undefined;
  }

  return parseMatrixSessionIdentity(resolvedStorage.getItem(LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY));
};

export const setLastBootstrappedMatrixIdentity = (
  identity: MatrixSessionIdentity,
  storage?: SessionStorage
): void => {
  const resolvedStorage = storage ?? getDefaultSessionStorage();
  if (!resolvedStorage) {
    return;
  }

  resolvedStorage.setItem(LAST_BOOTSTRAPPED_MATRIX_IDENTITY_KEY, JSON.stringify(identity));
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
  { storage, clearStores = clearMatrixLocalStores }: ClearMatrixStoresForIdentityChangeOptions = {}
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
  /** Set only after a successful Matrix login/registration that created this device. */
  freshLogin?: boolean;
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
  ...(session.sessionGeneration ? { sessionGeneration: session.sessionGeneration } : {}),
});

const toNativeSession = (session: Session): Session => {
  const nativeSession: Session = {
    accessToken: session.accessToken,
    baseUrl: session.baseUrl,
    deviceId: session.deviceId,
    userId: session.userId,
  };

  if (session.refreshToken) nativeSession.refreshToken = session.refreshToken;
  if (session.sessionGeneration) nativeSession.sessionGeneration = session.sessionGeneration;
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
    freshLogin = false,
  }: SessionPersistenceOptions = {}
): Promise<PersistedSessionResult> => {
  const nativeSession = toNativeSession(
    freshLogin && !session.sessionGeneration
      ? { ...session, sessionGeneration: createFreshLoginSessionGeneration() }
      : session
  );
  let nativeStoreError: NativeSessionStoreError | undefined;
  const persistedIdentity = {
    userId: nativeSession.userId,
    deviceId: nativeSession.deviceId,
    baseUrl: nativeSession.baseUrl,
    sessionGeneration: nativeSession.sessionGeneration,
  };

  if (nativeSessionStore?.setSession) {
    try {
      if (await nativeSessionStore.setSession(nativeSession)) {
        fallbackStore.removeFallbackSession();
        setSessionBootstrapResult({ session: nativeSession, source: 'native' });
        setLastPersistedMatrixIdentity(persistedIdentity, storage);
        if (freshLogin) markPendingFreshLoginIdentity(persistedIdentity, storage);
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
  if (freshLogin) markPendingFreshLoginIdentity(persistedIdentity, storage);

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
