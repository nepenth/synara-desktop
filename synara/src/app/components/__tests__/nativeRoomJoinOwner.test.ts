import assert from 'node:assert/strict';
import test from 'node:test';
import { joinRoomWithNativeOwner, type NativeInvoke } from '../nativeRoomJoinOwner';

const loggedInSession = { available: true as const, value: { status: 'logged_in' } };

test('room join fails closed when desktop is unavailable', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return loggedInSession;
  };

  await assert.rejects(
    () => joinRoomWithNativeOwner('!room:example.org', undefined, false, invoke),
    /unavailable/
  );
  assert.equal(invoked, false);
});

test('room join fails closed without a logged-in native session', async () => {
  const calls: string[] = [];
  const invoke: NativeInvoke = async (command) => {
    calls.push(command);
    return { available: true, value: { status: 'logged_out' } };
  };

  await assert.rejects(
    () => joinRoomWithNativeOwner('#alias:example.org', undefined, true, invoke),
    /unavailable/
  );
  assert.deepEqual(calls, ['matrix_session_snapshot']);
});

test('room join invokes the sole native owner for ids, aliases, and via servers', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    return command === 'matrix_session_snapshot'
      ? loggedInSession
      : { available: true, value: undefined };
  };

  await joinRoomWithNativeOwner(
    '  #alias:example.org  ',
    ['example.org', 'fallback.example.org'],
    true,
    invoke
  );

  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_join',
      args: {
        roomIdOrAlias: '  #alias:example.org  ',
        viaServers: ['example.org', 'fallback.example.org'],
      },
    },
  ]);
});

test('room join throws when the native command is unavailable', async () => {
  const invoke: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot' ? loggedInSession : { available: false };

  await assert.rejects(
    () => joinRoomWithNativeOwner('!room:example.org', [], true, invoke),
    /unavailable/
  );
});

test('room join rejects blank targets before invoking native session state', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return loggedInSession;
  };

  await assert.rejects(() => joinRoomWithNativeOwner('  ', undefined, true, invoke), /unavailable/);
  assert.equal(invoked, false);
});
