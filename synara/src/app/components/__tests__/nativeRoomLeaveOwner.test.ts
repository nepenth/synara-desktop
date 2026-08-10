import assert from 'node:assert/strict';
import { test } from 'node:test';
import { leaveRoomWithNativeOwner, type NativeInvoke } from '../nativeRoomLeaveOwner';

test('room leave invokes the native command for a logged-in desktop session', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: { status: 'logged_in' } };
    }
    return { available: true, value: undefined };
  };

  await leaveRoomWithNativeOwner('!room:example.org', true, invoke);

  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    { command: 'matrix_room_leave', args: { roomId: '!room:example.org' } },
  ]);
});

test('room leave fails closed outside the desktop product', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return { available: true, value: { status: 'logged_in' } };
  };

  await assert.rejects(
    () => leaveRoomWithNativeOwner('!room:example.org', false, invoke),
    /Native Matrix room leave is unavailable/
  );
  assert.equal(invoked, false);
});

test('room leave fails closed when the native session is unavailable or logged out', async () => {
  const loggedOut: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_out' } }
      : { available: true, value: undefined };
  await assert.rejects(
    () => leaveRoomWithNativeOwner('!room:example.org', true, loggedOut),
    /Native Matrix room leave is unavailable/
  );

  const missingCommand: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: false };
  await assert.rejects(
    () => leaveRoomWithNativeOwner('!room:example.org', true, missingCommand),
    /Native Matrix room leave is unavailable/
  );
});
