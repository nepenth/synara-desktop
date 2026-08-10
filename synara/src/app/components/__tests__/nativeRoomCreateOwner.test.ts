import assert from 'node:assert/strict';
import test from 'node:test';
import {
  createRoomWithNativeOwner,
  type NativeInvoke,
  type NativeRoomCreateRequest,
} from '../nativeRoomCreateOwner';

const request: NativeRoomCreateRequest = {
  name: 'Native room',
  roomVersion: '11',
  creationContent: { type: 'm.space', federate: false },
  joinRule: 'restricted',
  parentRoomId: '!parent:example.org',
};

test('room create fails closed when desktop is unavailable', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return { available: true, value: { status: 'logged_in' } };
  };

  await assert.rejects(() => createRoomWithNativeOwner(request, false, invoke), /unavailable/);
  assert.equal(invoked, false);
});

test('room create fails closed without a logged-in native session', async () => {
  const calls: string[] = [];
  const invoke: NativeInvoke = async (command) => {
    calls.push(command);
    return { available: true, value: { status: 'logged_out' } };
  };

  await assert.rejects(() => createRoomWithNativeOwner(request, true, invoke), /unavailable/);
  assert.deepEqual(calls, ['matrix_session_snapshot']);
});

test('room create invokes the sole native owner with the SDK-neutral request', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    return command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: true, value: '!created:example.org' };
  };

  const roomId = await createRoomWithNativeOwner(request, true, invoke);

  assert.equal(roomId, '!created:example.org');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    { command: 'matrix_room_create', args: { request } },
  ]);
});

test('room create throws when the native command is unavailable or returns no room id', async () => {
  const missingCommand: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: false };
  await assert.rejects(
    () => createRoomWithNativeOwner(request, true, missingCommand),
    /unavailable/
  );

  const missingRoomId: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: true, value: undefined };
  await assert.rejects(
    () => createRoomWithNativeOwner(request, true, missingRoomId),
    /unavailable/
  );
});
