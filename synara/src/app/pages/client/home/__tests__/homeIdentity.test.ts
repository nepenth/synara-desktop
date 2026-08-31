import assert from 'node:assert/strict';
import test from 'node:test';

import { homeserverDisplayName } from '../homeIdentity';

test('home identity displays only the authenticated homeserver hostname', () => {
  assert.equal(
    homeserverDisplayName('https://matrix.whyland.com/_matrix/client?access_token=secret'),
    'matrix.whyland.com'
  );
  assert.equal(homeserverDisplayName('https://[2001:db8::1]:8448'), '[2001:db8::1]');
});

test('home identity falls back to Matrix user server and finally Home', () => {
  assert.equal(
    homeserverDisplayName('not a url', '@chris:matrix.example.org'),
    'matrix.example.org'
  );
  assert.equal(homeserverDisplayName(undefined, '@invalid'), 'Home');
});
