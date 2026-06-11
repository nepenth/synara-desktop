// import { atom } from 'jotai';
// import {
//   atomWithLocalStorage,
//   getLocalStorageItem,
//   setLocalStorageItem,
// } from './utils/atomWithLocalStorage';

export type Session = {
  baseUrl: string;
  userId: string;
  deviceId: string;
  accessToken: string;
  expiresInMs?: number;
  storedAtMs?: number;
  refreshToken?: string;
  fallbackSdkStores?: boolean;
};

export type Sessions = Session[];
export type SessionStoreName = {
  sync: string;
  crypto: string;
};
export type SessionStorage = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
};
export type FallbackSessionInput = Pick<Session, 'accessToken' | 'baseUrl' | 'deviceId' | 'userId'>;
export type SessionStore = {
  setFallbackSession: (session: FallbackSessionInput) => void;
  removeFallbackSession: () => void;
  getFallbackSession: () => Session | undefined;
};

export const FALLBACK_SESSION_KEYS = {
  accessToken: 'synara_access_token',
  deviceId: 'synara_device_id',
  userId: 'synara_user_id',
  baseUrl: 'synara_hs_base_url',
} as const;

export const AFTER_LOGIN_REDIRECT_PATH_KEY = 'after_login_redirect_url';

export const NAV_TO_ACTIVE_PATH_PREFIX = 'navToActivePath';

/** Exact localStorage keys removed on logout. User settings keys are intentionally excluded. */
export const SESSION_LOCAL_STORAGE_EXACT_KEYS = [
  FALLBACK_SESSION_KEYS.accessToken,
  FALLBACK_SESSION_KEYS.deviceId,
  FALLBACK_SESSION_KEYS.userId,
  FALLBACK_SESSION_KEYS.baseUrl,
  AFTER_LOGIN_REDIRECT_PATH_KEY,
] as const;

/** localStorage key prefixes removed on logout (e.g. per-user navigation state). */
export const SESSION_LOCAL_STORAGE_PREFIXES = [NAV_TO_ACTIVE_PATH_PREFIX] as const;

export type SessionLocalStorage = SessionStorage & {
  readonly length?: number;
  key?: (index: number) => string | null;
};

/**
 * Single-session fallback storage for Synara.
 *
 * Used only when the native secret store cannot persist sessions. Tokens live in
 * localStorage, which is weaker than OS keychain storage under XSS. Settings
 * surfaces `nativeStoreError` when this path is active; fallback keys are cleared
 * on logout via `performLogout`.
 */
// const FALLBACK_STORE_NAME: SessionStoreName = {
//   sync: 'web-sync-store',
//   crypto: 'crypto-store',
// } as const;

export const createLocalStorageSessionStore = (storage: SessionStorage): SessionStore => ({
  setFallbackSession: (session) => {
    storage.setItem(FALLBACK_SESSION_KEYS.accessToken, session.accessToken);
    storage.setItem(FALLBACK_SESSION_KEYS.deviceId, session.deviceId);
    storage.setItem(FALLBACK_SESSION_KEYS.userId, session.userId);
    storage.setItem(FALLBACK_SESSION_KEYS.baseUrl, session.baseUrl);
  },
  removeFallbackSession: () => {
    storage.removeItem(FALLBACK_SESSION_KEYS.baseUrl);
    storage.removeItem(FALLBACK_SESSION_KEYS.userId);
    storage.removeItem(FALLBACK_SESSION_KEYS.deviceId);
    storage.removeItem(FALLBACK_SESSION_KEYS.accessToken);
  },
  getFallbackSession: () => {
    const baseUrl = storage.getItem(FALLBACK_SESSION_KEYS.baseUrl);
    const userId = storage.getItem(FALLBACK_SESSION_KEYS.userId);
    const deviceId = storage.getItem(FALLBACK_SESSION_KEYS.deviceId);
    const accessToken = storage.getItem(FALLBACK_SESSION_KEYS.accessToken);

    if (baseUrl && userId && deviceId && accessToken) {
      return {
        baseUrl,
        userId,
        deviceId,
        accessToken,
        fallbackSdkStores: true,
      };
    }

    return undefined;
  },
});

const getDefaultSessionStorage = (): SessionStorage | undefined =>
  typeof localStorage === 'undefined' ? undefined : localStorage;

export const fallbackSessionStore: SessionStore = {
  setFallbackSession: (session) => {
    const storage = getDefaultSessionStorage();
    if (!storage) return;
    createLocalStorageSessionStore(storage).setFallbackSession(session);
  },
  removeFallbackSession: () => {
    const storage = getDefaultSessionStorage();
    if (!storage) return;
    createLocalStorageSessionStore(storage).removeFallbackSession();
  },
  getFallbackSession: () => {
    const storage = getDefaultSessionStorage();
    if (!storage) return undefined;
    return createLocalStorageSessionStore(storage).getFallbackSession();
  },
};

export function setFallbackSession(
  accessToken: string,
  deviceId: string,
  userId: string,
  baseUrl: string
) {
  fallbackSessionStore.setFallbackSession({ accessToken, deviceId, userId, baseUrl });
}
export const removeFallbackSession = () => {
  fallbackSessionStore.removeFallbackSession();
};
export const getFallbackSession = (): Session | undefined => {
  return fallbackSessionStore.getFallbackSession();
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
/**
 * End of single-session fallback storage.
 */

// export const getSessionStoreName = (session: Session): SessionStoreName => {
//   if (session.fallbackSdkStores) {
//     return FALLBACK_STORE_NAME;
//   }

//   return {
//     sync: `sync${session.userId}`,
//     crypto: `crypto${session.userId}`,
//   };
// };

// export const MATRIX_SESSIONS_KEY = 'matrixSessions';
// const baseSessionsAtom = atomWithLocalStorage<Sessions>(
//   MATRIX_SESSIONS_KEY,
//   (key) => {
//     const defaultSessions: Sessions = [];
//     const sessions = getLocalStorageItem(key, defaultSessions);

//     // Before multi account support session was stored
//     // as multiple item in local storage.
//     // So we need these migration code.
//     const fallbackSession = getFallbackSession();
//     if (fallbackSession) {
//       removeFallbackSession();
//       sessions.push(fallbackSession);
//       setLocalStorageItem(key, sessions);
//     }
//     return sessions;
//   },
//   (key, value) => {
//     setLocalStorageItem(key, value);
//   }
// );

// export type SessionsAction =
//   | {
//       type: 'PUT';
//       session: Session;
//     }
//   | {
//       type: 'DELETE';
//       session: Session;
//     };

// export const sessionsAtom = atom<Sessions, [SessionsAction], undefined>(
//   (get) => get(baseSessionsAtom),
//   (get, set, action) => {
//     if (action.type === 'PUT') {
//       const sessions = [...get(baseSessionsAtom)];
//       const sessionIndex = sessions.findIndex(
//         (session) => session.userId === action.session.userId
//       );
//       if (sessionIndex === -1) {
//         sessions.push(action.session);
//       } else {
//         sessions.splice(sessionIndex, 1, action.session);
//       }
//       set(baseSessionsAtom, sessions);
//       return;
//     }
//     if (action.type === 'DELETE') {
//       const sessions = get(baseSessionsAtom).filter(
//         (session) => session.userId !== action.session.userId
//       );
//       set(baseSessionsAtom, sessions);
//     }
//   }
// );
