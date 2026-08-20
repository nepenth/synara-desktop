import assert from 'node:assert/strict';
import { test } from 'node:test';
import { setRoomFavoriteWithNativeOwner, type NativeInvoke } from '../nativeRoomFavoriteOwner';

test('room favorite invokes the native tag command for a logged-in desktop session', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: { status: 'logged_in' } };
    }
    return { available: true, value: undefined };
  };

  await setRoomFavoriteWithNativeOwner('!room:example.org', true, true, invoke);

  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_set_favorite',
      args: { roomId: '!room:example.org', favorite: true },
    },
  ]);
});

test('room favorite fails closed outside the desktop product', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return { available: true, value: { status: 'logged_in' } };
  };

  await assert.rejects(
    () => setRoomFavoriteWithNativeOwner('!room:example.org', true, false, invoke),
    /Native Matrix room favorite is unavailable/
  );
  assert.equal(invoked, false);
});

test('room favorite fails closed when the native session is unavailable or logged out', async () => {
  const loggedOut: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_out' } }
      : { available: true, value: undefined };
  await assert.rejects(
    () => setRoomFavoriteWithNativeOwner('!room:example.org', true, true, loggedOut),
    /Native Matrix room favorite is unavailable/
  );

  const missingCommand: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: false };
  await assert.rejects(
    () => setRoomFavoriteWithNativeOwner('!room:example.org', false, true, missingCommand),
    /Native Matrix room favorite is unavailable/
  );
});
