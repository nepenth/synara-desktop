import assert from 'node:assert/strict';
import test from 'node:test';
import { NativeBackupStatus, nativeBackupErrorMessage } from '../nativeBackup';

const readyStatus: NativeBackupStatus = {
  sessionGeneration: 3,
  availability: 'available',
  enabled: true,
  version: '8',
  keyCount: 14,
  deviceState: 'ready',
  recoveryState: 'ready',
  action: 'none',
};

test('native backup status contains projection data only', () => {
  assert.equal(readyStatus.enabled, true);
  assert.equal(readyStatus.action, 'none');
  const serialized = JSON.stringify(readyStatus).toLowerCase();
  for (const forbidden of [
    'access_token',
    'refresh_token',
    'recovery_key',
    'private_key',
    'ciphertext',
    'passphrase',
    'password',
  ]) {
    assert.equal(serialized.includes(forbidden), false);
  }
});

test('native backup failure copy is privacy safe', () => {
  const message = nativeBackupErrorMessage().toLowerCase();
  assert.equal(message.includes('native encryption backup'), true);
  assert.equal(message.includes('secret'), false);
  assert.equal(message.includes('token'), false);
});
