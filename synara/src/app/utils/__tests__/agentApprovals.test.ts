import assert from 'node:assert/strict';
import test from 'node:test';
import { detectAgentApprovalPrompt } from '../agentApprovals';

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
  assert.match(prompt?.command ?? '', /camofox-browser\.whyland\.com/);
  assert.doesNotMatch(prompt?.command ?? '', /Reason:/);
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

test('detectAgentApprovalPrompt falls back to formatted_body title text', () => {
  const prompt = detectAgentApprovalPrompt({
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
