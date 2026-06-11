import { createClient, MatrixClient, IndexedDBStore, IndexedDBCryptoStore } from 'matrix-js-sdk';

import {
  clearMatrixLocalStores,
  isCryptoAccountMismatchError,
  MATRIX_LEGACY_CRYPTO_STORE_NAME,
  MATRIX_SYNC_STORE_NAME,
} from './matrixLocalStores';
import { cryptoCallbacks } from './secretStorageKeys';
import { clearNavToActivePathStore } from '../app/state/navToActivePath';
import { pushSessionToSW } from '../sw-session';
import { clearPersistedSessions } from '../app/state/sessionPersistence';
import { clearSessionLocalStorage, type SessionLocalStorage } from '../app/state/sessions';
import { platformSessionStore } from '../app/platform';
import type { SessionPersistenceOptions } from '../app/state/sessionPersistence';

type Session = {
  baseUrl: string;
  accessToken: string;
  userId: string;
  deviceId: string;
};

const createMatrixClient = (session: Session) => {
  const indexedDBStore = new IndexedDBStore({
    indexedDB: global.indexedDB,
    localStorage: global.localStorage,
    dbName: MATRIX_SYNC_STORE_NAME,
  });

  const legacyCryptoStore = new IndexedDBCryptoStore(
    global.indexedDB,
    MATRIX_LEGACY_CRYPTO_STORE_NAME
  );

  const mx = createClient({
    baseUrl: session.baseUrl,
    accessToken: session.accessToken,
    userId: session.userId,
    store: indexedDBStore,
    cryptoStore: legacyCryptoStore,
    deviceId: session.deviceId,
    timelineSupport: true,
    cryptoCallbacks: cryptoCallbacks as any,
    verificationMethods: ['m.sas.v1'],
  });

  mx.setMaxListeners(50);
  return mx;
};

const startMatrixClient = async (session: Session): Promise<MatrixClient> => {
  const mx = createMatrixClient(session);
  await mx.store.startup();
  await mx.initRustCrypto();
  return mx;
};

export const initClient = async (session: Session): Promise<MatrixClient> => {
  try {
    return await startMatrixClient(session);
  } catch (error) {
    if (!isCryptoAccountMismatchError(error)) {
      throw error;
    }

    await clearMatrixLocalStores();
    return startMatrixClient(session);
  }
};

export const startClient = async (mx: MatrixClient) => {
  await mx.startClient({
    lazyLoadMembers: true,
  });
};

export const clearCacheAndReload = async (mx: MatrixClient) => {
  mx.stopClient();
  clearNavToActivePathStore(mx.getSafeUserId());
  await mx.store.deleteAllData();
  window.location.reload();
};

export type PerformLogoutDeps = {
  clearPersistedSessions: (options?: SessionPersistenceOptions) => Promise<void>;
  pushSessionToSW: typeof pushSessionToSW;
  clearSessionLocalStorage: typeof clearSessionLocalStorage;
  nativeSessionStore: SessionPersistenceOptions['nativeSessionStore'];
  reload: () => void;
};

const defaultPerformLogoutDeps = (): PerformLogoutDeps => ({
  clearPersistedSessions,
  pushSessionToSW,
  clearSessionLocalStorage,
  nativeSessionStore: platformSessionStore,
  reload: () => window.location.reload(),
});

export const performLogout = async (
  mx?: MatrixClient,
  {
    storage,
    ...depsOverrides
  }: Partial<PerformLogoutDeps> & { storage?: SessionLocalStorage } = {}
): Promise<void> => {
  const deps = { ...defaultPerformLogoutDeps(), ...depsOverrides };

  await deps.clearPersistedSessions({ nativeSessionStore: deps.nativeSessionStore });
  deps.pushSessionToSW();

  if (mx) {
    mx.stopClient();
    try {
      await mx.logout();
    } catch {
      // ignore if failed to logout
    }
    await mx.clearStores();
  }

  deps.clearSessionLocalStorage(storage);
  deps.reload();
};

export const logoutClient = async (mx: MatrixClient) => performLogout(mx);

export const clearLoginData = async (
  storage = typeof window === 'undefined' ? undefined : window.localStorage
) => performLogout(undefined, { storage: storage as SessionLocalStorage });
