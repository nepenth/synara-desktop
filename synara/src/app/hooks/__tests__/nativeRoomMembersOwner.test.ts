import assert from 'node:assert/strict';
import test from 'node:test';

import { readRoomMembersWithNativeOwner } from '../nativeRoomMembersOwner';

const roomId = '!room:example.org';
const member = {
  roomId,
  userId: '@alice:example.org',
  displayName: 'Alice',
  avatarUrl: 'mxc://example.org/alice',
  membership: 'join',
  powerLevel: 50,
  isDirectTarget: false,
};

test('native room member owner reads the validated Rust snapshot', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const members = await readRoomMembersWithNativeOwner(roomId, true, async (command, args) => {
    calls.push({ command, args });
    return {
      available: true,
      value: {
        sessionGeneration: 4,
        roomId,
        members: [member],
      },
    };
  });

  assert.deepEqual(members, [member]);
  assert.deepEqual(calls, [
    {
      command: 'matrix_room_members_snapshot',
      args: { roomId },
    },
  ]);
});

test('native room member owner fails closed on unavailable or malformed IPC', async () => {
  await assert.rejects(
    readRoomMembersWithNativeOwner(roomId, true, async () => ({ available: false })),
    /Native Matrix room members are unavailable/
  );

  await assert.rejects(
    readRoomMembersWithNativeOwner(roomId, true, async () => ({
      available: true,
      value: {
        sessionGeneration: 4,
        roomId,
        members: [{ ...member, roomId: '!other:example.org' }],
      },
    })),
    /Native Matrix room members are unavailable/
  );
});

test('non-native sessions do not invoke the native member owner', async () => {
  const neverInvoke = async () => {
    throw new Error('native member IPC should not be called');
  };

  assert.equal(await readRoomMembersWithNativeOwner(roomId, false, neverInvoke), undefined);
});
