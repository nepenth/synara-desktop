import assert from 'node:assert/strict';
import test from 'node:test';
import { hermesPayloadToMarkdown, parseHermesAgentPayload } from '../hermes';

test('parseHermesAgentPayload reads direct Hermes payloads and filters artifact URLs', () => {
  const payload = parseHermesAgentPayload({
    'org.hermes.agent': {
      title: 'Build result',
      status: 'failed',
      summary: 'One test failed.',
      artifacts: [
        {
          title: 'safe report',
          url: 'https://artifacts.example.org/report.html',
        },
        {
          title: 'unsafe report',
          url: 'http://127.0.0.1/report.html',
        },
      ],
      logs: [{ title: 'test log', code: 'FAIL test' }],
    },
  });

  assert.equal(payload?.title, 'Build result');
  assert.equal(payload?.artifacts[0].url, 'https://artifacts.example.org/report.html');
  assert.equal(payload?.artifacts.length, 1);
  assert.equal(payload?.logs[0].code, 'FAIL test');
  assert.equal(payload?.logs[0].id, 'logs-0');
});

test('parseHermesAgentPayload supports explicitly marked JSON bodies with payloads', () => {
  const payload = parseHermesAgentPayload({
    body: JSON.stringify({
      hermes: true,
      payload: {
        title: 'Agent summary',
        summary: 'Plan complete.',
        code: [{ language: 'ts', code: 'const ok = true;' }],
      },
    }),
  });

  assert.equal(payload?.title, 'Agent summary');
  assert.equal(payload?.code[0].language, 'ts');
});

test('parseHermesAgentPayload supports configured content keys', () => {
  const payload = parseHermesAgentPayload(
    {
      'm.custom.agent': {
        title: 'Custom card',
        summary: 'Rendered from a configured event content key.',
      },
    },
    ['m.custom.agent']
  );

  assert.equal(payload?.title, 'Custom card');
  assert.equal(payload?.summary, 'Rendered from a configured event content key.');
});

test('parseHermesAgentPayload removes malformed action and artifact entries', () => {
  const payload = parseHermesAgentPayload({
    'org.hermes.agent': {
      title: 'Actions test',
      actions: [
        {
          title: 'Valid action',
          prompt: 'Reply',
        },
        {
          title: 'Unsafe link',
          url: 'https://127.0.0.1',
        },
        {
          id: 'no-title',
        },
      ],
      artifacts: [
        {
          title: 'Safe artifact',
          url: 'https://artifacts.example.org/report.html',
        },
        {
          title: 'Bad artifact',
          url: 'https://127.0.0.1/report',
        },
      ],
    },
  });

  assert.equal(payload?.actions.length, 1);
  assert.equal(payload?.actions[0].title, 'Valid action');
  assert.equal(payload?.artifacts.length, 1);
  assert.equal(payload?.artifacts[0].title, 'Safe artifact');
});

test('hermesPayloadToMarkdown exports sections with language fences', () => {
  const payload = parseHermesAgentPayload({
    'org.hermes.agent': {
      title: 'Build result',
      status: 'ok',
      summary: 'Done.',
      code: [{ title: 'Snippet', language: 'ts', code: 'const ok = true;' }],
    },
  });

  assert.ok(payload);
  const markdown = hermesPayloadToMarkdown(payload);
  assert.match(markdown, /^# Build result/);
  assert.match(markdown, /Status: ok/);
  assert.match(markdown, /```ts\nconst ok = true;\n```/);
});

test('parseHermesAgentPayload supports bounded quick actions', () => {
  const payload = parseHermesAgentPayload({
    'org.hermes.agent': {
      title: 'Agent controls',
      actions: [
        {
          id: 'regen',
          title: 'Regenerate',
          kind: 'agent',
          url: 'https://agent.example.org/runs/1/regenerate',
        },
        {
          title: 'Unsafe action',
          url: 'https://127.0.0.1/private',
        },
        {
          title: 'Continue',
          prompt: 'Continue from the last summary.',
        },
      ],
    },
  });

  assert.equal(payload?.actions.length, 2);
  assert.equal(payload?.actions[0].url, 'https://agent.example.org/runs/1/regenerate');
  assert.equal(payload?.actions[1].prompt, 'Continue from the last summary.');
  assert.ok(payload);
  assert.match(hermesPayloadToMarkdown(payload), /## Actions/);
});

test('parseHermesAgentPayload ignores ordinary JSON messages', () => {
  assert.equal(
    parseHermesAgentPayload({
      body: JSON.stringify({
        title: 'Not an agent card',
        summary: 'ordinary JSON',
      }),
    }),
    undefined
  );
});

test('parseHermesAgentPayload ignores giant plain-text bodies', () => {
  assert.equal(parseHermesAgentPayload({ body: 'x'.repeat(200_001) }), undefined);
});

test('parseHermesAgentPayload caps direct payload counts and block sizes', () => {
  const payload = parseHermesAgentPayload({
    'org.hermes.agent': {
      title: 'x'.repeat(300),
      summary: 'y'.repeat(6_000),
      logs: Array.from({ length: 25 }, (_, index) => ({
        title: `log ${index}`,
        code: 'z'.repeat(60_000),
      })),
    },
  });

  assert.equal(payload?.title.length, 200);
  assert.equal(payload?.summary?.length, 5_000);
  assert.equal(payload?.logs.length, 20);
  assert.equal(payload?.logs[0].code.length, 50_000);
});
