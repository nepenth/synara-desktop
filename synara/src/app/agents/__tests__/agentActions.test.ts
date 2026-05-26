import test from 'node:test';
import assert from 'node:assert/strict';
import {
  MAX_AGENT_ACTION_MARKDOWN_LENGTH,
  MAX_AGENT_ACTION_TEXT_LENGTH,
  MAX_AGENT_ACTION_URL_LENGTH,
  normalizeAgentActionPayload,
} from '../agentActions';

test('normalizeAgentActionPayload accepts safe allowed actions', () => {
  assert.deepEqual(
    normalizeAgentActionPayload({
      id: ' export ',
      title: ' Export Thread ',
      kind: 'COPY',
      prompt: ' Copy this prompt ',
      markdown: '# Thread',
    }),
    {
      id: 'export',
      title: 'Export Thread',
      kind: 'copy',
      prompt: 'Copy this prompt',
      markdown: '# Thread',
      url: undefined,
    }
  );

  assert.deepEqual(
    normalizeAgentActionPayload({
      id: 'open',
      title: 'Open report',
      kind: 'open_url',
      url: 'https://agent.example.org/report',
    }),
    {
      id: 'open',
      title: 'Open report',
      kind: 'open_url',
      prompt: undefined,
      markdown: undefined,
      url: 'https://agent.example.org/report',
    }
  );
});

test('normalizeAgentActionPayload allows safe URL actions without explicit kind', () => {
  assert.deepEqual(
    normalizeAgentActionPayload({
      id: 'artifact',
      title: 'Open artifact',
      url: 'https://artifacts.example.org/report.html',
    }),
    {
      id: 'artifact',
      title: 'Open artifact',
      kind: undefined,
      prompt: undefined,
      markdown: undefined,
      url: 'https://artifacts.example.org/report.html',
    }
  );
});

test('normalizeAgentActionPayload rejects missing identifiers or runnable payloads', () => {
  assert.equal(
    normalizeAgentActionPayload({
      id: '',
      title: 'Missing id',
      prompt: 'Continue',
    }),
    undefined
  );
  assert.equal(
    normalizeAgentActionPayload({
      id: 'no-payload',
      title: 'No payload',
    }),
    undefined
  );
});

test('normalizeAgentActionPayload rejects unsafe urls and unsupported kinds', () => {
  assert.equal(
    normalizeAgentActionPayload({
      id: 'unsafe-url',
      title: 'Open private',
      url: 'https://127.0.0.1/private',
    }),
    undefined
  );
  assert.equal(
    normalizeAgentActionPayload({
      id: 'unknown-kind',
      title: 'Execute',
      kind: 'shell',
      prompt: 'rm -rf /',
    }),
    undefined
  );
});

test('normalizeAgentActionPayload rejects oversized fields', () => {
  assert.equal(
    normalizeAgentActionPayload({
      id: 'x'.repeat(MAX_AGENT_ACTION_TEXT_LENGTH + 1),
      title: 'Too long',
      prompt: 'Continue',
    }),
    undefined
  );
  assert.equal(
    normalizeAgentActionPayload({
      id: 'too-long-url',
      title: 'Open',
      url: `https://example.org/${'x'.repeat(MAX_AGENT_ACTION_URL_LENGTH)}`,
    }),
    undefined
  );
  assert.equal(
    normalizeAgentActionPayload({
      id: 'too-long-markdown',
      title: 'Copy',
      markdown: 'x'.repeat(MAX_AGENT_ACTION_MARKDOWN_LENGTH + 1),
    }),
    undefined
  );
});
