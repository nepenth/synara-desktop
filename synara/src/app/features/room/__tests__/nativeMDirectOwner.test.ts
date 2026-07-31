import assert from 'node:assert/strict';
import test from 'node:test';
import {
  addRoomToMDirectWithNativeOwner,
  removeRoomFromMDirectWithNativeOwner,
  type NativeInvoke,
} from '../nativeMDirectOwner';

const loggedIn: NativeInvoke = async (command) => {
  if (command === 'matrix_session_snapshot') {
    return { available: true, value: { status: 'logged_in' } };
  }
  if (command === 'matrix_mdirect_add' || command === 'matrix_mdirect_remove') {
    return { available: true, value: { roomId: '!dm:example.org', status: 'updated' } };
  }
  return { available: false };
};

test('native m.direct add requires desktop session', async () => {
  await assert.rejects(
    () => addRoomToMDirectWithNativeOwner('!dm:example.org', '@bob:example.org', false, loggedIn),
    /unavailable/
  );
});

test('native m.direct add invokes matrix_mdirect_add when logged in', async () => {
  const calls: string[] = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push(command);
    if (command === 'matrix_mdirect_add') {
      assert.deepEqual(args, { roomId: '!dm:example.org', userId: '@bob:example.org' });
    }
    return loggedIn(command, args);
  };
  await addRoomToMDirectWithNativeOwner('!dm:example.org', '@bob:example.org', true, invoke);
  assert.deepEqual(calls, ['matrix_session_snapshot', 'matrix_mdirect_add']);
});

test('native m.direct remove invokes matrix_mdirect_remove when logged in', async () => {
  const calls: string[] = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push(command);
    return loggedIn(command, args);
  };
  await removeRoomFromMDirectWithNativeOwner('!dm:example.org', true, invoke);
  assert.deepEqual(calls, ['matrix_session_snapshot', 'matrix_mdirect_remove']);
});
