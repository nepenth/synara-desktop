import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const source = readFileSync(
  join(process.cwd(), 'src/app/features/message-search/useMessageSearch.ts'),
  'utf8'
);

test('native message search invokes matrix_message_search and never mx.search', () => {
  assert.match(source, /isNativeMatrixSession\(\)/);
  assert.match(source, /matrix_message_search/);
  assert.match(source, /invokeDesktopWithAvailability/);
  const nativeBranch = source.slice(
    source.indexOf('if (nativeSession)'),
    source.indexOf('const limit = 20')
  );
  assert.match(nativeBranch, /matrix_message_search/);
  assert.equal(nativeBranch.includes('mx.search'), false);
  assert.equal(nativeBranch.includes('leftover-unavailable'), false);
});
