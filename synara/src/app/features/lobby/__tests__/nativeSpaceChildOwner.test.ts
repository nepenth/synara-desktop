import assert from 'node:assert/strict';
import test from 'node:test';
import {
  removeSpaceChildWithNativeOwner,
  setRoomJoinRulesWithNativeOwner,
  setSpaceChildWithNativeOwner,
  type NativeInvoke,
} from '../nativeSpaceChildOwner';

const loggedIn: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (
    command === 'matrix_space_child_set' ||
    command === 'matrix_space_child_remove' ||
    command === 'matrix_room_join_rules_set'
  ) {
    return {
      available: true,
      value:
        command === 'matrix_room_join_rules_set'
          ? { roomId: '!room:example.org', status: 'updated' }
          : { parentId: '!space:example.org', childId: '!room:example.org', status: 'updated' },
    };
  }
  return { available: false };
};

test('native space child set requires desktop session', async () => {
  await assert.rejects(
    () =>
      setSpaceChildWithNativeOwner(
        {
          parentId: '!space:example.org',
          childId: '!room:example.org',
          via: ['example.org'],
        },
        false,
        loggedIn
      ),
    /unavailable/
  );
});

test('native space child set invokes matrix_space_child_set when logged in', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    return loggedIn(command, args);
  };
  await setSpaceChildWithNativeOwner(
    {
      parentId: '!space:example.org',
      childId: '!room:example.org',
      via: ['example.org'],
      order: 'a0',
      suggested: true,
    },
    true,
    invoke
  );
  assert.deepEqual(calls, [
    ['matrix_session_snapshot', undefined],
    [
      'matrix_space_child_set',
      {
        parentId: '!space:example.org',
        childId: '!room:example.org',
        via: ['example.org'],
        order: 'a0',
        suggested: true,
      },
    ],
  ]);
});

test('native space child remove invokes matrix_space_child_remove when logged in', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    return loggedIn(command, args);
  };
  await removeSpaceChildWithNativeOwner('!space:example.org', '!room:example.org', true, invoke);
  assert.deepEqual(calls, [
    ['matrix_session_snapshot', undefined],
    [
      'matrix_space_child_remove',
      { parentId: '!space:example.org', childId: '!room:example.org' },
    ],
  ]);
});

test('native join rules set invokes matrix_room_join_rules_set when logged in', async () => {
  const calls: Array<[string, Record<string, unknown> | undefined]> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push([command, args]);
    return loggedIn(command, args);
  };
  await setRoomJoinRulesWithNativeOwner(
    {
      roomId: '!room:example.org',
      joinRule: 'restricted',
      allow: [{ roomId: '!space:example.org' }],
    },
    true,
    invoke
  );
  assert.deepEqual(calls, [
    ['matrix_session_snapshot', undefined],
    [
      'matrix_room_join_rules_set',
      {
        roomId: '!room:example.org',
        joinRule: 'restricted',
        allow: [{ type: 'm.room_membership', roomId: '!space:example.org' }],
      },
    ],
  ]);
});
