import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const read = (path: string) => readFileSync(join(process.cwd(), path), 'utf8');
const tile = read('src/app/components/BackupRestore.tsx');
const backupClient = read('src/app/features/backup/nativeBackup.ts');

test('Backup setup enrols through secret-storage bootstrap so the recovery key is shown', () => {
  const setupBranch = tile.split("if (action === 'setup_required') {")[1]?.split('}')[0] ?? '';
  assert.match(setupBranch, /bootstrapNativeSecretStorage\(secret\)/);
  assert.match(setupBranch, /return result\.recoveryKey/);
  assert.match(tile, /Copy this recovery key now/);
  assert.match(tile, /\{operationState\.data\}/);
});

test('The renderer no longer calls the key-dropping backup setup command', () => {
  assert.doesNotMatch(tile, /setupNativeBackup/);
  assert.doesNotMatch(backupClient, /'matrix_backup_setup'/);
});

test('Restore and repair keep their backup commands', () => {
  assert.match(tile, /await restoreNativeBackup\(secret\)/);
  assert.match(tile, /await repairNativeBackup\(secret\)/);
});
