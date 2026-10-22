import { readFileSync } from 'node:fs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { isHomeRoomRoutePath, resolveRoomRoutePath, type RoomRouteScope } from '../roomRoutePath';

const scope = (overrides: Partial<RoomRouteScope> = {}): RoomRouteScope => ({
  roomToParents: new Map([
    ['!child:example.org', new Set(['!space:example.org'])],
    ['!nested:example.org', new Set(['!sub:example.org'])],
    ['!sub:example.org', new Set(['!space:example.org'])],
  ]),
  directRoomIds: new Set(['!dm:example.org']),
  ...overrides,
});

test('a DM resolves to Direct, not the Home "View" card', () => {
  assert.equal(
    resolveRoomRoutePath('!dm:example.org', { eventId: '$event:example.org' }, scope()),
    '/direct/!dm%3Aexample.org/%24event%3Aexample.org'
  );
  assert.equal(
    resolveRoomRoutePath('!dm:example.org', { threadRootId: '$root:example.org' }, scope()),
    '/direct/!dm%3Aexample.org/thread/%24root%3Aexample.org'
  );
});

test('a room inside a space, nested or not, resolves under its top-level space', () => {
  assert.equal(
    resolveRoomRoutePath('!child:example.org', { eventId: '$event:example.org' }, scope()),
    '/!space%3Aexample.org/!child%3Aexample.org/%24event%3Aexample.org'
  );
  assert.equal(
    resolveRoomRoutePath('!nested:example.org', {}, scope()),
    '/!space%3Aexample.org/!nested%3Aexample.org'
  );
  assert.equal(
    resolveRoomRoutePath('!child:example.org', { threadRootId: '$root:example.org' }, scope()),
    '/!space%3Aexample.org/!child%3Aexample.org/thread/%24root%3Aexample.org'
  );
});

test('a room with several top-level spaces prefers the selected one, then the picker', () => {
  const twoSpaces = scope({
    roomToParents: new Map([['!child:example.org', new Set(['!a:example.org', '!b:example.org'])]]),
  });
  assert.match(
    resolveRoomRoutePath(
      '!child:example.org',
      {},
      { ...twoSpaces, spaceSelectedId: '!b:example.org' }
    ),
    /^\/!b%3Aexample\.org\//
  );
  assert.match(
    resolveRoomRoutePath(
      '!child:example.org',
      {},
      { ...twoSpaces, pickParent: () => '!b:example.org' }
    ),
    /^\/!b%3Aexample\.org\//
  );
  assert.match(resolveRoomRoutePath('!child:example.org', {}, twoSpaces), /^\/!a%3Aexample\.org\//);
});

test('any other room resolves to Home, with canonical aliases applied', () => {
  const path = resolveRoomRoutePath(
    '!plain:example.org',
    { eventId: '$event:example.org' },
    scope({ toIdOrAlias: (id) => (id === '!plain:example.org' ? '#plain:example.org' : id) })
  );
  assert.equal(path, '/home/%23plain%3Aexample.org/%24event%3Aexample.org');
  assert.equal(isHomeRoomRoutePath(path), true);
  assert.equal(isHomeRoomRoutePath('/direct/!dm%3Aexample.org'), false);
});

test('a joined DM or space room opened from a Home link redirects to its own route', () => {
  const provider = readFileSync('src/app/pages/client/home/RoomProvider.tsx', 'utf8');
  assert.match(provider, /summary\.membership === 'join'/);
  assert.match(provider, /!room\.isSpaceRoom\(\)/);
  assert.match(
    provider,
    /roomPath\(room\.roomId, threadRootId \? \{ threadRootId \} : \{ eventId \}\)/
  );
  assert.match(provider, /if \(canonical && !isHomeRoomRoutePath\(canonical\)\) \{/);
  assert.match(provider, /<Navigate to=\{canonical\} replace \/>/);
});
