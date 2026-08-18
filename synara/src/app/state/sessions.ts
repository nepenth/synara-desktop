export type Session = {
  baseUrl: string;
  userId: string;
  deviceId: string;
  /** Host-owned generation marker when one is available. */
  sessionGeneration?: string;
};

export type SessionStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
};
/** Retired renderer-session keys retained only for one-way cleanup. */
export const FALLBACK_SESSION_KEYS = {
  accessToken: 'synara_access_token',
  deviceId: 'synara_device_id',
  userId: 'synara_user_id',
  baseUrl: 'synara_hs_base_url',
  sessionGeneration: 'synara_session_generation',
} as const;

export const AFTER_LOGIN_REDIRECT_PATH_KEY = 'after_login_redirect_url';

/**
 * Non-secret marker written after the homeserver creates a new device during login.
 * Crypto startup uses it to distinguish that device from a restored device whose
 * local crypto store has gone missing.
 */
export const PENDING_FRESH_LOGIN_IDENTITY_KEY = 'synara_pending_fresh_login_identity' as const;

export const NAV_TO_ACTIVE_PATH_PREFIX = 'navToActivePath';

/** Exact localStorage keys removed on logout. User settings keys are intentionally excluded. */
export const SESSION_LOCAL_STORAGE_EXACT_KEYS = [
  FALLBACK_SESSION_KEYS.accessToken,
  FALLBACK_SESSION_KEYS.deviceId,
  FALLBACK_SESSION_KEYS.userId,
  FALLBACK_SESSION_KEYS.baseUrl,
  FALLBACK_SESSION_KEYS.sessionGeneration,
  AFTER_LOGIN_REDIRECT_PATH_KEY,
  PENDING_FRESH_LOGIN_IDENTITY_KEY,
] as const;

/** localStorage key prefixes removed on logout (e.g. per-user navigation state). */
export const SESSION_LOCAL_STORAGE_PREFIXES = [NAV_TO_ACTIVE_PATH_PREFIX] as const;

export type SessionLocalStorage = SessionStorage & {
  readonly length?: number;
  key?: (index: number) => string | null;
};

const getDefaultSessionStorage = (): SessionStorage | undefined =>
  typeof localStorage === 'undefined' ? undefined : localStorage;

/** Remove only retired renderer credential-envelope keys during bootstrap. */
export const clearLegacyRendererSessionCredentials = (
  storage: SessionStorage = getDefaultSessionStorage() as SessionStorage
): void => {
  if (!storage) return;
  Object.values(FALLBACK_SESSION_KEYS).forEach((key) => storage.removeItem(key));
};

const removePrefixedSessionKeys = (storage: SessionLocalStorage, prefix: string): void => {
  if (typeof storage.length !== 'number' || typeof storage.key !== 'function') {
    return;
  }

  const keysToRemove: string[] = [];
  for (let index = 0; index < storage.length; index += 1) {
    const key = storage.key(index);
    if (key?.startsWith(prefix)) {
      keysToRemove.push(key);
    }
  }

  keysToRemove.forEach((key) => storage.removeItem(key));
};

export const clearSessionLocalStorage = (
  storage: SessionLocalStorage = getDefaultSessionStorage() as SessionLocalStorage
): void => {
  if (!storage) return;

  SESSION_LOCAL_STORAGE_EXACT_KEYS.forEach((key) => storage.removeItem(key));
  SESSION_LOCAL_STORAGE_PREFIXES.forEach((prefix) => removePrefixedSessionKeys(storage, prefix));
};
