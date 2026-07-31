import assert from 'node:assert/strict';
import test from 'node:test';

import { sendStickerWithNativeOwner } from '../nativeSendStickerOwner';

test('native logged-in session is the sole composer sticker send owner', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const owner = await sendStickerWithNativeOwner(
    {
      roomId: '!room:example.org',
      body: 'cat',
      mxc: 'mxc://example.org/sticker1',
      info: {
        width: 128,
        height: 128,
        mimetype: 'image/png',
        size: 2048,
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
          status: 'sent',
        },
      };
    }
  );

  assert.equal(owner, 'native');
  assert.deepEqual(calls, [
    { command: 'matrix_session_snapshot', args: undefined },
    {
      command: 'matrix_send_sticker',
      args: {
        roomId: '!room:example.org',
        body: 'cat',
        mxc: 'mxc://example.org/sticker1',
        width: 128,
        height: 128,
        mimetype: 'image/png',
        size: 2048,
        replyTo: '$event:example.org',
        threadRoot: '$root:example.org',
      },
    },
  ]);
});

test('web and native logged-out sessions retain the legacy sticker owner', async () => {
  assert.equal(
    await sendStickerWithNativeOwner(
      {
        roomId: '!room:example.org',
        body: 'cat',
        mxc: 'mxc://example.org/s',
      },
      false,
      async () => {
        throw new Error('invoke should not be called');
      }
    ),
    'legacy'
  );
  assert.equal(
    await sendStickerWithNativeOwner(
      {
        roomId: '!room:example.org',
        body: 'cat',
        mxc: 'mxc://example.org/s',
      },
      true,
      async () => ({ available: true, value: { status: 'logged_out' } })
    ),
    'legacy'
  );
});

test('native sticker command failure never falls through to legacy sendEvent', async () => {
  await assert.rejects(
    sendStickerWithNativeOwner(
      {
        roomId: '!room:example.org',
        body: 'cat',
        mxc: 'mxc://example.org/s',
      },
      true,
      async (command) =>
        command === 'matrix_session_snapshot'
          ? { available: true, value: { status: 'logged_in' } }
          : { available: false }
    ),
    /Native Matrix sticker send is unavailable/
  );
});
