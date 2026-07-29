import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { nativeRoomKeyErrorMessage, NativeRoomKeyTransferResult } from '../nativeRoomKeys';

test('native room-key result contains counts and labels but no secret material', () => {
  const result: NativeRoomKeyTransferResult = {
    outcome: 'complete',
    fileLabel: 'synara-room-keys.txt',
    keysProcessed: 17,
    roomsTouched: 4,
    status: {
      sessionGeneration: 6,
      kind: 'export',
      phase: 'succeeded',
      progressPercent: 100,
      keysProcessed: 17,
      roomsTouched: 4,
      fileLabel: 'synara-room-keys.txt',
    },
  };
  const serialized = JSON.stringify(result).toLowerCase();
  for (const forbidden of [
    'session_key',
    'ciphertext',
    '"passphrase":',
    '"path":',
    'access_token',
    'refresh_token',
  ]) {
    assert.equal(serialized.includes(forbidden), false);
  }
});

test('native room-key unavailable copy is privacy safe', () => {
  const message = nativeRoomKeyErrorMessage().toLowerCase();
  assert.equal(message.includes('native room-key transfer'), true);
  for (const forbidden of ['token', 'ciphertext', 'password']) {
    assert.equal(message.includes(forbidden), false);
  }
});

test('Local Backup has exactly one Rust IPC owner and no browser keyfile helper', () => {
  const localBackup = readFileSync(
    resolve('src/app/features/settings/devices/LocalBackup.tsx'),
    'utf8'
  );
  for (const forbidden of [
    'LegacyLocalBackup',
    'isNativeMatrixSession',
    'useMatrixClient',
    'getCrypto',
    'exportRoomKeysAsJson',
    'importRoomKeysAsJson',
    'FileSaver',
    'cryptE2ERoomKeys',
    'useFilePicker',
    '.arrayBuffer()',
    'new Blob',
  ]) {
    assert.equal(localBackup.includes(forbidden), false, `${forbidden} must stay deleted`);
  }
  assert.equal(localBackup.includes('exportNativeRoomKeys'), true);
  assert.equal(localBackup.includes('importNativeRoomKeys'), true);
  assert.equal(
    existsSync(resolve('src/util/cryptE2ERoomKeys.js')),
    false,
    'browser room-key crypto helper must stay physically deleted'
  );
});
