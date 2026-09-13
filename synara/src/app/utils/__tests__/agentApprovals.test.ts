import assert from 'node:assert/strict';
import test from 'node:test';
import {
  AGENT_APPROVAL_NATIVE_ACTION_TTL_MS,
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
  detectAgentApprovalPrompt,
  hasLocalAgentApprovalReactionFromSenders,
  isAgentApprovalNativeActionExpired,
  planAgentApprovalNativeNotificationAction,
} from '../agentApprovals';

const exampleApprovalBody = `⚠️ Dangerous command requires approval

Code

Copy

set -euo pipefail
WR=/home/example/.hermes/profiles/example/workspace/resources/vaultwarden.sh
printf 'status:\\n'

Reason: Security scan - [HIGH] Pipe to interpreter: $WR | python3

Reply /approve to execute, /approve session to approve this pattern for the session, /approve always to approve permanently, or /deny to cancel.

You can also click the reaction to approve:
✅ = /approve
♾️ = /approve always
❌ = /deny`;

test('detectAgentApprovalPrompt recognizes Hermes dangerous command prompts', () => {
  const prompt = detectAgentApprovalPrompt({ body: exampleApprovalBody });

  assert.equal(prompt?.title, 'Approval Required: Dangerous Command');
  assert.match(prompt?.body ?? '', /Security scan/);
  assert.equal(prompt?.commandPreview, 'set -euo pipefail');
  assert.match(prompt?.command ?? '', /vaultwarden\.sh/);
  assert.match(prompt?.sourceContext ?? '', /Dangerous command requires approval/);
  assert.match(prompt?.replyInstructions ?? '', /Reply \/approve/);
});

test('detectAgentApprovalPrompt recognizes compact approval command bodies', () => {
  const prompt = detectAgentApprovalPrompt({
    body: `⚠️ Dangerous command requires approval

Code

Copy
scp /tmp/gatekeeper-autoresearch-main.bundle build-host@192.0.2.10:/tmp/gatekeeper-autoresearch-main.bundle
Reason: Security scan — [MEDIUM] URL uses raw IP address: URL points to IP address 192.0.2.10 instead of a domain name

Reply /approve to execute, /approve session to approve this pattern for the session, /approve always to approve permanently, or /deny to cancel.

You can also click the reaction to approve:
✅ = /approve
♾️ = /approve always
❌ = /deny`,
  });

  assert.match(prompt?.body ?? '', /URL uses raw IP address/);
  assert.equal(
    prompt?.commandPreview,
    'scp /tmp/gatekeeper-autoresearch-main.bundle build-host@192.0.2.10:/tmp/gatekeeper-autoresearch-main.bundle'
  );
});

test('detectAgentApprovalPrompt recognizes bang-command reaction prompts', () => {
  const prompt = detectAgentApprovalPrompt({
    body: `⚠️ Dangerous command requires approval

Code

Copy
set -euo pipefail
curl -fsS http://browser-control.example.com:9377/openapi.json -o /tmp/camofox_openapi.json
python3 - <<'PY'
import json
with open('/tmp/camofox_openapi.json') as f: d=json.load(f)
for p, methods in d.get('paths',{}).items():
    if any(x in p for x in ['/tabs','snapshot','navigate','sessions']):
        print(p, ','.join(methods.keys()))
PY
Reason: Security scan — [HIGH] Plain HTTP URL in execution context: URL 'http://browser-control.example.com:9377/openapi.json' uses unencrypted HTTP and is being passed to a command that downloads or executes content. An attacker on the network could modify the content.

Reply !approve to execute, !approve session to approve this pattern for the session, !approve always to approve permanently, or !deny to cancel.

You can also react to this prompt:
✅ = approve once
♾️ = approve always
❌ = deny`,
  });

  assert.match(prompt?.body ?? '', /Plain HTTP URL/);
  assert.equal(prompt?.commandPreview, 'set -euo pipefail');
  assert.match(prompt?.command ?? '', /browser-control\.example\.com/);
  assert.match(prompt?.command ?? '', /python3 - <<'PY'/);
  assert.match(prompt?.command ?? '', /json\.load/);
  assert.doesNotMatch(prompt?.command ?? '', /Reason:/);
  assert.match(prompt?.replyInstructions ?? '', /Reply !approve/);
  assert.match(prompt?.replyInstructions ?? '', /approve always/);
  assert.match(prompt?.sourceContext ?? '', /You can also react/);
});

