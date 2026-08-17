import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

test('matrix helpers no longer ship JS attachment encrypt/decrypt', () => {
  const source = readFileSync(join(process.cwd(), 'src/app/utils/matrix.ts'), 'utf8');
  const pkg = readFileSync(join(process.cwd(), 'package.json'), 'utf8');
  assert.doesNotMatch(
    source,
    /browser-encrypt-attachment|encryptFile|decryptFile|encryptAttachment|decryptAttachment|downloadEncryptedMedia|getThumbnailContent/
  );
  assert.doesNotMatch(pkg, /browser-encrypt-attachment/);
});
