import assert from 'node:assert/strict';
import test from 'node:test';
import {
  enableRoomEncryptedStateWithNativeOwner,
  sendLeftoverStateEvent,
  sendStateEventWithNativeOwner,
  type NativeStateEventInvoke,
} from '../nativeStateEventOwner';

test('leftover state send fails closed when native is unavailable', async () => {
  let invoked = false;
  const invoke: NativeStateEventInvoke = async () => {
    invoked = true;
    return { available: true, value: { status: 'ok' } };
  };
  await assert.rejects(
    () =>
      sendStateEventWithNativeOwner(
        '!r:example.org',
        'm.room.canonical_alias',
        { alias: '#r:example.org' },
        '',
        invoke,
        false
      ),
    /unavailable/
  );
  assert.equal(invoked, false);
});

test('leftover state send invokes the native owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeStateEventInvoke = async (command, args) => {
    calls.push({ command, args });
    return { available: true, value: { status: 'ok' } };
  };
  await sendStateEventWithNativeOwner(
    '!r:example.org',
    'm.room.canonical_alias',
    { alias: '#r:example.org' },
    '',
    invoke,
    true
  );
  assert.deepEqual(calls, [
    {
      command: 'matrix_send_state_event',
      args: {
        roomId: '!r:example.org',
        eventType: 'm.room.canonical_alias',
        stateKey: '',
        content: { alias: '#r:example.org' },
      },
    },
  ]);
});

test('leftover helper prefers native and never falls through to JS', async () => {
  let js = 0;
  const calls: string[] = [];
  const invoke: NativeStateEventInvoke = async (command) => {
    calls.push(command);
    return { available: true, value: { status: 'ok' } };
  };
  await sendLeftoverStateEvent(
    '!r:example.org',
    'm.room.server_acl',
    { allow: ['*'] },
    '',
    async () => {
      js += 1;
    },
    true,
    invoke
  );
  assert.equal(js, 0);
  assert.deepEqual(calls, ['matrix_send_state_event']);
});

test('leftover helper uses JS only when native is not live', async () => {
  let js = 0;
  const invoke: NativeStateEventInvoke = async () => {
    throw new Error('must not invoke native');
  };
  await sendLeftoverStateEvent(
    '!r:example.org',
    'm.room.server_acl',
    { allow: ['*'] },
    '',
    async () => {
      js += 1;
    },
    false,
    invoke
  );
  assert.equal(js, 1);
});

test('encrypted-state opt-in fails closed when native is unavailable', async () => {
  await assert.rejects(
    () =>
      enableRoomEncryptedStateWithNativeOwner(
        '!r:example.org',
        true,
        async () => {
          return { available: true, value: { status: 'ok' } };
        },
        false
      ),
    /unavailable/
  );
});
