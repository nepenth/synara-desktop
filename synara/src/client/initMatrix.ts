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
import { platformSessionStore } from '../app/platform';

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

export const logoutClient = async (mx: MatrixClient) => {
  await clearPersistedSessions({ nativeSessionStore: platformSessionStore });
  pushSessionToSW();
  mx.stopClient();
  try {
    await mx.logout();
  } catch {
    // ignore if failed to logout
  }
  await mx.clearStores();
  window.localStorage.clear();
  window.location.reload();
};

export const clearLoginData = async () => {
  await clearPersistedSessions({ nativeSessionStore: platformSessionStore });
  await clearMatrixLocalStores();
  window.localStorage.clear();
  window.location.reload();
};
