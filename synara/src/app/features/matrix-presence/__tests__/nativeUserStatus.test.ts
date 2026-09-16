import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearOwnUserStatusNative,
  parseInCallField,
  parseUserStatusField,
  parseUserStatusSnapshot,
  parseUserStatusWriteAck,
  setOwnUserStatusNative,
  snapshotUserStatusNative,
  UserStatusWriteError,
  type NativeUserStatusInvoke,
} from '../nativeUserStatus';

test('status parser maps emoji/text and in-call without presence vocab', () => {
  const snapshot = parseUserStatusSnapshot({
    sessionGeneration: 4,
    userId: '@alice:example.org',
    userStatus: { emoji: '☕', text: 'in a meeting' },
    inCall: { callJoinedTs: 1_720_000_000 },
  });
  assert.equal(snapshot?.userStatus?.emoji, '☕');
  assert.equal(snapshot?.userStatus?.text, 'in a meeting');
  assert.equal(snapshot?.inCall?.callJoinedTs, 1_720_000_000);
});

test('status parser fail-closes on unknown keys and presence state mix', () => {
  assert.equal(
    parseUserStatusSnapshot({
      sessionGeneration: 1,
      userId: '@alice:example.org',
      state: 'online',
    }),
    null
  );
  assert.equal(
    parseUserStatusSnapshot({
      sessionGeneration: 1,
      userId: '@alice:example.org',
      userStatus: { emoji: '☕', text: 'ok', state: 'away' },
    }),
    null
  );
  assert.equal(parseUserStatusField({ emoji: '☕', text: 'ok', statusMsg: 'nope' }), null);
  assert.equal(parseInCallField({ callJoinedTs: 1, currentlyActive: true }), null);
  assert.equal(
    parseUserStatusSnapshot({
      sessionGeneration: 1,
      userId: '@alice:example.org',
      widget: true,
    }),
    null
  );
});

test('status parser drops oversize emoji/text and empty status', () => {
  assert.equal(parseUserStatusField({ emoji: '😀'.repeat(33), text: 'ok' }), null);
  assert.equal(parseUserStatusField({ emoji: '☕', text: 'x'.repeat(257) }), null);
  assert.equal(parseUserStatusField({ emoji: '', text: '' }), null);
  assert.equal(parseUserStatusWriteAck({ status: 'ok', emoji: '☕' }), false);
  assert.equal(parseUserStatusWriteAck({ status: 'ok' }), true);
});

test('snapshot and writes stay on native commands and never echo secrets', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeUserStatusInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_user_status_snapshot') {
      return {
        available: true,
        value: {
          sessionGeneration: 2,
          userId: '@alice:example.org',
          inCall: {},
        },
      };
    }
    return { available: true, value: { status: 'ok' } };
  };
  const snapshot = await snapshotUserStatusNative('@alice:example.org', {
    desktopNativeSession: true,
    invoke,
  });
  assert.equal(snapshot?.inCall !== undefined, true);
  await setOwnUserStatusNative('☕', 'in a meeting', {
    desktopNativeSession: true,
    invoke,
  });
  await clearOwnUserStatusNative({ desktopNativeSession: true, invoke });
  assert.deepEqual(
    calls.map((row) => row.command),
    [
      'matrix_user_status_snapshot',
      'matrix_user_status_set',
      'matrix_user_status_clear',
    ]
  );
  assert.deepEqual(calls[1]?.args, { emoji: '☕', text: 'in a meeting' });
});

test('capability-missing write fails closed without echoing status text', async () => {
  const invoke: NativeUserStatusInvoke = async () => {
    throw {
      message: 'This homeserver does not support user status.',
      diagnosticId: 'v-user-status-unsupported',
    };
  };
  await assert.rejects(
    () =>
      setOwnUserStatusNative('☕', 'secret-status-text', {
        desktopNativeSession: true,
        invoke,
      }),
    (error: unknown) => {
      assert.ok(error instanceof UserStatusWriteError);
      assert.equal(error.kind, 'unsupported');
      assert.equal(error.message.includes('secret-status-text'), false);
      return true;
    }
  );
});
