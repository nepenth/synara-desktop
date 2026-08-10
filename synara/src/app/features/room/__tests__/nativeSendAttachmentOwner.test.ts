import assert from 'node:assert/strict';
import test from 'node:test';

import {
  isNativeMatrixLoggedIn,
  sendAttachmentWithNativeOwner,
  sendAttachmentsWithNativeOwner,
} from '../nativeSendAttachmentOwner';

test('native logged-in session is the sole composer attachment send owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await sendAttachmentWithNativeOwner(
    {
      roomId: '!room:example.org',
      file: {
        filename: 'cat.png',
        mimeType: 'image/png',
        bytes: [1, 2, 3],
      },
      replyTo: '$event:example.org',
      threadRoot: '$root:example.org',
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
          localTxnId: 'attach-txn-1',
          status: 'sent',
        },
      };
    }
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_send_attachment',
      args: {
        roomId: '!room:example.org',
        filename: 'cat.png',
        mimeType: 'image/png',
        bytes: [1, 2, 3],
        replyTo: '$event:example.org',
        threadRoot: '$root:example.org',
      },
    },
  ]);
});

test('web and native logged-out sessions retain the legacy owner', async () => {
  assert.equal(await isNativeMatrixLoggedIn(false, async () => ({ available: false })), false);
  assert.equal(
    await sendAttachmentWithNativeOwner(
      {
        roomId: '!room:example.org',
        file: { filename: 'a.txt', mimeType: 'text/plain', bytes: [97] },
      },
      false,
      async () => {
        throw new Error('invoke should not be called');
      }
    ),
    'legacy'
  );
  assert.equal(
    await sendAttachmentsWithNativeOwner(
      '!room:example.org',
      [{ filename: 'a.txt', mimeType: 'text/plain', bytes: [97] }],
      undefined,
      undefined,
      true,
      async () => ({ available: true, value: { status: 'logged_out' } })
    ),
    'legacy'
  );
});

test('native command failure never falls through to legacy upload/send', async () => {
  await assert.rejects(
    sendAttachmentWithNativeOwner(
      {
        roomId: '!room:example.org',
        file: { filename: 'a.txt', mimeType: 'text/plain', bytes: [97] },
      },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : { available: false }
    ),
    /Native Matrix attachment send is unavailable/
  );
});
