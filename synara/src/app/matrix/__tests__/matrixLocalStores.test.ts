import test from 'node:test';
import assert from 'node:assert/strict';
import {
  isCryptoAccountMismatchError,
  MATRIX_LOCAL_STORE_NAMES,
} from '../../../client/matrixLocalStores';

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