import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { lastActiveRoomPath } from '../lastActiveRoomPath';

const path = (pathname: string, search = '', hash = '') => ({ pathname, search, hash });
const utility = ['create/', 'join/', 'search/'];

test('cold start reopens the last Home room, keeping its query and hash', () => {
  const stored = new Map([['home', path('/home/!room%3Aexample.org/', '?viaServers=a', '#x')]]);
  assert.equal(
    lastActiveRoomPath(stored, 'home', '/home/', utility),
    '/home/!room%3Aexample.org/?viaServers=a#x'
  );
  // The persisted shape is a plain object; the loader reads it without React.
  assert.equal(
    lastActiveRoomPath({ home: path('/home/!room%3Aexample.org/') }, 'home', '/home/', utility),
    '/home/!room%3Aexample.org/'
  );
});

test('utility routes, the base path and other scopes are not restored', () => {
  assert.equal(
    lastActiveRoomPath(new Map([['home', path('/home/')]]), 'home', '/home/', utility),
    undefined
  );
  assert.equal(
    lastActiveRoomPath(new Map([['home', path('/home/create/')]]), 'home', '/home/', utility),
    undefined
  );
  assert.equal(
    lastActiveRoomPath(
      new Map([['home', path('/home/search/?term=x')]]),
      'home',
      '/home/',
      utility
    ),
    undefined
  );
  assert.equal(
    lastActiveRoomPath(
      new Map([['home', path('/direct/!dm%3Aexample.org/')]]),
      'home',
      '/home/',
      utility
    ),
    undefined
  );
  assert.equal(lastActiveRoomPath(new Map(), 'home', '/home/', utility), undefined);
  assert.equal(lastActiveRoomPath(undefined, 'direct', '/direct/', ['create/']), undefined);
  assert.equal(
    lastActiveRoomPath(
      new Map([['direct', path('/direct/!dm%3Aexample.org/')]]),
      'direct',
      '/direct/',
      ['create/']
    ),
    '/direct/!dm%3Aexample.org/'
  );
});

test('the one Home list restores its last room or DM (NAV-12)', () => {
  const homeBases = ['/home/', '/direct/'];
  assert.equal(
    lastActiveRoomPath(
      new Map([['home', path('/direct/!dm%3Aexample.org/', '', '#e')]]),
      'home',
      homeBases,
      utility
    ),
    '/direct/!dm%3Aexample.org/#e'
  );
  assert.equal(
    lastActiveRoomPath(new Map([['home', path('/home/!room/')]]), 'home', homeBases, utility),
    '/home/!room/'
  );
  for (const utilityPath of ['/direct/', '/direct/create/', '/home/join/']) {
    assert.equal(
      lastActiveRoomPath(new Map([['home', path(utilityPath)]]), 'home', homeBases, utility),
      undefined,
      utilityPath
    );
  }
  assert.equal(
    lastActiveRoomPath(new Map([['home', path('/space/!s/!r/')]]), 'home', homeBases, utility),
    undefined
  );
});

test('Home and Direct index routes redirect through the last-room loader before Welcome', () => {
  const router = readFileSync('src/app/pages/Router.tsx', 'utf8');
  assert.match(
    router,
    /lastRoomLoader\(\s*'home',\s*\[HOME_PATH, DIRECT_PATH\],\s*\[_CREATE_PATH, _JOIN_PATH, _SEARCH_PATH\]\s*\)/
  );
  // Both routes share the Home pane (one list for rooms and DMs, NAV-12).
  assert.match(router, /<MobileFriendlyPageNav path=\{\[HOME_PATH, DIRECT_PATH\]\}>\s*<Home \/>/);
  assert.doesNotMatch(router, /<Direct \/>/);
  assert.match(router, /lastRoomLoader\('direct', DIRECT_PATH, \[_CREATE_PATH\]\)/);
  // Welcome stays as the fallback when no room is remembered.
  assert.match(router, /element=\{<WelcomePage \/>\}/);
});
