import test from 'node:test';
import assert from 'node:assert/strict';
import { normalizeSynaraRoute, parseSynaraRouteDestination } from '../synaraRoutes';

test('normalizeSynaraRoute accepts documented app-relative destinations', () => {
  const routes = [
    '/',
    '/home/',
    '/home/!room%3Aexample.org/%24event/',
    '/home/!room/$event/',
    '/direct/',
    '/direct/!room%3Aexample.org/',
    '/%23space%3Aexample.org/',
    '/%23space%3Aexample.org/lobby/',
    '/%23space%3Aexample.org/!room%3Aexample.org/%24event/',
    '/inbox/',
    '/inbox/notifications/',
    '/inbox/invites/',
    '/inbox/later/',
    '/create',
    '/explore/',
    '/explore/matrix.org/',
    '/settings/',
  ];

  routes.forEach((route) => assert.equal(normalizeSynaraRoute(route), route));
});

test('normalizeSynaraRoute rejects external and unsupported routes', () => {
  const routes = [
    '',
    'https://example.org',
    '//example.org/path',
    '/https://example.org',
    '/login/',
    '/register/matrix.org/',
    '/reset-password/matrix.org/',
    '/room-settings/',
    '/space-settings/',
    '/inbox/unknown/',
    '/home/!room//',
    '/home/%E0%A4%A/',
    '/unknown/a/b/c/',
    '/home/!room/$event/extra/',
    `/home/${'x'.repeat(2_048)}/`,
  ];

  routes.forEach((route) => assert.equal(normalizeSynaraRoute(route), undefined));
});

test('parseSynaraRouteDestination describes route destinations', () => {
  assert.deepEqual(parseSynaraRouteDestination('/inbox/later/'), {
    kind: 'inbox',
    section: 'later',
  });
  assert.deepEqual(parseSynaraRouteDestination('/home/!room%3Aexample.org/%24event/'), {
    kind: 'room',
    roomIdOrAlias: '!room:example.org',
    eventId: '$event',
  });
  assert.deepEqual(
    parseSynaraRouteDestination('/%23space%3Aexample.org/!room%3Aexample.org/%24event/'),
    {
      kind: 'room',
      parentSpaceIdOrAlias: '#space:example.org',
      roomIdOrAlias: '!room:example.org',
      eventId: '$event',
    }
  );
  assert.deepEqual(parseSynaraRouteDestination('/settings/'), {
    kind: 'settings',
  });
});
