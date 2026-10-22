import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';
import { useMediaAuthentication } from '../../../hooks/useMediaAuthentication';

const read = (path: string) => readFileSync(`src/app/${path}`, 'utf8');

test('D83: the signed-in client does not fetch /.well-known or /versions itself', () => {
  const clientRoot = read('pages/client/ClientRoot.tsx');
  assert.doesNotMatch(clientRoot, /AutoDiscovery|SpecVersions/);
  assert.equal(existsSync('src/app/pages/client/AutoDiscovery.tsx'), false);
  assert.equal(existsSync('src/app/pages/client/SpecVersions.tsx'), false);

  // Media goes through Core's synara-media protocol, which authenticates
  // itself, so the flag no longer depends on the server's spec versions.
  const mediaAuthentication = read('hooks/useMediaAuthentication.ts');
  assert.doesNotMatch(mediaAuthentication, /useSpecVersions/);
  assert.equal(useMediaAuthentication(), true);
  assert.match(read('utils/matrix.ts'), /_useAuthentication\?: boolean/);
});
