import assert from 'node:assert/strict';
import test from 'node:test';

import {
  acceptsDraftResolution,
  acceptsDraftSyncStatus,
  draftSlotFor,
  isSlateValue,
  resolveSyncedDraft,
  saveSyncedDraft,
  type DraftSyncInvoke,
} from '../nativeDraftSync';

const draft = {
  roomId: '!room:example.org',
  slot: 'main',
  updatedAt: 10,
  deviceId: 'PHONE',
  cleared: false,
  markdown: 'from the phone',
  truncated: false,
};

test('slots mirror the composer draft key', () => {
  assert.equal(draftSlotFor(), 'main');
  assert.equal(draftSlotFor('$root'), 'thread:$root');
});

test('status accepts only the closed key-state vocabulary', () => {
  assert.equal(
    acceptsDraftSyncStatus({ sessionGeneration: 1, enabled: true, keyState: 'needsRecovery' }),
    true
  );
  assert.equal(
    acceptsDraftSyncStatus({ sessionGeneration: 1, enabled: true, keyState: 'unlocked' }),
    false
  );
  assert.equal(acceptsDraftSyncStatus(null), false);
});

test('a resolution that applies or prompts must carry a draft', () => {
  assert.equal(acceptsDraftResolution({ sessionGeneration: 1, decision: 'keep' }), true);
  assert.equal(acceptsDraftResolution({ sessionGeneration: 1, decision: 'apply', draft }), true);
  assert.equal(acceptsDraftResolution({ sessionGeneration: 1, decision: 'prompt' }), false);
  assert.equal(
    acceptsDraftResolution({
      sessionGeneration: 1,
      decision: 'apply',
      draft: { ...draft, markdown: 42 },
    }),
    false
  );
  assert.equal(acceptsDraftResolution({ sessionGeneration: 1, decision: 'merge', draft }), false);
});

test('only editor-shaped Slate values from a peer are used', () => {
  assert.equal(isSlateValue([{ type: 'paragraph', children: [{ text: 'hi' }] }]), true);
  assert.equal(isSlateValue([]), false);
  assert.equal(isSlateValue([{ children: [] }]), false);
  assert.equal(isSlateValue([{ type: 'paragraph' }]), false);
  assert.equal(isSlateValue('paragraph'), false);
});

test('save and resolve pass the composer state through unchanged', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke = (async (command: string, args?: Record<string, unknown>) => {
    calls.push({ command, args });
    if (command === 'matrix_drafts_save') {
      return { available: true, value: { sessionGeneration: 1, updatedAt: 99 } };
    }
    return { available: true, value: { sessionGeneration: 1, decision: 'prompt', draft } };
  }) as DraftSyncInvoke;

  const stamp = await saveSyncedDraft(
    { roomId: draft.roomId, slot: 'main', markdown: 'typing', flush: true },
    invoke
  );
  assert.equal(stamp, 99);
  const resolution = await resolveSyncedDraft(
    draft.roomId,
    'main',
    { updatedAt: 5, dirty: true, markdown: 'typing' },
    true,
    invoke
  );
  assert.equal(resolution?.decision, 'prompt');
  assert.deepEqual(calls[1], {
    command: 'matrix_drafts_resolve',
    args: {
      roomId: draft.roomId,
      slot: 'main',
      local: { updatedAt: 5, dirty: true, markdown: 'typing' },
      fetchFresh: true,
    },
  });
});

test('an unavailable desktop bridge degrades to no sync', async () => {
  const invoke = (async () => ({ available: false })) as DraftSyncInvoke;
  assert.equal(await saveSyncedDraft({ roomId: draft.roomId, slot: 'main' }, invoke), undefined);
  assert.equal(
    await resolveSyncedDraft(draft.roomId, 'main', { dirty: false }, false, invoke),
    undefined
  );
});
