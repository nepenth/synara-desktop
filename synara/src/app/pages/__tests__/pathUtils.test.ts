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
