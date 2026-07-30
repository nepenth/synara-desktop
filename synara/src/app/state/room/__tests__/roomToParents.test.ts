import assert from 'node:assert/strict';
import test from 'node:test';
import { roomToParentsFromNativeSnapshot } from '../roomToParents';

test('native space parents projection builds child→parent sets', () => {
  const map = roomToParentsFromNativeSnapshot([
    { roomId: '!room:example.org', parentIds: ['!space:example.org', '!other:example.org'] },
    { roomId: '!empty:example.org', parentIds: [] },
  ]);
  assert.equal(map.size, 1);
  assert.deepEqual(
    [...(map.get('!room:example.org') ?? [])].sort(),
    ['!other:example.org', '!space:example.org']
  );
});
