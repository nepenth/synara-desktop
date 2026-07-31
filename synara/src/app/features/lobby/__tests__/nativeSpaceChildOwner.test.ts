import assert from 'node:assert/strict';
import test from 'node:test';
import {
  readSpaceChildrenWithNativeOwner,
  reparentRestrictedJoinWithNativeOwner,
  removeSpaceChildWithNativeOwner,
  setSpaceChildWithNativeOwner,
  type NativeInvoke,
} from '../nativeSpaceChildOwner';

const loggedIn: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (command === 'matrix_space_children_snapshot') {
    return {
      available: true,
      value: {
        sessionGeneration: 3,
        edges: [
          {
            parentId: '!space:example.org',
            childId: '!room:example.org',
            order: 'a',
            suggested: true,
            via: ['example.org'],
            originServerTs: 10,
          },
        ],
      },
    };
  }
  if (command === 'matrix_space_child_set') {
    return {
      available: true,
      value: { parentId: '!space:example.org', childId: '!room:example.org', status: 'updated' },
    };
  }
  if (command === 'matrix_space_child_remove') {
    return {
      available: true,
      value: { parentId: '!space:example.org', childId: '!room:example.org', status: 'removed' },
    };
  }
  if (command === 'matrix_restricted_join_reparent') {
    return { available: true, value: { roomId: '!room:example.org', status: 'updated' } };
  }
  return { available: false };
};

test('space children read rejects non-desktop ownership', async () => {
  await assert.rejects(() => readSpaceChildrenWithNativeOwner(false, loggedIn), /unavailable/);
});

test('space children read requires logged-in session', async () => {
  const invoke: NativeInvoke = async () => ({
    available: true,
    value: { status: 'logged_out' },
  });
  await assert.rejects(() => readSpaceChildrenWithNativeOwner(true, invoke), /logged-in/);
});

test('space children read invokes sole Rust owner', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    return loggedIn(command, args);
  };
  const snapshot = await readSpaceChildrenWithNativeOwner(true, invoke);
  assert.equal(snapshot.sessionGeneration, 3);
  assert.equal(snapshot.edges[0]?.childId, '!room:example.org');
  assert.deepEqual(calls, [
    ['matrix_session_snapshot', undefined],
    ['matrix_space_children_snapshot', undefined],
  ]);
});

test('set/remove/reparent invoke sole Rust owners', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    return loggedIn(command, args);
  };
  await setSpaceChildWithNativeOwner(
    '!space:example.org',
    '!room:example.org',
    { via: ['example.org'], suggested: true, order: 'b' },
    true,
    invoke,
  );
  await removeSpaceChildWithNativeOwner('!space:example.org', '!room:example.org', true, invoke);
  await reparentRestrictedJoinWithNativeOwner(
    '!room:example.org',
    '!old:example.org',
    '!new:example.org',
    true,
    invoke,
  );
  assert.deepEqual(
    calls.filter(([command]) => command !== 'matrix_session_snapshot'),
    [
      [
        'matrix_space_child_set',
        {
          parentId: '!space:example.org',
          childId: '!room:example.org',
          via: ['example.org'],
          order: 'b',
          suggested: true,
        },
      ],
      [
        'matrix_space_child_remove',
        { parentId: '!space:example.org', childId: '!room:example.org' },
      ],
      [
        'matrix_restricted_join_reparent',
        {
          roomId: '!room:example.org',
          removeParentId: '!old:example.org',
          addParentId: '!new:example.org',
        },
      ],
    ],
  );
});
