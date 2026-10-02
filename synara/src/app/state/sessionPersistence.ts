import { clearSessionBootstrap } from './sessionBootstrap';
import { PENDING_FRESH_LOGIN_IDENTITY_KEY, type Session, type SessionStorage } from './sessions';
import { recordClientDiagnostic } from '../utils/clientDiagnostics';

export type FreshLoginBootstrapIdentity = Pick<
  Session,
  'userId' | 'deviceId' | 'baseUrl' | 'sessionGeneration'
>;

export type FreshLoginBootstrapMarker = Required<FreshLoginBootstrapIdentity> & {
  issuedAtMs: number;
};

/** A fresh-device bootstrap should never survive beyond the login hand-off window. */
export const FRESH_LOGIN_BOOTSTRAP_TTL_MS = 10 * 60 * 1000;

const getDefaultSessionStorage = (): SessionStorage | undefined =>
  typeof localStorage === 'undefined' ? undefined : localStorage;

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

/** Clear only the renderer bootstrap; native session cleanup completes before this call. */
export const clearPersistedSessions = async (): Promise<void> => {
  const clearStartedAtMs = performance.now();
  clearSessionBootstrap();
  recordClientDiagnostic('session', 'persisted-session-clear.completed', {
    outcome: 'completed',
    durationMs: performance.now() - clearStartedAtMs,
  });
};
