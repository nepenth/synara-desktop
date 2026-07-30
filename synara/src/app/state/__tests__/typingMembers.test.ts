import assert from 'node:assert/strict';
import test from 'node:test';
import { typingMembersFromNativeSnapshot } from '../typingMembers';

test('native typing projection maps rooms to receipts and skips empty rooms', () => {
  const rooms = typingMembersFromNativeSnapshot(
    [
      { roomId: '!a:example.org', userIds: ['@alice:example.org', '@bob:example.org'] },
      { roomId: '!b:example.org', userIds: [] },
    ],
    1_700_000_000_000
  );
  assert.equal(rooms.size, 1);
  assert.deepEqual(rooms.get('!a:example.org'), [
    { userId: '@alice:example.org', ts: 1_700_000_000_000 },
    { userId: '@bob:example.org', ts: 1_700_000_000_000 },
  ]);
});