test('detectAgentApprovalPrompt keeps heredoc commands and invalid-hostname context', () => {
  const body = `⚠️ Dangerous command requires approval

Code

Copy
python3 - <<'PY'
import socket
hostname = socket.gethostname()
if hostname not in {"worker-a", "worker-b"}:
    raise SystemExit(f"invalid hostname: {hostname}")
print("ok", hostname)
PY

Reason: Security scan — [HIGH] Invalid hostname: target host is not in the allowlist for remote execution.

Reply !approve to execute, !approve session to approve this pattern for the session, !approve always to approve permanently, or !deny to cancel.

You can also react to this prompt:
✅ = approve once
♾️ = approve always
❌ = deny`;

  const prompt = detectAgentApprovalPrompt({ body });

  assert.equal(prompt?.title, 'Approval Required: Dangerous Command');
  assert.match(prompt?.body ?? '', /Invalid hostname/);
  assert.equal(prompt?.commandPreview, "python3 - <<'PY'");
  assert.match(prompt?.command ?? '', /python3 - <<'PY'/);
  assert.match(prompt?.command ?? '', /socket\.gethostname/);
  assert.match(prompt?.command ?? '', /raise SystemExit/);
  assert.match(prompt?.command ?? '', /^PY$/m);
  assert.doesNotMatch(prompt?.command ?? '', /Reason:/);
  assert.doesNotMatch(prompt?.command ?? '', /Reply !approve/);
  assert.match(prompt?.replyInstructions ?? '', /Reply !approve/);
  assert.match(prompt?.replyInstructions ?? '', /♾️ = approve always/);
  assert.match(prompt?.replyInstructions ?? '', /❌ = deny/);
  assert.match(prompt?.sourceContext ?? '', /Code/);
  assert.match(prompt?.sourceContext ?? '', /Copy/);
  assert.match(prompt?.sourceContext ?? '', /Invalid hostname/);
  assert.match(prompt?.sourceContext ?? '', /You can also react/);
});

test('detectAgentApprovalPrompt recognizes markdown-bold approval headings', () => {
  const prompt = detectAgentApprovalPrompt({
    body: `⚠️ **Dangerous command requires approval**
\`\`\`
set -euo pipefail
scp /tmp/configure-monitoring.py hypervisor:/tmp/configure-monitoring.py
\`\`\`
Reason: sudo with privilege flag (stdin/askpass/shell/list)`,
  });

  assert.match(prompt?.body ?? '', /sudo with privilege flag/);
  assert.equal(prompt?.commandPreview, 'set -euo pipefail');
});

test('detectAgentApprovalPrompt recognizes the stable approval title only', () => {
  const prompt = detectAgentApprovalPrompt({
    body: 'Approval Required: Dangerous Command',
  });

  assert.equal(prompt?.title, 'Approval Required: Dangerous Command');
  assert.equal(prompt?.body, 'A Hermes Agent command is waiting for approval.');
});

test('detectAgentApprovalPrompt uses formatted text only after a valid plain-body heading', () => {
  const prompt = detectAgentApprovalPrompt({
    body: 'Approval Required: Dangerous Command',
    formatted_body:
      '<strong>Approval Required: Dangerous Command</strong><p>Code</p><p>Copy</p><pre>npm audit</pre>',
  });

  assert.equal(prompt?.commandPreview, 'npm audit');
});

test('detectAgentApprovalPrompt supports fenced command blocks', () => {
  const prompt = detectAgentApprovalPrompt({
    body: `Approval Required: Dangerous Command

\`\`\`bash
npm install
\`\`\``,
  });

  assert.equal(prompt?.commandPreview, 'npm install');
});

test('approval notification detection accepts the same first-line normalization as Core', () => {
  for (const heading of [
    'Approval Required: Dangerous Command',
    'Dangerous command requires approval',
    '⚠️ **Dangerous command requires approval**',
    '  ⚠ **APPROVAL REQUIRED: DANGEROUS COMMAND**:  ',
    '\n \t\r\n⚠️ Dangerous\tcommand   requires approval:\r\n',
    '\u0085\nDangerous\u0085command requires approval',
    'Dangerous\u2028command requires approval',
  ]) {
    assert.ok(detectAgentApprovalPrompt({ body: `${heading}\nReason: test` }), heading);
  }
});

