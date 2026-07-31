import assert from 'node:assert/strict';
import test from 'node:test';

import { sendTextWithNativeOwner } from '../nativeSendTextOwner';

test('native logged-in session owns rich composer message sends', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await sendTextWithNativeOwner(
    {
      roomId: '!room:example.org',
      body: 'hello',
      msgType: 'm.emote',
      formattedBody: '<strong>hello</strong>',
      mentionUserIds: ['@alice:example.org'],
      mentionRoom: true,
      replyTo: '$event:example.org',
    },
    true,
    async (command, args) => {
      calls.push({ command, args });
      if (command === 'matrix_session_snapshot') {
        return { available: true, value: { status: 'logged_in' } };
      }
      return {
        available: true,
        value: {
          roomId: '!room:example.org',
          eventId: '$sent:example.org',
          localTxnId: 'local-txn-1',
          status: 'sent',
        },
      };
    }
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_send_text',
      args: {
        roomId: '!room:example.org',
        body: 'hello',
        msgType: 'm.emote',
        formattedBody: '<strong>hello</strong>',
        mentionUserIds: ['@alice:example.org'],
        mentionRoom: true,
        replyTo: '$event:example.org',
      },
    },
  ]);
});

test('web and native logged-out sessions retain the legacy owner', async () => {
  const neverInvoke = async () => {
    throw new Error('invoke should not be called');
  };
  assert.equal(
    await sendTextWithNativeOwner(
      { roomId: '!room:example.org', body: 'hello' },
      false,
      neverInvoke
    ),
    'legacy'
  );

  assert.equal(
    await sendTextWithNativeOwner(
      { roomId: '!room:example.org', body: 'hello' },
      true,
      async () => ({ available: true, value: { status: 'logged_out' } })
    ),
    'legacy'
  );
});

test('native command failure never falls through to legacy send', async () => {
  await assert.rejects(
    sendTextWithNativeOwner({ roomId: '!room:example.org', body: 'hello' }, true, async (command) =>
      command === 'matrix_session_snapshot'
        ? { available: true, value: { status: 'logged_in' } }
        : { available: false }
    ),
    /Native Matrix text send is unavailable/
  );
});
