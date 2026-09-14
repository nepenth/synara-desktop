import assert from 'node:assert/strict';
import test from 'node:test';
import {
  historyDecisionLabel,
  historyItemToInboxItem,
  unionRecentApprovals,
} from '../approvalHistoryProjection';
import type { ApprovalInboxItem } from '../nativeApprovalInbox';
import type { SynaraAgentApprovalHistoryItem } from '../../../../types/matrix/accountData';

const inbox = (
  overrides: Partial<ApprovalInboxItem> & Pick<ApprovalInboxItem, 'eventId' | 'status'>
): ApprovalInboxItem => ({
  roomId: '!room:example.org',
  sender: '@hermes:example.org',
  body: 'Approval Required: Dangerous Command',
  canSendReaction: true,
  bodyTruncated: false,
  originServerTs: 1_000,
  expiresAt: 301_000,
  ...overrides,
});

const history = (
  overrides: Partial<SynaraAgentApprovalHistoryItem> = {}
): SynaraAgentApprovalHistoryItem => ({
  roomId: '!room:example.org',
  eventId: '$history',
  sender: '@hermes:example.org',
  decision: 'approve_once',
  decidedAt: 2_000,
  originServerTs: 1_000,
  expiresAt: 301_000,
  summary: 'rm file',
  ...overrides,
});

test('unionRecentApprovals keeps pending out and lets account data win', () => {
  const recent = unionRecentApprovals(
    [
      inbox({ eventId: '$pending', status: 'pending' }),
      inbox({ eventId: '$expired', status: 'expired', originServerTs: 500 }),
      inbox({
        eventId: '$same',
        status: 'decided',
        body: 'inbox body',
        originServerTs: 1_500,
      }),
    ],
    [
      history({ eventId: '$same', decidedAt: 9_000, summary: 'account summary' }),
      history({ eventId: '$synced', decidedAt: 8_000, summary: 'ls' }),
    ]
  );

  assert.deepEqual(
    recent.map((item) => item.eventId),
    ['$same', '$synced', '$expired']
  );
  assert.equal(recent[0].summary, 'account summary');
  assert.equal(recent[0].status, 'decided');
  assert.equal(recent[0].decision, 'approve_once');
  assert.equal(recent[2].status, 'expired');
  assert.equal(
    recent.find((item) => item.eventId === '$pending'),
    undefined
  );
});

test('history items never render as expired and keep a bounded summary', () => {
  const item = historyItemToInboxItem(
    history({
      eventId: '$old',
      expiresAt: 2,
      decidedAt: 1_700_000_000_000,
    })
  );
  assert.equal(item.status, 'decided');
  assert.equal(historyDecisionLabel(item.decision, item.status), 'Approved once');
  assert.equal(historyDecisionLabel('approve_always', 'decided'), 'Approved always');
  assert.equal(historyDecisionLabel('deny', 'decided'), 'Denied');
  assert.equal(historyDecisionLabel(undefined, 'expired'), 'Expired');
});
