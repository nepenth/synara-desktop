import assert from 'node:assert/strict';
import test from 'node:test';

import type { GifResult } from '../../../utils/gifProvider';
import { sendGifWithNativeOwner } from '../nativeSendGifOwner';

const sampleGif: GifResult = {
  id: 'gif-1',
  title: 'happy cat',
  url: 'https://cdn.example.org/happy.gif',
  previewUrl: 'https://cdn.example.org/happy-preview.gif',
  width: 200,
  height: 150,
  provider: 'custom',
};

test('native logged-in session owns GIF send via matrix_send_attachment', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const gifBytes = new Uint8Array([0x47, 0x49, 0x46, 0x38]); // GIF8
  const owner = await sendGifWithNativeOwner(
    {
      roomId: '!room:example.org',
      gif: sampleGif,
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
          localTxnId: 'gif-txn-1',
          status: 'sent',
        },
      };
    },
    async () => ({
      blob: new Blob([gifBytes], { type: 'image/gif' }),
      fileName: 'happy_cat.gif',
    })
  );

  assert.equal(owner, 'native');
  assert.equal(calls[0]?.command, 'matrix_session_snapshot');
  // isNativeMatrixLoggedIn + sendAttachmentWithNativeOwner each snapshot once.
  const attachCall = calls.find((c) => c.command === 'matrix_send_attachment');
  assert.ok(attachCall);
  assert.equal(attachCall?.args?.roomId, '!room:example.org');
  assert.equal(attachCall?.args?.filename, 'happy_cat.gif');
  assert.equal(attachCall?.args?.mimeType, 'image/gif');
  assert.equal(attachCall?.args?.replyTo, '$event:example.org');
  assert.equal(attachCall?.args?.threadRoot, '$root:example.org');
  assert.deepEqual(attachCall?.args?.bytes, Array.from(gifBytes));
});

test('web and logged-out sessions retain the legacy GIF owner', async () => {
  assert.equal(
    await sendGifWithNativeOwner(
      { roomId: '!room:example.org', gif: sampleGif },
      false,
      async () => {
        throw new Error('invoke should not be called');
      },
      async () => {
        throw new Error('fetch should not be called');
      }
    ),
    'legacy'
  );
  assert.equal(
    await sendGifWithNativeOwner(
      { roomId: '!room:example.org', gif: sampleGif },
      true,
      async () => ({ available: true, value: { status: 'logged_out' } }),
      async () => {
        throw new Error('fetch should not be called');
      }
    ),
    'legacy'
  );
});

test('native GIF command failure never falls through to legacy upload/send', async () => {
  await assert.rejects(
    sendGifWithNativeOwner(
      { roomId: '!room:example.org', gif: sampleGif },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : { available: false },
      async () => ({
        blob: new Blob([new Uint8Array([1, 2, 3])], { type: 'image/gif' }),
        fileName: 'x.gif',
      })
    ),
    /Native Matrix (attachment|GIF) send is unavailable/
  );
});
