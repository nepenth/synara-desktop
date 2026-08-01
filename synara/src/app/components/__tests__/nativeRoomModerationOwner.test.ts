import assert from 'node:assert/strict';
import test from 'node:test';
import type { DesktopInvokeResult } from '../../utils/desktop';
import {
  banUserWithNativeOwner,
  inviteUserWithNativeOwner,
  kickUserWithNativeOwner,
  setPowerLevelWithNativeOwner,
  unbanUserWithNativeOwner,
  type NativeInvoke,
} from '../nativeRoomModerationOwner';

const loggedInSession: DesktopInvokeResult<unknown> = {
  available: true,
  value: { status: 'logged_in' },
};

test('native room moderation fails closed before invoking when desktop is unavailable', async () => {
  let invoked = false;
  const invoke: NativeInvoke = async () => {
    invoked = true;
    return { available: true, value: undefined };
  };

  await assert.rejects(
    inviteUserWithNativeOwner('!room:example.org', '@alice:example.org', undefined, false, invoke),
    /native matrix room moderation is unavailable/i
  );
  assert.equal(invoked, false);
});

test('native room moderation fails closed for an unavailable or logged-out session', async () => {
  const calls: string[] = [];
  const invoke: NativeInvoke = async (command) => {
    calls.push(command);
    return command === 'matrix_session_snapshot'
      ? { available: true, value: { status: 'logged_out' } }
      : { available: true, value: undefined };
  };

  await assert.rejects(
    kickUserWithNativeOwner('!room:example.org', '@alice:example.org', 'spam', true, invoke),
    /native matrix room moderation is unavailable/i
  );
  assert.deepEqual(calls, ['matrix_session_snapshot']);

  await assert.rejects(
    banUserWithNativeOwner(
      '!room:example.org',
      '@alice:example.org',
      undefined,
      true,
      async () => ({ available: false })
    ),
    /native matrix room moderation is unavailable/i
  );
});

test('native room moderation invokes the registered Rust mutations with reasons', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    return command === 'matrix_session_snapshot'
      ? loggedInSession
      : { available: true, value: undefined };
  };

  await inviteUserWithNativeOwner(
    '!room:example.org',
    '@alice:example.org',
    '  invited for onboarding  ',
    true,
    invoke
  );
  await kickUserWithNativeOwner('!room:example.org', '@alice:example.org', 'spam', true, invoke);
  await banUserWithNativeOwner('!room:example.org', '@alice:example.org', undefined, true, invoke);
  await unbanUserWithNativeOwner('!room:example.org', '@alice:example.org', true, invoke);
  await setPowerLevelWithNativeOwner('!room:example.org', '@alice:example.org', 50, true, invoke);

  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_invite',
      args: {
        roomId: '!room:example.org',
        userId: '@alice:example.org',
        reason: '  invited for onboarding  ',
      },
    },
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_kick',
      args: { roomId: '!room:example.org', userId: '@alice:example.org', reason: 'spam' },
    },
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_ban',
      args: { roomId: '!room:example.org', userId: '@alice:example.org' },
    },
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_unban',
      args: { roomId: '!room:example.org', userId: '@alice:example.org' },
    },
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_room_set_power_level',
      args: { roomId: '!room:example.org', userId: '@alice:example.org', powerLevel: 50 },
    },
  ]);
});

test('native room moderation validates mutation inputs before IPC', async () => {
  const invoke: NativeInvoke = async () => ({ available: true, value: undefined });

  await assert.rejects(
    unbanUserWithNativeOwner('', '@alice:example.org', true, invoke),
    /native matrix room moderation is unavailable/i
  );
  await assert.rejects(
    setPowerLevelWithNativeOwner('!room:example.org', '@alice:example.org', 1.5, true, invoke),
    /native matrix room moderation is unavailable/i
  );
});
