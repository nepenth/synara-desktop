import test from 'node:test';
import assert from 'node:assert/strict';
import {
  clearSecretStorageKeys,
  cryptoCallbacks,
  storePrivateKey,
} from '../../../client/secretStorageKeys';

const getCachedKey = async (keyId: string) =>
  cryptoCallbacks.getSecretStorageKey({ keys: { [keyId]: {} } });

test('clearSecretStorageKeys removes cached private keys', async () => {
  storePrivateKey('key1', new Uint8Array([1, 2, 3]));

  assert.notEqual(await getCachedKey('key1'), undefined);

  clearSecretStorageKeys();

  assert.equal(await getCachedKey('key1'), undefined);
});

test('clearSecretStorageKeys is idempotent', async () => {
  storePrivateKey('key1', new Uint8Array([1, 2, 3]));

  clearSecretStorageKeys();
  clearSecretStorageKeys();

  assert.equal(await getCachedKey('key1'), undefined);
});