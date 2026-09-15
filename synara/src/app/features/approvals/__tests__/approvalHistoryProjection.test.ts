import assert from 'node:assert/strict';
import test from 'node:test';
import {
  compareRecentApprovals,
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

test('unionRecentApprovals keeps pending out and does not let account data name an unproven decision', () => {
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
  assert.equal(recent[0].decision, undefined);
  assert.equal(recent[2].status, 'expired');
  assert.equal(
    recent.find((item) => item.eventId === '$pending'),
    undefined
  );
});

test('inbox decision wins a conflict with history', () => {
  const recent = unionRecentApprovals(
    [
      inbox({
        eventId: '$same',
        status: 'decided',
        decision: 'deny',
        decidedAt: 4_000,
      }),
    ],
    [history({ eventId: '$same', decision: 'approve_always', decidedAt: 9_000, summary: 'ls' })]
  );
  assert.equal(recent.length, 1);
  assert.equal(recent[0].decision, 'deny');
  assert.equal(recent[0].status, 'decided');
  assert.equal(recent[0].summary, 'ls');
});

test('expired inbox without a decision still shows the history decision', () => {
  const recent = unionRecentApprovals(
    [inbox({ eventId: '$same', status: 'expired', originServerTs: 1_000 })],
    [history({ eventId: '$same', decision: 'approve_always', decidedAt: 9_000, summary: 'ls' })]
  );
  assert.equal(recent[0].decision, 'approve_always');
  assert.equal(recent[0].status, 'decided');
  assert.equal(historyDecisionLabel(recent[0].decision, recent[0].status), 'Approved always');
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
  assert.equal(historyDecisionLabel(undefined, 'decided'), 'Decided');
});

test('empty or whitespace summaries fall back to a readable label', () => {
  const blank = historyItemToInboxItem(history({ summary: '' }));
  const spaces = historyItemToInboxItem(history({ summary: '   ' }));
  assert.equal(blank.summary, 'Command not recorded');
  assert.equal(blank.body, 'Command not recorded');
  assert.equal(spaces.summary, 'Command not recorded');
});

test('recent sort is stable for equal timestamps and missing decidedAt', () => {
  const recent = unionRecentApprovals(
    [
      inbox({
        eventId: '$b',
        roomId: '!room:example.org',
        status: 'expired',
        originServerTs: 1_000,
      }),
      inbox({
        eventId: '$a',
        roomId: '!room:example.org',
        status: 'expired',
        originServerTs: 1_000,
      }),
    ],
    [
      history({
        eventId: '$late',
        roomId: '!other:example.org',
        decidedAt: 5_000,
        originServerTs: 1,
      }),
    ]
  );
  assert.deepEqual(
    recent.map((item) => item.eventId),
    ['$late', '$a', '$b']
  );
  assert.equal(compareRecentApprovals(recent[1], recent[2]) < 0, true);
});

test('duplicate history identities keep a single row and 240-char unspaced summaries stay intact', () => {
  const long = 'あ'.repeat(240);
  const recent = unionRecentApprovals(
    [inbox({ eventId: '$dup', status: 'expired', originServerTs: 1 })],
    [
      history({ eventId: '$dup', decidedAt: 3, summary: 'first' }),
      history({ eventId: '$dup', decidedAt: 4, summary: long }),
    ]
  );
  assert.equal(recent.length, 1);
  assert.equal(recent[0].summary, long);
  assert.equal(recent[0].status, 'decided');
});
