import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  isNativeMatrixLoggedIn,
  sendAttachmentPlanWithNativeOwner,
  sendAttachmentWithNativeOwner,
  type NativeInvoke,
} from '../nativeSendAttachmentOwner';

const sent = {
  available: true as const,
  value: {
    roomId: '!room:example.org',
    eventId: '$sent:example.org',
    localTxnId: 'attach-txn-1',
    status: 'sent',
  },
};

/** A logged-in native session that stages bytes and sends. */
const nativeInvoke =
  (
    calls: Array<{ command: string; args?: Record<string, unknown> | Uint8Array }>,
    send: () => Awaited<ReturnType<NativeInvoke>> = () => sent
  ): NativeInvoke =>
  async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: { status: 'logged_in' } };
    }
    if (command === 'matrix_attachment_stage') {
      return { available: true, value: `staged-${calls.length}` };
    }
    return send();
  };

test('native logged-in session is the sole composer attachment send owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> | Uint8Array }> = [];
  const bytes = new Uint8Array([1, 2, 3]);
  const owner = await sendAttachmentWithNativeOwner(
    {
      roomId: '!room:example.org',
      transactionId: 'synara-attachment-cat',
      file: {
        filename: 'cat.png',
        mimeType: 'image/png',
        bytes,
      },
      caption: 'A cat',
      formattedCaption: '<strong>A cat</strong>',
      replyTo: '$event:example.org',
      threadRoot: '$root:example.org',
    },
    true,
    nativeInvoke(calls)
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    // The bytes are the raw IPC body itself, not a JSON field.
    { command: 'matrix_attachment_stage', args: bytes },
    {
      command: 'matrix_send_attachment',
      args: {
        roomId: '!room:example.org',
        filename: 'cat.png',
        mimeType: 'image/png',
        stagedId: 'staged-2',
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
  assert.ok(calls[1]?.args instanceof Uint8Array);
  assert.ok(!('bytes' in (calls[2]?.args as Record<string, unknown>)));
});

test('attachment bytes never become a JSON number array', () => {
  const source = readFileSync('src/app/features/room/nativeSendAttachment.ts', 'utf8');
  assert.ok(!source.includes('Array.from(new Uint8Array'));
  const gif = readFileSync('src/app/features/room/nativeSendGifOwner.ts', 'utf8');
  assert.ok(!gif.includes('Array.from(new Uint8Array'));
});

test('web and native logged-out sessions retain the legacy owner', async () => {
  assert.equal(await isNativeMatrixLoggedIn(false, async () => ({ available: false })), false);
  assert.equal(
    await sendAttachmentWithNativeOwner(
      {
        roomId: '!room:example.org',
        transactionId: 'synara-attachment-a',
        file: { filename: 'a.txt', mimeType: 'text/plain', bytes: new Uint8Array([97]) },
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
        file: { filename: 'a.txt', mimeType: 'text/plain', bytes: new Uint8Array([97]) },
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
  const sentSteps: number[] = [];
  let attachmentCalls = 0;
  const calls: Array<{ command: string; args?: Record<string, unknown> | Uint8Array }> = [];
  await assert.rejects(
    sendAttachmentPlanWithNativeOwner(
      [
        {
          roomId: '!room:example.org',
          transactionId: 'synara-attachment-one',
          file: { filename: 'one.png', mimeType: 'image/png', bytes: new Uint8Array([1]) },
        },
        {
          roomId: '!room:example.org',
          transactionId: 'synara-attachment-two',
          file: { filename: 'two.png', mimeType: 'image/png', bytes: new Uint8Array([2]) },
        },
      ],
      true,
      nativeInvoke(calls, () => {
        attachmentCalls += 1;
        return attachmentCalls === 2 ? { available: false } : sent;
      }),
      (index) => {
        sentSteps.push(index);
      }
    ),
    /Native Matrix attachment send is unavailable/
  );

  assert.deepEqual(sentSteps, [0]);
});

test('an attachment the native send queue still holds is accepted, not re-attached', async () => {
  const owner = await sendAttachmentWithNativeOwner(
    {
      roomId: '!room:example.org',
      transactionId: 'synara-attachment-queued',
      file: { filename: 'cat.png', mimeType: 'image/png', bytes: new Uint8Array([1]) },
    },
    true,
    nativeInvoke([], () => ({
      available: true,
      value: {
        roomId: '!room:example.org',
        eventId: '',
        localTxnId: 'txn-queued',
        status: 'queued',
      },
    }))
  );
  assert.equal(owner, 'native');
});
