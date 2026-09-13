import assert from 'node:assert/strict';
import test from 'node:test';
import {
  acceptsApprovalInbox,
  approvalIdentity,
  approvalStatus,
  loadApprovalInbox,
  type ApprovalInboxItem,
} from '../nativeApprovalInbox';
import { parseSynaraRouteDestination } from '../../../routes/synaraRoutes';

const item: ApprovalInboxItem = {
  roomId: '!room:example.org',
  eventId: '$request',
  sender: '@hermes:example.org',
  body: 'Dangerous command requires approval',
  canSendReaction: true,
  bodyTruncated: false,
  originServerTs: 1000,
  expiresAt: 301000,
  status: 'pending',
};
const snapshot = {
  coverageWindowMs: 300000,
  sessionGeneration: 2,
  items: [item],
  loading: false,
  incomplete: false,
};

test('pending requests expire at the same exact boundary as Core; decisions stay terminal', () => {
  assert.equal(approvalStatus(item, 300999), 'pending');
  assert.equal(approvalStatus(item, 301000), 'expired');
  assert.equal(approvalStatus({ ...item, status: 'decided' }, 400000), 'decided');
});

test('snapshot validation rejects stub, malformed expiry, duplicate identities, and unknown statuses', () => {
  assert.equal(acceptsApprovalInbox(snapshot), true);
  for (const value of [
    {},
    null,
    { ...snapshot, items: [null] },
    { ...snapshot, items: [item, item] },
    { ...snapshot, items: [{ ...item, expiresAt: 0 }] },
    { ...snapshot, items: [{ ...item, status: 'approved' }] },
    { ...snapshot, sessionGeneration: -1 },
    { ...snapshot, incomplete: undefined },
  ]) {
    assert.equal(acceptsApprovalInbox(value), false);
  }
});

test('approval identities include the room and never collapse equal event labels across rooms', () => {
  assert.notEqual(
    approvalIdentity(item),
    approvalIdentity({ ...item, roomId: '!other:example.org' })
  );
});

test('inbox reads only its native command and retains incomplete coverage', async () => {
  const result = await loadApprovalInbox(async (command) => {
    assert.equal(command, 'matrix_agent_approvals_list');
    return { available: true, value: { ...snapshot, incomplete: true } };
  });
  assert.equal(result.incomplete, true);
  await assert.rejects(
    () => loadApprovalInbox(async () => ({ available: false })),
    /could not be loaded/
  );
  await assert.rejects(
    () => loadApprovalInbox(async () => ({ available: true, value: {} })),
    /could not be loaded/
  );
});

test('approvals routes resolve as the approval center rather than a Matrix space', () => {
  assert.deepEqual(parseSynaraRouteDestination('/approvals/'), { kind: 'approvals' });
  assert.deepEqual(parseSynaraRouteDestination('/approvals'), { kind: 'approvals' });
  assert.equal(parseSynaraRouteDestination('/approvals/unknown'), undefined);
});
