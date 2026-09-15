import assert from 'node:assert/strict';
import test from 'node:test';
import { createApprovalInboxProjection } from '../approvalInboxProjection';
import { loadApprovalInbox, type ApprovalInboxSnapshot } from '../nativeApprovalInbox';
import {
  activateApprovalDecisionScope,
  subscribeApprovalDecisions,
} from '../approvalDecisionEvents';
import { decideAgentApprovalWithNativeOwner } from '../../room/nativeReactionOwner';

const snapshot = (sessionGeneration: number): ApprovalInboxSnapshot => ({
  sessionGeneration,
  coverageWindowMs: 300000,
  coverage: 'discovery',
  loading: false,
  incomplete: false,
  items: [
    {
      roomId: '!room:example.org',
      eventId: '$event',
      sender: '@hermes:example.org',
      body: 'Approval required',
      originServerTs: 1000,
      expiresAt: 301000,
      status: 'pending',
      canSendReaction: true,
      bodyTruncated: false,
    },
  ],
});

test('late native decision cannot contaminate the replacement snapshot or its future reads', async () => {
  const client = {};
  const projection = createApprovalInboxProjection();
  projection.receive(
    await loadApprovalInbox(async () => ({ available: true, value: snapshot(1) })),
    client
  );
  const releaseFirst = activateApprovalDecisionScope(projection.scope);
  const applied: boolean[] = [];
  const unsubscribe = subscribeApprovalDecisions((notice) =>
    applied.push(projection.complete(notice.scope, notice))
  );
  let finish!: () => void;
  const pending = decideAgentApprovalWithNativeOwner(
    { roomId: '!room:example.org', eventId: '$event', actionId: 'agent-approval.deny' },
    async () => {
      await new Promise<void>((resolve) => {
        finish = resolve;
      });
      return {
        available: true,
        value: { roomId: '!room:example.org', eventId: '$event', status: 'applied' },
      };
    }
  );
  projection.receive(
    await loadApprovalInbox(async () => ({ available: true, value: snapshot(2) })),
    client
  );
  releaseFirst();
  const releaseSecond = activateApprovalDecisionScope(projection.scope);
  finish();
  await pending; // Await the actual decision publication, not a fixture render.
  assert.deepEqual(applied, [false]);
  assert.equal(projection.read(2000)?.items[0].status, 'pending');
  projection.receive(snapshot(2), client);
  assert.equal(projection.read(2000)?.items[0].status, 'pending');
  unsubscribe();
  releaseSecond();
});

test('all shared native decisions update the same pending projection before server readback', async () => {
  const projection = createApprovalInboxProjection();
  const client = {};
  projection.receive(snapshot(1), client);
  const release = activateApprovalDecisionScope(projection.scope);
  const unsubscribe = subscribeApprovalDecisions((notice) =>
    projection.complete(notice.scope, notice)
  );
  await decideAgentApprovalWithNativeOwner(
    { roomId: '!room:example.org', eventId: '$event', actionId: 'agent-approval.approve-once' },
    async () => ({
      available: true,
      value: { roomId: '!room:example.org', eventId: '$event', status: 'applied' },
    })
  );
  assert.equal(projection.read(2000)?.items[0].status, 'decided');
  projection.receive(snapshot(1), client); // Deliberately stale pending readback.
  assert.equal(projection.read(2000)?.items[0].status, 'decided');
  projection.receive(snapshot(1), {}); // Different account, same generation number.
  assert.equal(projection.read(2000)?.items[0].status, 'pending');
  unsubscribe();
  release();
});

test('failed or mismatched native replies cannot publish optimistic approval completion', async () => {
  const notices: unknown[] = [];
  const unsubscribe = subscribeApprovalDecisions((notice) => notices.push(notice));
  const input = { roomId: '!room:example.org', eventId: '$event', actionId: 'agent-approval.deny' };
  await assert.rejects(
    decideAgentApprovalWithNativeOwner(input, async () => {
      throw new Error('offline');
    })
  );
  await assert.rejects(
    decideAgentApprovalWithNativeOwner(input, async () => ({
      available: true,
      value: { roomId: '!wrong:example.org', eventId: '$event', status: 'applied' },
    }))
  );
  assert.deepEqual(notices, []);
  unsubscribe();
});

test('an optimistic decision labels the Recent row before account data syncs', async () => {
  const client = {};
  const projection = createApprovalInboxProjection();
  projection.receive(
    await loadApprovalInbox(async () => ({ available: true, value: snapshot(1) })),
    client
  );
  const release = activateApprovalDecisionScope(projection.scope);
  const unsubscribe = subscribeApprovalDecisions((notice) =>
    projection.complete(notice.scope, notice)
  );
  await decideAgentApprovalWithNativeOwner(
    { roomId: '!room:example.org', eventId: '$event', actionId: 'agent-approval.deny' },
    async () => ({
      available: true,
      value: { roomId: '!room:example.org', eventId: '$event', status: 'applied' },
    })
  );
  const item = projection.read(2000)?.items[0];
  assert.equal(item?.status, 'decided');
  assert.equal(item?.decision, 'deny');
  assert.equal(item?.decidedAt, 2000);
  unsubscribe();
  release();
});
