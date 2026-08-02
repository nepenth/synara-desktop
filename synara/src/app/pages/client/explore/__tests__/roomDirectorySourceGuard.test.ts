import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const routeDir = join(process.cwd(), 'src/app/pages/client/explore');

test('Explore directory routes have no route-scoped JS directory owner', () => {
  for (const file of ['Server.tsx', 'Explore.tsx']) {
    const source = readFileSync(join(routeDir, file), 'utf8');
    for (const forbidden of [
      'matrix-js-sdk',
      'useMatrixClient',
      'mx.http',
      'mx.publicRooms',
      'mx.getThirdpartyProtocols',
      'authedRequest',
      'Method.Post',
      "'/publicRooms'",
    ]) {
      assert.equal(source.includes(forbidden), false, `${file} must not contain ${forbidden}`);
    }
  }
});
