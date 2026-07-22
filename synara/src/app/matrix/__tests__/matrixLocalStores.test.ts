import test from 'node:test';
import assert from 'node:assert/strict';
import {
  deleteIndexedDb,
  isCryptoAccountMismatchError,
  MATRIX_LOCAL_STORE_NAMES,
} from '../../../client/matrixLocalStores';
import {
  clearMatrixStoresForIdentityChange,
  shouldClearMatrixStoresBeforeInit,
} from '../../state/sessionPersistence';

const withIndexedDbDelete = async (
  trigger: 'success' | 'error' | 'blocked',
  run: () => Promise<void>
) => {
  const originalIndexedDb = globalThis.indexedDB;
  const request = {
    error: trigger === 'error' ? new DOMException('delete failed') : null,
    onsuccess: null as null | (() => void),
    onerror: null as null | (() => void),
    onblocked: null as null | (() => void),
  };

  Object.defineProperty(globalThis, 'indexedDB', {
    configurable: true,
    value: {
      deleteDatabase: () => {
        queueMicrotask(() => request[`on${trigger}`]?.());
        return request;
      },
    },
  });

  try {
    await run();
  } finally {
    Object.defineProperty(globalThis, 'indexedDB', {
      configurable: true,
      value: originalIndexedDb,
    });
  }
};

test('isCryptoAccountMismatchError detects rust crypto account mismatch failures', () => {
  const error = new Error(
    "the account in the store doesn't match the account in the constructor: expected @alice:example.org:OLDDEVICE, got @alice:example.org:NEWDEVICE"
  );

  assert.equal(isCryptoAccountMismatchError(error), true);
});

test('isCryptoAccountMismatchError ignores unrelated failures', () => {
  assert.equal(isCryptoAccountMismatchError(new Error('network timeout')), false);
  assert.equal(isCryptoAccountMismatchError('not-an-error'), false);
});

test('matrix local store names include sync, legacy crypto, and rust crypto stores', () => {
  assert.deepEqual(MATRIX_LOCAL_STORE_NAMES, [
    'web-sync-store',
    'crypto-store',
    'matrix-js-sdk::matrix-sdk-crypto',
    'matrix-js-sdk::matrix-sdk-crypto-meta',
  ]);
});

test('IndexedDB deletion resolves only after success', async () => {
  await withIndexedDbDelete('success', async () => {
    await deleteIndexedDb('matrix-test');
  });
});

test('IndexedDB deletion rejects failed and blocked deletes', async () => {
  await withIndexedDbDelete('error', async () => {
    await assert.rejects(() => deleteIndexedDb('matrix-test'), /delete failed/);
  });
  await withIndexedDbDelete('blocked', async () => {
    await assert.rejects(() => deleteIndexedDb('matrix-test'), /blocked by an open client/);
  });
});

test('account switch proactive clear complements crypto mismatch fallback detection', async () => {
  const previousIdentity = { userId: '@alice:example.org', deviceId: 'ALICE_DEVICE' };
  const nextIdentity = { userId: '@bob:example.org', deviceId: 'BOB_DEVICE' };
  const mismatchError = new Error(
    "the account in the store doesn't match the account in the constructor: expected @alice:example.org:ALICE_DEVICE, got @bob:example.org:BOB_DEVICE"
  );

  assert.equal(shouldClearMatrixStoresBeforeInit(nextIdentity, previousIdentity), true);
  assert.equal(isCryptoAccountMismatchError(mismatchError), true);

  let proactiveClearCalls = 0;
  const proactivelyCleared = await clearMatrixStoresForIdentityChange(nextIdentity, {
    storage: {
      getItem: (key) =>
        key === 'synara_last_bootstrapped_matrix_identity'
          ? JSON.stringify(previousIdentity)
          : null,
      setItem: () => undefined,
      removeItem: () => undefined,
    },
    clearStores: async () => {
      proactiveClearCalls += 1;
    },
  });

  assert.equal(proactivelyCleared, true);
  assert.equal(proactiveClearCalls, 1);
});
