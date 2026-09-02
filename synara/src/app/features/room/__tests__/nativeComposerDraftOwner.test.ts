import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearReplyDraftWithNativeComposerOwner,
  getReplyDraftWithNativeComposerOwner,
  nativeComposerSendRelation,
  NativeComposerReplyDraftProjection,
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
            draftRevision: 1,
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
    { roomId: '!room:example.org', expectedDraftRevision: 1 },
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

test('send-time compare-and-clear preserves a threaded draft for the same event selected while send is delayed', async () => {
  const roomId = '!room:example.org';
  const sentEventId = '$sent:example.org';
  let currentDraft = {
    draftRevision: 1,
    eventId: sentEventId,
    senderId: '@alice:example.org',
    body: 'original target',
    threadRootEventId: undefined as string | undefined,
  };
  let finishSend: (() => void) | undefined;
  const sendCompleted = new Promise<void>((resolve) => {
    finishSend = resolve;
  });
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke = async (command: string, args?: Record<string, unknown>) => {
    calls.push({ command, args });
    const expectedDraftRevision = (args?.request as { expectedDraftRevision: number })
      .expectedDraftRevision;
    if (currentDraft.draftRevision === expectedDraftRevision) {
      return {
        available: true as const,
        value: {
          schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
          roomId,
          status: 'cleared',
        },
      };
    }
    return {
      available: true as const,
      value: {
        schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
        roomId,
        status: 'set',
        draft: currentDraft,
      },
    };
  };

  const clearAfterSend = (async () => {
    await sendCompleted;
    return clearReplyDraftWithNativeComposerOwner(
      { roomId, expectedDraftRevision: 1 },
      true,
      invoke
    );
  })();

  currentDraft = {
    draftRevision: 2,
    eventId: sentEventId,
    senderId: '@bob:example.org',
    body: 'same target, now threaded',
    threadRootEventId: sentEventId,
  };
  finishSend?.();

  const readback = await clearAfterSend;
  assert.notEqual(readback, 'unavailable');
  if (readback === 'unavailable') return;
  assert.equal(readback.status, 'set');
  assert.equal(readback.draft?.eventId, sentEventId);
  assert.equal(readback.draft?.threadRootEventId, sentEventId);
  assert.equal(currentDraft.draftRevision, 2);
  assert.deepEqual(calls, [
    {
      command: 'matrix_composer_clear_reply_draft',
      args: { request: { roomId, expectedDraftRevision: 1 } },
    },
  ]);
});

test('delayed manual cancellation preserves a repeated selection with a newer Core revision', async () => {
  const roomId = '!room:example.org';
  const eventId = '$same:example.org';
  let currentDraft = {
    draftRevision: 10,
    eventId,
    senderId: '@alice:example.org',
    body: 'same target',
  };
  let releaseClear: (() => void) | undefined;
  let clearStarted: (() => void) | undefined;
  const started = new Promise<void>((resolve) => {
    clearStarted = resolve;
  });
  const blocked = new Promise<void>((resolve) => {
    releaseClear = resolve;
  });
  const invoke = async (_command: string, args?: Record<string, unknown>) => {
    clearStarted?.();
    await blocked;
    const expectedDraftRevision = (args?.request as { expectedDraftRevision: number })
      .expectedDraftRevision;
    if (currentDraft.draftRevision === expectedDraftRevision) {
      return {
        available: true as const,
        value: {
          schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
          roomId,
          status: 'cleared',
        },
      };
    }
    return {
      available: true as const,
      value: {
        schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
        roomId,
        status: 'set',
        draft: currentDraft,
      },
    };
  };

  const cancellation = clearReplyDraftWithNativeComposerOwner(
    { roomId, expectedDraftRevision: currentDraft.draftRevision },
    true,
    invoke
  );
  await started;
  currentDraft = { ...currentDraft, draftRevision: 11 };
  releaseClear?.();

  const readback = await cancellation;
  assert.notEqual(readback, 'unavailable');
  if (readback === 'unavailable') return;
  assert.equal(readback.status, 'set');
  assert.equal(readback.draft?.draftRevision, 11);
  assert.equal(currentDraft.draftRevision, 11);
});

test('nativeComposerSendRelation preserves selected event and thread root', () => {
  assert.deepEqual(
    nativeComposerSendRelation({
      draftRevision: 1,
      eventId: '$evt:example.org',
      senderId: '@alice:example.org',
      body: 'hello',
      threadRootEventId: '$root:example.org',
    }),
    {
      draftRevision: 1,
      replyTo: '$evt:example.org',
      threadRoot: '$root:example.org',
    }
  );
  assert.deepEqual(
    nativeComposerSendRelation({
      draftRevision: 2,
      eventId: '$plain:example.org',
      senderId: '@alice:example.org',
      body: 'hello',
    }),
    { draftRevision: 2, replyTo: '$plain:example.org', threadRoot: undefined }
  );
  assert.deepEqual(nativeComposerSendRelation(undefined), {
    draftRevision: undefined,
    replyTo: undefined,
    threadRoot: undefined,
  });
});

test('reply projection changes only through typed Core readbacks and clears deterministically', () => {
  const projection = new NativeComposerReplyDraftProjection();
  const roomId = '!room:example.org';
  let changes = 0;
  const unsubscribe = projection.subscribe(roomId, () => {
    changes += 1;
  });
  const draft = {
    draftRevision: 1,
    eventId: '$evt:example.org',
    senderId: '@alice:example.org',
    body: 'hello',
  };

  projection.apply({
    schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
    roomId,
    status: 'set',
    draft,
  });
  assert.equal(projection.get(roomId), draft);
  assert.equal(changes, 1);

  projection.apply({
    schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
    roomId,
    status: 'cleared',
  });
  assert.equal(projection.get(roomId), undefined);
  assert.equal(changes, 2);

  unsubscribe();
  projection.apply({
    schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
    roomId,
    status: 'empty',
  });
  assert.equal(changes, 2);
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

test('reply owner rejects invalid readbacks while accepting a superseding Core revision', async () => {
  const setReadback = {
    schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
    roomId: '!other:example.org',
    status: 'set',
    draft: {
      draftRevision: 2,
      eventId: '$other:example.org',
      senderId: '@alice:example.org',
      body: 'hello',
    },
  };
  assert.equal(
    await setReplyDraftWithNativeComposerOwner(
      { roomId: '!room:example.org', eventId: '$evt:example.org' },
      true,
      async () => ({ available: true, value: setReadback })
    ),
    'unavailable'
  );
  const superseding = await clearReplyDraftWithNativeComposerOwner(
    { roomId: '!room:example.org', expectedDraftRevision: 1 },
    true,
    async () => ({
      available: true,
      value: { ...setReadback, roomId: '!room:example.org' },
    })
  );
  assert.notEqual(superseding, 'unavailable');
  if (superseding !== 'unavailable') {
    assert.equal(superseding.draft?.draftRevision, 2);
  }
  assert.equal(
    await clearReplyDraftWithNativeComposerOwner(
      { roomId: '!room:example.org', expectedDraftRevision: 2 },
      true,
      async () => ({
        available: true,
        value: { ...setReadback, roomId: '!room:example.org' },
      })
    ),
    'unavailable',
    'Core must not report the consumed revision as still set'
  );
  assert.equal(
    await getReplyDraftWithNativeComposerOwner({ roomId: '!room:example.org' }, true, async () => ({
      available: true,
      value: {
        schemaVersion: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
        roomId: '!room:example.org',
        status: 'cleared',
      },
    })),
    'unavailable'
  );
});
