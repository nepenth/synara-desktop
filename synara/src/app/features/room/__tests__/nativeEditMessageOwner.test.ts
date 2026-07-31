import assert from 'node:assert/strict';
import test from 'node:test';

import { editMessageWithNativeOwner } from '../nativeEditMessageOwner';

test('native logged-in session is the sole message-edit owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await editMessageWithNativeOwner(
    {
      roomId: '!room:example.org',
      eventId: '$original:example.org',
      body: 'corrected',
      msgType: 'm.text',
      formattedBody: '<p>corrected</p>',
      mentionUserIds: ['@alice:example.org'],
      mentionRoom: true,
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
          eventId: '$edit:example.org',
          localTxnId: 'txn-1',
          status: 'sent',
        },
      };
    },
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_edit_message',
      args: {
        roomId: '!room:example.org',
        eventId: '$original:example.org',
        body: 'corrected',
        msgType: 'm.text',
        formattedBody: '<p>corrected</p>',
        mentionUserIds: ['@alice:example.org'],
        mentionRoom: true,
        txnId: undefined,
      },
    },
  ]);
});

test('web and native logged-out sessions retain the legacy edit owner', async () => {
  assert.equal(
    await editMessageWithNativeOwner(
      {
        roomId: '!room:example.org',
        eventId: '$original:example.org',
        body: 'corrected',
      },
      false,
      async () => {
        throw new Error('invoke should not be called');
      },
    ),
    'legacy',
  );
});

test('native edit command failure never falls through to legacy sendMessage', async () => {
  await assert.rejects(
    editMessageWithNativeOwner(
      {
        roomId: '!room:example.org',
        eventId: '$original:example.org',
        body: 'corrected',
      },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : { available: false },
    ),
    /Native Matrix message edit is unavailable/,
  );
});

test('native edit non-sent status throws (fail-closed)', async () => {
  await assert.rejects(
    editMessageWithNativeOwner(
      {
        roomId: '!room:example.org',
        eventId: '$original:example.org',
        body: 'corrected',
      },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : {
              available: true,
              value: { roomId: '!room:example.org', eventId: '', localTxnId: '', status: 'failed' },
            },
    ),
    /Native Matrix message edit is unavailable/,
  );
});
