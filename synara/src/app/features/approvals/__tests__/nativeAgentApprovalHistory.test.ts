import assert from 'node:assert/strict';
import test from 'node:test';

import { acceptsAgentApprovalHistory } from '../nativeAgentApprovalHistory';

const item = {
  roomId: '!room:example.org',
  eventId: '$event',
  sender: '@hermes:example.org',
  decision: 'deny' as const,
  decidedAt: 2,
  originServerTs: 1,
  expiresAt: 3,
  summary: 'rm file',
};

test('approval history snapshot rejects spoofed identities and control summaries', () => {
  assert.equal(acceptsAgentApprovalHistory({ items: [item] }), true);
  assert.equal(
    acceptsAgentApprovalHistory({
      items: [{ ...item, sender: 'You approved this' }],
    }),
    false
  );
  assert.equal(
    acceptsAgentApprovalHistory({
      items: [{ ...item, roomId: '!ApprovedAlways' }],
    }),
    false
  );
  assert.equal(
    acceptsAgentApprovalHistory({
      items: [{ ...item, summary: 'rm \u202Eelif' }],
    }),
    false
  );
  assert.equal(
    acceptsAgentApprovalHistory({
      items: [{ ...item, decidedAt: -1 }],
    }),
    false
  );
  assert.equal(
    acceptsAgentApprovalHistory({
      items: Array.from({ length: 201 }, (_, index) => ({
        ...item,
        eventId: `$event${index}`,
      })),
    }),
    false
  );
});
