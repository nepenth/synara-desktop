import assert from 'node:assert/strict';
import test from 'node:test';

import { parseNativeTimelineAgentCard } from '../nativeTimelineView';

test('native timeline parses the bounded Rust agent-card payload', () => {
  const payload = parseNativeTimelineAgentCard(
    JSON.stringify({
      title: 'Approval',
      status: 'pending',
      summary: 'Review the action',
      actions: [{ id: 'approve', title: 'Approve', kind: 'approve' }],
    })
  );

  assert.equal(payload?.title, 'Approval');
  assert.equal(payload?.summary, 'Review the action');
  assert.equal(payload?.actions[0]?.title, 'Approve');
});

test('native timeline rejects malformed or non-card JSON', () => {
  assert.equal(parseNativeTimelineAgentCard('{'), undefined);
  assert.equal(parseNativeTimelineAgentCard(JSON.stringify({ title: 'No content' })), undefined);
  assert.equal(parseNativeTimelineAgentCard(undefined), undefined);
});
