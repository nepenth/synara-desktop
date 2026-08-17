import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

test('matrix helpers no longer ship JS attachment encrypt/decrypt', () => {
  const source = readFileSync(fileURLToPath(new URL('../matrix.ts', import.meta.url)), 'utf8');
  const pkg = readFileSync(
    fileURLToPath(new URL('../../../../package.json', import.meta.url)),
    'utf8'
  );
  assert.doesNotMatch(
    source,
    /browser-encrypt-attachment|encryptFile|decryptFile|encryptAttachment|decryptAttachment|downloadEncryptedMedia|getThumbnailContent/
  );
  assert.doesNotMatch(pkg, /browser-encrypt-attachment/);
});
