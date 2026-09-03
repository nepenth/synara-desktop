import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  decideAgentApprovalWithNativeOwner,
  ensureReactionWithNativeOwner,
  redactReactionWithNativeOwner,
  toggleReactionWithNativeOwner,
  type NativeReactionMutationResult,
} from '../nativeReactionOwner';

test('approval decisions route only through the Core authority command', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const result = await decideAgentApprovalWithNativeOwner(
    {
      roomId: '!room:example.org',
      eventId: '$approval:example.org',
      actionId: 'agent-approval.approve-always',
    },
    async (command, args) => {
      calls.push({ command, args });
      return {
        available: true,
        value: {
          roomId: '!room:example.org',
          eventId: '$approval:example.org',
          status: 'applied',
          reaction: {
            roomId: '!room:example.org',
            targetEventId: '$approval:example.org',
            key: '♾️',
            mutation: 'added',
          },
        },
      };
    }
  );

  assert.equal(result.status, 'applied');
  assert.deepEqual(calls, [
    {
      command: 'matrix_agent_approval_decide',
      args: {
        roomId: '!room:example.org',
        eventId: '$approval:example.org',
        actionId: 'agent-approval.approve-always',
      },
    },
  ]);
});

test('approval card cannot bypass Core through a generic reaction write', () => {
  const source = readFileSync(
    `${process.cwd()}/src/app/components/agent-approval/AgentApprovalCard.tsx`,
    'utf8'
  );
  assert.match(source, /decideAgentApprovalWithNativeOwner/);
  assert.doesNotMatch(source, /ensureReactionWithNativeOwner|matrix_reaction_ensure/);
});

function ok(result: NativeReactionMutationResult) {
  return async () => ({ available: true as const, value: result });
}

test('toggle routes only through matrix_timeline_reaction_toggle with no fallback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const result = await toggleReactionWithNativeOwner(
    {
      roomId: '!room:example.org',
      eventId: '$event:example.org',
      key: '✅',
      expectedOwn: true,
    },
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
        readback: { key: '👍', count: 1, me: true, senders: [] },
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
      {
        roomId: '!room:example.org',
        eventId: '$event:example.org',
        key: '✅',
        expectedOwn: true,
      },
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

test('reaction owners reject mismatched identity, mutation, and projected state', async () => {
  const request = {
    roomId: '!room:example.org',
    eventId: '$event:example.org',
    key: '✅',
    expectedOwn: true,
  };
  const invalid: NativeReactionMutationResult[] = [
    {
      roomId: '!other:example.org',
      targetEventId: request.eventId,
      key: request.key,
      mutation: 'added',
      readback: { key: request.key, count: 1, me: true, senders: [] },
    },
    {
      roomId: request.roomId,
      targetEventId: '$other:example.org',
      key: request.key,
      mutation: 'added',
      readback: { key: request.key, count: 1, me: true, senders: [] },
    },
    {
      roomId: request.roomId,
      targetEventId: request.eventId,
      key: '👎',
      mutation: 'added',
      readback: { key: '👎', count: 1, me: true, senders: [] },
    },
    {
      roomId: request.roomId,
      targetEventId: request.eventId,
      key: request.key,
      mutation: 'redacted',
    },
    {
      roomId: request.roomId,
      targetEventId: request.eventId,
      key: request.key,
      mutation: 'added',
      readback: { key: request.key, count: 1, me: false, senders: [] },
    },
    {
      roomId: request.roomId,
      targetEventId: request.eventId,
      key: request.key,
      mutation: 'removed',
      readback: { key: request.key, count: 1, me: true, senders: [] },
    },
  ];

  for (const value of invalid) {
    await assert.rejects(
      toggleReactionWithNativeOwner(request, async () => ({ available: true, value })),
      /readback did not match/
    );
  }
});

test('toggle accepts committed add without immediate readback but rejects the wrong mutation', async () => {
  const request = {
    roomId: '!room:example.org',
    eventId: '$event:example.org',
    key: '✅',
    expectedOwn: true,
  };
  const added = await toggleReactionWithNativeOwner(request, async () => ({
    available: true,
    value: {
      roomId: request.roomId,
      targetEventId: request.eventId,
      key: request.key,
      mutation: 'added',
    },
  }));
  assert.equal(added.mutation, 'added');

  await assert.rejects(
    toggleReactionWithNativeOwner(request, async () => ({
      available: true,
      value: {
        roomId: request.roomId,
        targetEventId: request.eventId,
        key: request.key,
        mutation: 'removed',
      },
    })),
    /readback did not match/
  );
});
