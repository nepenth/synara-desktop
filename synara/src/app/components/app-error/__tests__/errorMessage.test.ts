import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

import { formatUnknownError } from '../errorMessage';

test('formatUnknownError prefers Error messages and route status text', () => {
  assert.equal(formatUnknownError(new Error('boom')), 'boom');
  assert.equal(formatUnknownError('plain'), 'plain');
  assert.equal(formatUnknownError({ status: 404, statusText: 'Not Found' }), '404 Not Found');
  assert.equal(formatUnknownError(undefined), 'An unexpected error occurred.');
});

test('router provides a route errorElement instead of the default React Router page', () => {
  const router = readFileSync(join(process.cwd(), 'src/app/pages/Router.tsx'), 'utf8');
  assert.match(router, /errorElement=\{<RouteError \/>\}/);
  assert.match(router, /import \{ RouteError \} from '\.\/RouteError'/);
});

test('room and space settings trap render errors inside the settings modal', () => {
  const roomSettings = readFileSync(
    join(process.cwd(), 'src/app/features/room-settings/RoomSettingsRenderer.tsx'),
    'utf8'
  );
  const spaceSettings = readFileSync(
    join(process.cwd(), 'src/app/features/space-settings/SpaceSettingsRenderer.tsx'),
    'utf8'
  );

  for (const source of [roomSettings, spaceSettings]) {
    assert.match(source, /<ErrorBoundary/);
    assert.match(source, /AppErrorFallback/);
    assert.match(source, /onClose=\{closeSettings\}/);
  }
});
