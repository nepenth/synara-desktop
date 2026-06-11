export const MATRIX_SYNC_STORE_NAME = 'web-sync-store';
export const MATRIX_LEGACY_CRYPTO_STORE_NAME = 'crypto-store';
export const MATRIX_RUST_CRYPTO_STORE_PREFIX = 'matrix-js-sdk';

export const MATRIX_LOCAL_STORE_NAMES = [
  MATRIX_SYNC_STORE_NAME,
  MATRIX_LEGACY_CRYPTO_STORE_NAME,
  `${MATRIX_RUST_CRYPTO_STORE_PREFIX}::matrix-sdk-crypto`,
  `${MATRIX_RUST_CRYPTO_STORE_PREFIX}::matrix-sdk-crypto-meta`,
] as const;

const CRYPTO_ACCOUNT_MISMATCH_MESSAGE =
  "the account in the store doesn't match the account in the constructor";

export const isCryptoAccountMismatchError = (error: unknown): boolean => {
  if (!(error instanceof Error)) {
    return false;
  }

  return error.message.includes(CRYPTO_ACCOUNT_MISMATCH_MESSAGE);
};

const deleteIndexedDb = (name: string): Promise<void> =>
  new Promise((resolve) => {
    if (typeof indexedDB === 'undefined') {
      resolve();
      return;
    }

    const request = indexedDB.deleteDatabase(name);
    request.onsuccess = () => resolve();
    request.onerror = () => resolve();
    request.onblocked = () => resolve();
  });

export const clearMatrixLocalStores = async (): Promise<void> => {
  await Promise.all(MATRIX_LOCAL_STORE_NAMES.map((name) => deleteIndexedDb(name)));
};
