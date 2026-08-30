import assert from 'node:assert/strict';
import test from 'node:test';

import {
  isNativeMatrixLoggedIn,
  sendAttachmentPlanWithNativeOwner,
  sendAttachmentWithNativeOwner,
} from '../nativeSendAttachmentOwner';

test('native logged-in session is the sole composer attachment send owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await sendAttachmentWithNativeOwner(
    {
      roomId: '!room:example.org',
      transactionId: 'synara-attachment-cat',
      file: {
        filename: 'cat.png',
        mimeType: 'image/png',
        bytes: [1, 2, 3],
      },
      caption: 'A cat',
      formattedCaption: '<strong>A cat</strong>',
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
        transactionId: 'synara-attachment-cat',
        caption: 'A cat',
        formattedCaption: '<strong>A cat</strong>',
        mentionUserIds: undefined,
        mentionRoom: undefined,
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
        transactionId: 'synara-attachment-a',
        file: { filename: 'a.txt', mimeType: 'text/plain', bytes: [97] },
      },
      false,
      async () => {
        throw new Error('invoke should not be called');
      }
    ),
    'legacy'
  );
});

test('native command failure never falls through to legacy upload/send', async () => {
  await assert.rejects(
    sendAttachmentWithNativeOwner(
      {
        roomId: '!room:example.org',
        transactionId: 'synara-attachment-failure',
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

test('partial attachment plan reports only completed steps so retry cannot resend them', async () => {
  const sent: number[] = [];
  let attachmentCalls = 0;
  await assert.rejects(
    sendAttachmentPlanWithNativeOwner(
      [
        {
          roomId: '!room:example.org',
          transactionId: 'synara-attachment-one',
          file: { filename: 'one.png', mimeType: 'image/png', bytes: [1] },
        },
        {
          roomId: '!room:example.org',
          transactionId: 'synara-attachment-two',
          file: { filename: 'two.png', mimeType: 'image/png', bytes: [2] },
        },
      ],
      true,
      async (command) => {
        if (command === 'matrix_session_snapshot') {
          return { available: true, value: { status: 'logged_in' } };
        }
        attachmentCalls += 1;
        if (attachmentCalls === 2) {
          return { available: false };
        }
        return {
          available: true,
          value: {
            roomId: '!room:example.org',
            eventId: '$sent:example.org',
            localTxnId: 'txn',
            status: 'sent',
          },
        };
      },
      (index) => {
        sent.push(index);
      }
    ),
    /Native Matrix attachment send is unavailable/
  );

  assert.deepEqual(sent, [0]);
});
