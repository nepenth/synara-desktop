import assert from 'node:assert/strict';
import test from 'node:test';
import { NativeInvoke, readSpaceHierarchyWithNativeOwner } from '../nativeSpaceHierarchyOwner';

const loggedIn: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (command === 'matrix_space_hierarchy_snapshot') {
    return {
      available: true,
      value: {
        sessionGeneration: 9,
        rooms: [
          {
            roomId: '!room:example.org',
            numJoinedMembers: 2,
            joinRule: 'public',
            worldReadable: true,
            guestCanJoin: false,
          },
        ],
      },
    };
  }
  return { available: false };
};

test('native hierarchy read rejects non-desktop ownership', async () => {
  await assert.rejects(
    () => readSpaceHierarchyWithNativeOwner('!space:example.org', false, loggedIn),
    /unavailable/
  );
});

test('native hierarchy read requires the managed logged-in session', async () => {
  const invoke: NativeInvoke = async () => ({
    available: true,
    value: { status: 'logged_out' },
  });
  await assert.rejects(
    () => readSpaceHierarchyWithNativeOwner('!space:example.org', true, invoke),
    /logged-in/
  );
});

test('native hierarchy read invokes the sole Rust owner', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    return loggedIn(command, args);
  };
  const snapshot = await readSpaceHierarchyWithNativeOwner('!space:example.org', true, invoke);
  assert.equal(snapshot.sessionGeneration, 9);
  assert.deepEqual(calls, [
    ['matrix_session_snapshot', undefined],
    ['matrix_space_hierarchy_snapshot', { roomId: '!space:example.org' }],
  ]);
});
