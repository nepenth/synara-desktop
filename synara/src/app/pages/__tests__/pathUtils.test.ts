import assert from 'node:assert/strict';
import test from 'node:test';
import { getHomeRoomPathWithViaServers } from '../pathUtils';

test('home room path helper preserves bare room path without via servers', () => {
  assert.equal(getHomeRoomPathWithViaServers('#room:server'), '/home/%23room%3Aserver');
});

test('home room path helper includes event and via servers for join route targets', () => {
  assert.equal(
    getHomeRoomPathWithViaServers('#room:server', '$event:server', ['one.example', 'two.example']),
    '/home/%23room%3Aserver/%24event%3Aserver?viaServers=one.example%2Ctwo.example'
  );
});

test('all room destinations encode Matrix room and event identifiers exactly once', async () => {
  const { getHomeRoomPath, getDirectRoomPath, getSpaceRoomPath, getSpacePath } = await import(
    '../pathUtils'
  );
  const { parseSynaraRouteDestination } = await import('../../routes/synaraRoutes');
  const room = '#agent room/%25:example.test';
  const event = '$event/+?%25:example.test';
  const space = '#parent space/%25:example.test';
  for (const [path, parent] of [
    [getHomeRoomPath(room, event), undefined],
    [getDirectRoomPath(room, event), undefined],
    [getSpaceRoomPath(space, room, event), space],
  ]) {
    assert.ok(path?.includes(encodeURIComponent(event)));
    const parsed = parseSynaraRouteDestination(path);
    assert.equal(parsed?.kind, 'room');
    if (parsed?.kind !== 'room') throw new Error('Expected room destination');
    assert.equal(parsed.roomIdOrAlias, room);
    assert.equal(parsed.eventId, event);
    assert.equal(parsed.parentSpaceIdOrAlias, parent);
  }
  assert.equal(getSpacePath(space), `/${encodeURIComponent(space)}`);
});

test('thread destinations encode the room and root and survive parse', async () => {
  const {
    getHomeRoomThreadPath,
    getDirectRoomThreadPath,
    getSpaceRoomThreadPath,
  } = await import('../pathUtils');
  const { parseSynaraRouteDestination } = await import('../../routes/synaraRoutes');
  const room = '#agent room/%25:example.test';
  const root = '$root/+?%25:example.test';
  const space = '#parent space/%25:example.test';
  for (const [path, parent] of [
    [getHomeRoomThreadPath(room, root), undefined],
    [getDirectRoomThreadPath(room, root), undefined],
    [getSpaceRoomThreadPath(space, room, root), space],
  ]) {
    assert.ok(path?.includes('/thread/'));
    assert.ok(path?.includes(encodeURIComponent(root)));
    const parsed = parseSynaraRouteDestination(path);
    assert.equal(parsed?.kind, 'room');
    if (parsed?.kind !== 'room') throw new Error('Expected room destination');
    assert.equal(parsed.roomIdOrAlias, room);
    assert.equal(parsed.threadRootId, root);
    assert.equal(parsed.eventId, undefined);
    assert.equal(parsed.parentSpaceIdOrAlias, parent);
  }
});

test('approval Back preserves its originating location and rejects invalid or self returns', async () => {
  const { getBackRoutePath } = await import('../../components/backRoutePath');
  const { createApprovalsNavigationState, getApprovalsOriginSpace } = await import(
    '../../routes/approvalsNavigation'
  );
  for (const pathname of ['/direct/', '/inbox/later/', '/%23space%3Aexample.test/', '/home/']) {
    const state = createApprovalsNavigationState({
      pathname,
      search: '?saved=true',
      hash: '',
      state: null,
    });
    assert.equal(getBackRoutePath('/approvals/', state), `${pathname}?saved=true`);
    assert.deepEqual(
      createApprovalsNavigationState({ pathname: '/approvals/', search: '', hash: '', state }),
      state
    );
  }
  assert.equal(getBackRoutePath('/approvals/'), '/home/');
  assert.equal(getBackRoutePath('/approvals/', { approvalsReturnTo: '//evil.test' }), '/home/');
  assert.equal(getBackRoutePath('/approvals/', { approvalsReturnTo: '/approvals/' }), '/home/');
  assert.equal(getBackRoutePath('/inbox/notifications/'), '/inbox/');
  assert.equal(getBackRoutePath('/explore/example.test/'), '/explore/');
  assert.equal(getBackRoutePath('/login/'), '/home/');
  assert.equal(getBackRoutePath('/%ZZ/'), '/home/');
  assert.equal(
    getApprovalsOriginSpace({ approvalsReturnTo: '/%23space%3Aexample.test/!room/%24event' }),
    '#space:example.test'
  );
});
