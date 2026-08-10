import assert from 'node:assert/strict';
import test from 'node:test';
import {
  nativeSecretStorageErrorMessage,
  NativeSecretStorageOperationResult,
  NativeSecretStorageStatus,
} from '../nativeSecretStorage';

const lockedStatus: NativeSecretStorageStatus = {
  sessionGeneration: 9,
  state: 'locked',
  exists: true,
  unlocked: false,
  defaultKeySet: true,
  passphraseConfigured: true,
  bootstrapReady: true,
  missingSecrets: ['encryption_backup'],
  action: 'unlock_required',
};

test('native secret storage projection and operation result contain no recovery material', () => {
  const result: NativeSecretStorageOperationResult = {
    outcome: 'complete',
    recoveryDocumentSaved: true,
    recoveryDocumentName: 'synara-recovery-key.txt',
    status: lockedStatus,
  };
  const serialized = JSON.stringify(result).toLowerCase();
  for (const forbidden of [
    'access_token',
    'refresh_token',
    'private_key',
    'ciphertext',
    '"passphrase":',
    '"recoverysecret":',
    '"secretstoragekey":',
  ]) {
    assert.equal(serialized.includes(forbidden), false);
  }
  assert.equal(result.status.action, 'unlock_required');
});

test('native secret storage failure copy is fixed and privacy safe', () => {
  const message = nativeSecretStorageErrorMessage().toLowerCase();
  assert.equal(message.includes('native secret storage'), true);
  for (const forbidden of ['token', 'ciphertext', 'password']) {
    assert.equal(message.includes(forbidden), false);
  }
});