test('approval notification detection rejects quotes, later headings, and heading-like prose', () => {
  for (const body of [
    '> ⚠️ **Dangerous command requires approval**\n> quoted prompt',
    '"Dangerous command requires approval"',
    'A quoted dangerous command requires approval later in this message',
    'Status update\n\nApproval Required: Dangerous Command\nCode\nrm file',
    'Dangerous command\nrequires approval',
    'Dangerous command requires approval before we continue',
    'Approval Required: Dangerous Command — example only',
    '# Dangerous command requires approval',
    '```\nDangerous command requires approval\n```',
    'Dangerous **command** requires approval',
    '\uFEFFDangerous command requires approval',
    '   \n\t',
  ]) {
    assert.equal(detectAgentApprovalPrompt({ body }), undefined, body);
  }
});

test('formatted HTML cannot manufacture approval eligibility or bypass the plain-body limit', () => {
  const formatted_body = '<strong>Approval Required: Dangerous Command</strong><pre>rm file</pre>';
  for (const body of [
    undefined,
    '',
    'An ordinary message',
    '> Dangerous command requires approval',
    `Dangerous command requires approval\n${'x'.repeat(100_000)}`,
  ]) {
    assert.equal(detectAgentApprovalPrompt({ body, formatted_body }), undefined);
  }
});

test('approval body bound counts Unicode scalar values like Core', () => {
  const heading = 'Dangerous command requires approval\n';
  const body = heading + '😀'.repeat(100_000 - heading.length);
  assert.ok(detectAgentApprovalPrompt({ body }));
  assert.equal(detectAgentApprovalPrompt({ body: `${body}😀` }), undefined);
});

test('detectAgentApprovalPrompt ignores ordinary messages mentioning commands', () => {
  assert.equal(
    detectAgentApprovalPrompt({
      body: 'I think we should approve this command later, but there is no approval title.',
    }),
    undefined
  );
});

test('detectAgentApprovalPrompt ignores huge bodies', () => {
  assert.equal(
    detectAgentApprovalPrompt({
      body: `Dangerous command requires approval ${'x'.repeat(
        100_001
      )} Reply /approve /approve always /deny`,
    }),
    undefined
  );
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
      nowMs: 10_000_000,
      eventTsMs: 1,
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

test('planAgentApprovalNativeNotificationAction enforces TTL and local dedupe gates', () => {
  const nowMs = 1_000_000;
  const expired = planAgentApprovalNativeNotificationAction({
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
    context: {
      kind: AGENT_APPROVAL_NOTIFICATION_KIND,
      roomId: '!room:matrix.org',
      eventId: '$event:matrix.org',
    },
    nowMs,
    eventTsMs: nowMs - AGENT_APPROVAL_NATIVE_ACTION_TTL_MS - 1,
    eventResolved: true,
    isApprovalPrompt: true,
  });
  assert.equal(expired.type, 'reject');
  if (expired.type === 'reject') assert.equal(expired.reason, 'expired-ttl');

  assert.equal(
    planAgentApprovalNativeNotificationAction({
      actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
      context: {
        kind: AGENT_APPROVAL_NOTIFICATION_KIND,
        roomId: '!room:matrix.org',
        eventId: '$event:matrix.org',
      },
      alreadyActed: true,
      eventResolved: true,
      isApprovalPrompt: true,
    }).type,
    'reject'
  );
  assert.equal(
    planAgentApprovalNativeNotificationAction({
      actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
      context: {
        kind: AGENT_APPROVAL_NOTIFICATION_KIND,
        roomId: '!room:matrix.org',
        eventId: '$event:matrix.org',
      },
      alreadyReactedLocally: true,
      eventResolved: true,
      isApprovalPrompt: true,
    }).type,
    'reject'
  );
});

test('isAgentApprovalNativeActionExpired uses event and notification timestamps', () => {
  const nowMs = 10_000;
  assert.equal(
    isAgentApprovalNativeActionExpired({
      nowMs,
      eventTsMs: nowMs - AGENT_APPROVAL_NATIVE_ACTION_TTL_MS + 1,
    }),
    false
  );
  assert.equal(
    isAgentApprovalNativeActionExpired({
      nowMs,
      eventTsMs: nowMs - AGENT_APPROVAL_NATIVE_ACTION_TTL_MS - 1,
    }),
    true
  );
  assert.equal(
    isAgentApprovalNativeActionExpired({
      nowMs,
      notificationCreatedAtMs: nowMs - AGENT_APPROVAL_NATIVE_ACTION_TTL_MS - 1,
    }),
    true
  );
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
