import assert from 'node:assert/strict';
import test from 'node:test';

import {
  ensureReactionWithNativeOwner,
  redactReactionWithNativeOwner,
  toggleReactionWithNativeOwner,
  type NativeReactionMutationResult,
} from '../nativeReactionOwner';

function ok(result: NativeReactionMutationResult) {
  return async () => ({ available: true as const, value: result });
}

test('toggle routes only through matrix_timeline_reaction_toggle with no fallback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const result = await toggleReactionWithNativeOwner(
    { roomId: '!room:example.org', eventId: '$event:example.org', key: '✅' },
    async (command, args) => {
      calls.push({ command, args });
      return {
        available: true,
        value: {
          roomId: '!room:example.org',
          targetEventId: '$event:example.org',
          key: '✅',
          mutation: 'added',
          readback: { key: '✅', count: 1, me: true, senders: [{ userId: '@me:example.org' }] },
        },
      };
    }
  );

  assert.equal(result.mutation, 'added');
  assert.deepEqual(calls, [
    {
      command: 'matrix_timeline_reaction_toggle',
      args: {
        roomId: '!room:example.org',
        eventId: '$event:example.org',
        key: '✅',
      },
    },
  ]);
});

test('ensure is a distinct idempotent command, never toggle', async () => {
  const calls: string[] = [];
  const result = await ensureReactionWithNativeOwner(
    { roomId: '!room:example.org', eventId: '$event:example.org', key: '👍' },
    async (command, args) => {
      calls.push(command);
      assert.deepEqual(args, {
        roomId: '!room:example.org',
        eventId: '$event:example.org',
        key: '👍',
      });
      return ok({
        roomId: '!room:example.org',
        targetEventId: '$event:example.org',
        key: '👍',
        mutation: 'already_present',
      })();
    }
  );

  assert.equal(result.mutation, 'already_present');
  assert.deepEqual(calls, ['matrix_reaction_ensure']);
});

test('redact binds the selected annotation event id for the viewer path', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const result = await redactReactionWithNativeOwner(
    {
      roomId: '!room:example.org',
      eventId: '$target:example.org',
      reactionEventId: '$annotation:example.org',
      key: '✅',
    },
    async (command, args) => {
      calls.push({ command, args });
      return ok({
        roomId: '!room:example.org',
        targetEventId: '$target:example.org',
        key: '✅',
        mutation: 'redacted',
      })();
    }
  );

  assert.equal(result.mutation, 'redacted');
  assert.deepEqual(calls, [
    {
      command: 'matrix_reaction_redact',
      args: {
        roomId: '!room:example.org',
        targetEventId: '$target:example.org',
        reactionEventId: '$annotation:example.org',
        key: '✅',
      },
    },
  ]);
});

test('native reaction command failure never invents a JS SDK fallback', async () => {
  await assert.rejects(
    toggleReactionWithNativeOwner(
      { roomId: '!room:example.org', eventId: '$event:example.org', key: '✅' },
      async () => ({ available: false })
    ),
    /Native Matrix reactions are unavailable/
  );
  await assert.rejects(
    ensureReactionWithNativeOwner(
      { roomId: '!room:example.org', eventId: '$event:example.org', key: '✅' },
      async () => ({ available: true, value: undefined })
    ),
    /Native Matrix reactions are unavailable/
  );
});
