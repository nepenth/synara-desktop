import assert from 'node:assert/strict';
import test from 'node:test';
import {
  AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS,
  AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS,
  AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
  AGENT_APPROVAL_NOTIFICATION_ACTION_DENY,
  AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW,
  AGENT_APPROVAL_NOTIFICATION_ACTIONS,
  AGENT_APPROVAL_NOTIFICATION_KIND,
  AGENT_APPROVAL_REACTION_APPROVE_ONCE,
  AGENT_APPROVAL_REACTION_DENY,
  buildAgentApprovalNativeActionDedupeKey,
  createAgentApprovalNativeActionDedupeStore,
  formatCoreAgentApprovalPrompt,
  hasLocalAgentApprovalReactionFromSenders,
  planAgentApprovalNativeNotificationAction,
} from '../agentApprovals';

test('Core-approved prompt formatting preserves command and context without classifying', () => {
  const prompt = formatCoreAgentApprovalPrompt(
    'ordinary text\n```sh\necho hello\n```\nReason: demonstrate formatting'
  );
  assert.equal(prompt.title, 'Approval Required: Dangerous Command');
  assert.equal(prompt.command, 'echo hello');
  assert.match(prompt.sourceContext ?? '', /ordinary text/);
  assert.match(prompt.body, /demonstrate formatting/);
});

test('native notification actions exclude approve-always', () => {
  assert.ok(
    AGENT_APPROVAL_NOTIFICATION_ACTIONS.some(
      (action) => action.id === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS
    )
  );
  assert.equal(
    AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS.some(
      (action) => action.id === AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS
    ),
    false
  );
  assert.deepEqual(
    AGENT_APPROVAL_NATIVE_NOTIFICATION_ACTIONS.map((action) => action.id),
    [
      AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
      AGENT_APPROVAL_NOTIFICATION_ACTION_DENY,
      AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW,
    ]
  );
});

test('native Review action opens the exact prompt without requiring event validation', () => {
  assert.deepEqual(
    planAgentApprovalNativeNotificationAction({
      actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_REVIEW,
      context: {
        kind: AGENT_APPROVAL_NOTIFICATION_KIND,
        roomId: '!room:matrix.org',
        eventId: '$event:matrix.org',
      },
    }),
    {
      type: 'open-room',
      roomId: '!room:matrix.org',
      eventId: '$event:matrix.org',
      reason: 'review-requested',
    }
  );
});

test('planAgentApprovalNativeNotificationAction rejects malformed payloads', () => {
  assert.equal(
    planAgentApprovalNativeNotificationAction({
      actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
      context: { kind: 'message', roomId: '!r', eventId: '$e' },
    }).type,
    'reject'
  );
  assert.equal(
    planAgentApprovalNativeNotificationAction({
      actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
      context: { kind: AGENT_APPROVAL_NOTIFICATION_KIND, roomId: '', eventId: '$e' },
    }).type,
    'reject'
  );
  assert.equal(
    planAgentApprovalNativeNotificationAction({
      actionId: 'agent-approval.unknown',
      context: {
        kind: AGENT_APPROVAL_NOTIFICATION_KIND,
        roomId: '!room:matrix.org',
        eventId: '$event:matrix.org',
      },
    }).type,
    'reject'
  );
});

test('planAgentApprovalNativeNotificationAction blocks approve-always from native path', () => {
  const plan = planAgentApprovalNativeNotificationAction({
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS,
    context: {
      kind: AGENT_APPROVAL_NOTIFICATION_KIND,
      roomId: '!room:matrix.org',
      eventId: '$event:matrix.org',
    },
    eventResolved: true,
    isApprovalPrompt: true,
  });

  assert.deepEqual(plan, {
    type: 'open-room',
    roomId: '!room:matrix.org',
    eventId: '$event:matrix.org',
    reason: 'approve-always-requires-in-app-confirmation',
  });
});

test('planAgentApprovalNativeNotificationAction requires validated approval prompt before send', () => {
  const base = {
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
    context: {
      kind: AGENT_APPROVAL_NOTIFICATION_KIND,
      roomId: '!room:matrix.org',
      eventId: '$event:matrix.org',
    },
  };

  assert.equal(
    planAgentApprovalNativeNotificationAction({ ...base, eventResolved: false }).type,
    'reject'
  );
  assert.equal(
    planAgentApprovalNativeNotificationAction({
      ...base,
      eventResolved: true,
      isApprovalPrompt: false,
    }).type,
    'reject'
  );

  assert.deepEqual(
    planAgentApprovalNativeNotificationAction({
      ...base,
      eventResolved: true,
      isApprovalPrompt: true,
    }),
    {
      type: 'send-reaction',
      roomId: '!room:matrix.org',
      eventId: '$event:matrix.org',
      actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
      reaction: AGENT_APPROVAL_REACTION_APPROVE_ONCE,
      dedupeKey: buildAgentApprovalNativeActionDedupeKey('!room:matrix.org', '$event:matrix.org'),
    }
  );

  const denyPlan = planAgentApprovalNativeNotificationAction({
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_DENY,
    context: base.context,
    eventResolved: true,
    isApprovalPrompt: true,
  });
  assert.equal(denyPlan.type, 'send-reaction');
  if (denyPlan.type === 'send-reaction') {
    assert.equal(denyPlan.reaction, AGENT_APPROVAL_REACTION_DENY);
  }
});

test('native presentation planner retains delivery dedupe; Core owns expiry', () => {
  const plan = planAgentApprovalNativeNotificationAction({
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
    context: { kind: AGENT_APPROVAL_NOTIFICATION_KIND, roomId: '!r:example.org', eventId: '$e' },
    alreadyActed: true,
  });
  assert.equal(plan.type, 'reject');
  if (plan.type === 'reject') assert.equal(plan.reason, 'already-acted');
});

test('native action dedupe store persists across store instances sharing storage', () => {
  const memory = new Map<string, string>();
  const storage = {
    getItem: (key: string) => memory.get(key) ?? null,
    setItem: (key: string, value: string) => {
      memory.set(key, value);
    },
    removeItem: (key: string) => {
      memory.delete(key);
    },
    clear: () => memory.clear(),
    key: () => null,
    length: 0,
  } as Storage;

  const first = createAgentApprovalNativeActionDedupeStore(storage, '@alice:example.org');
  const key = buildAgentApprovalNativeActionDedupeKey('!r', '$e');
  assert.equal(key, buildAgentApprovalNativeActionDedupeKey('!r', '$e'));
  assert.equal(first.has(key), false);
  first.add(key);
  assert.equal(first.has(key), true);

  const second = createAgentApprovalNativeActionDedupeStore(storage, '@alice:example.org');
  assert.equal(second.has(key), true);
  const otherAccount = createAgentApprovalNativeActionDedupeStore(storage, '@bob:example.org');
  assert.equal(otherAccount.has(key), false);
  second.remove(key);
  assert.equal(second.has(key), false);
});

test('hasLocalAgentApprovalReactionFromSenders detects current user approval reactions', () => {
  assert.equal(
    hasLocalAgentApprovalReactionFromSenders(
      [
        ['👍', ['@alice:matrix.org']],
        [AGENT_APPROVAL_REACTION_APPROVE_ONCE, ['@bob:matrix.org']],
      ],
      '@alice:matrix.org'
    ),
    false
  );
  assert.equal(
    hasLocalAgentApprovalReactionFromSenders(
      [[AGENT_APPROVAL_REACTION_DENY, ['@alice:matrix.org']]],
      '@alice:matrix.org'
    ),
    true
  );
});
