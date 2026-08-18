import {
  clearSessionBootstrap,
  type AsyncSessionStore,
  type NativeSessionStoreError,
} from './sessionBootstrap';
import { clearMatrixLocalStores } from '../../client/matrixLocalStores';
import { PENDING_FRESH_LOGIN_IDENTITY_KEY, type Session, type SessionStorage } from './sessions';
import { recordClientDiagnostic } from '../utils/clientDiagnostics';

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

export type SessionPersistenceOptions = {
  nativeSessionStore?: Pick<AsyncSessionStore, never>;
};

export const clearPersistedSessions = async ({
  nativeSessionStore: _nativeSessionStore,
}: SessionPersistenceOptions = {}): Promise<void> => {
  const clearStartedAtMs = performance.now();
  let matrixStoreClearSuccess = false;
  clearSessionBootstrap();

  try {
    await clearMatrixLocalStores();
    matrixStoreClearSuccess = true;
  } catch {
    // Logout must continue even if IndexedDB cleanup is unavailable.
  }
  clearSessionBootstrap();
  recordClientDiagnostic('session', 'persisted-session-clear.completed', {
    outcome: 'completed',
    durationMs: performance.now() - clearStartedAtMs,
    nativeStoreConfigured: Boolean(_nativeSessionStore),
    matrixStoreClearSuccess,
  });
};
