import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearReplyDraftWithNativeComposerOwner,
  mapNativeReplyDraftToJs,
  NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
  setReplyDraftWithNativeComposerOwner,
} from '../nativeComposerDraftOwner';

test('setReplyDraftWithNativeComposerOwner invokes typed command and accepts readback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const readback = await setReplyDraftWithNativeComposerOwner(
    { roomId: '!room:example.org', eventId: '$evt:example.org', startThread: true },
    true,
    async (command, args) => {
      calls.push({ command, args });
      return {
        available: true,
        value: {
          schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
          roomId: '!room:example.org',
          status: 'set',
          draft: {
            eventId: '$evt:example.org',
            senderId: '@alice:example.org',
            body: 'hello',
            formattedBody: '<p>hello</p>',
            threadRootEventId: '$evt:example.org',
          },
        },
      };
    }
  );

  assert.deepEqual(calls, [
    {
      command: 'matrix_composer_set_reply_draft',
      args: {
        request: {
          roomId: '!room:example.org',
          eventId: '$evt:example.org',
          startThread: true,
        },
      },
    },
  ]);
  assert.notEqual(readback, 'unavailable');
  if (readback === 'unavailable') return;
  assert.equal(readback.status, 'set');
  assert.equal(readback.draft?.threadRootEventId, '$evt:example.org');
});

test('clearReplyDraftWithNativeComposerOwner returns cleared status', async () => {
  const readback = await clearReplyDraftWithNativeComposerOwner(
    { roomId: '!room:example.org' },
    true,
    async () => ({
      available: true,
      value: {
        schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
        roomId: '!room:example.org',
        status: 'cleared',
      },
    })
  );
  assert.notEqual(readback, 'unavailable');
  if (readback === 'unavailable') return;
  assert.equal(readback.status, 'cleared');
  assert.equal(readback.draft, undefined);
});

test('mapNativeReplyDraftToJs preserves thread relation for composer content', () => {
  assert.deepEqual(
    mapNativeReplyDraftToJs({
      eventId: '$evt:example.org',
      senderId: '@alice:example.org',
      body: 'hello',
      threadRootEventId: '$root:example.org',
    }),
    {
      userId: '@alice:example.org',
      eventId: '$evt:example.org',
      body: 'hello',
      formattedBody: undefined,
      relation: { rel_type: 'm.thread', event_id: '$root:example.org' },
    }
  );
});

test('setReplyDraftWithNativeComposerOwner is unavailable off desktop', async () => {
  assert.equal(
    await setReplyDraftWithNativeComposerOwner(
      { roomId: '!room:example.org', eventId: '$evt:example.org' },
      false,
      async () => {
        throw new Error('should not invoke');
      }
    ),
    'unavailable'
  );
});
