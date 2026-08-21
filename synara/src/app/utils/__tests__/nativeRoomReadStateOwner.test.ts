import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { setRoomReadStateWithNativeOwner, type NativeInvoke } from '../nativeRoomReadStateOwner';

test('room read state invokes the native owner for a logged-in desktop session', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: { status: 'logged_in' } };
    }
    return { available: true, value: undefined };
  };

  await setRoomReadStateWithNativeOwner('!room:example.org', 'mark_read', true, invoke);

  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_set_read_state',
      args: { roomId: '!room:example.org', action: 'mark_read' },
    },
  ]);
});

test('room unread invokes the native unread-flag command', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: { status: 'logged_in' } };
    }
    return { available: true, value: undefined };
  };

  await setRoomReadStateWithNativeOwner('!room:example.org', 'mark_unread', true, invoke);

  assert.equal(calls[1]?.command, 'matrix_room_set_read_state');
  assert.deepEqual(calls[1]?.args, { roomId: '!room:example.org', action: 'mark_unread' });
});

test('room read state fails closed outside the desktop product', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return { available: true, value: { status: 'logged_in' } };
  };

  await assert.rejects(
    () => setRoomReadStateWithNativeOwner('!room:example.org', 'mark_read', false, invoke),
    /Native Matrix room read state is unavailable/
  );
  assert.equal(invoked, false);
});

test('room read state fails closed when the native session is unavailable or logged out', async () => {
  const loggedOut: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_out' } }
      : { available: true, value: undefined };
  await assert.rejects(
    () => setRoomReadStateWithNativeOwner('!room:example.org', 'mark_read', true, loggedOut),
    /Native Matrix room read state is unavailable/
  );

  const missingCommand: NativeInvoke = async (command) =>
    command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_in' } }
      : { available: false };
  await assert.rejects(
    () => setRoomReadStateWithNativeOwner('!room:example.org', 'mark_unread', true, missingCommand),
    /Native Matrix room read state is unavailable/
  );
});

test('desktop mark-as-read helpers route through the native owner, not the JS-sdk GAP', () => {
  const notifications = readFileSync('src/app/utils/notifications.ts', 'utf8');
  assert.match(notifications, /setRoomReadStateWithNativeOwner/);
  assert.match(notifications, /typeof window !== 'undefined' && isSynaraDesktop\(\)/);
  assert.match(notifications, /mark_read/);
  assert.match(notifications, /mark_unread/);
});

test('room read state does not echo room ids in fail-closed errors', async () => {
  const invoke: NativeInvoke = async () => ({ available: false });
  await assert.rejects(
    () => setRoomReadStateWithNativeOwner('!secret-room:example.org', 'mark_read', true, invoke),
    (error: unknown) => {
      const text = error instanceof Error ? error.message : String(error);
      assert.equal(text.includes('!secret-room:example.org'), false);
      assert.match(text, /unavailable/);
      return true;
    }
  );
});
