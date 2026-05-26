import assert from 'node:assert/strict';
import test from 'node:test';
import {
  DEFAULT_POLL_SELECTIONS,
  MAX_POLL_SELECTIONS,
  makePollResponseContent,
  makePollStartContent,
  makePollStartContentFromCommand,
  normalizePollParts,
  parsePollCommand,
  parsePollResponseContent,
  parsePollStartContent,
  summarizePollResponses,
} from '../polls';

test('makePollStartContent creates a Matrix poll with fallback text', () => {
  const content = makePollStartContent('Pick one?', ['Alpha', 'Beta']);
  assert.ok(content);
  assert.equal(content?.body, 'Pick one?\n1. Alpha\n2. Beta');

  const poll = parsePollStartContent(content as Record<string, unknown>);
  assert.equal(poll?.question, 'Pick one?');
  assert.deepEqual(
    poll?.answers.map((answer) => answer.text),
    ['Alpha', 'Beta']
  );
  assert.equal(poll?.maxSelections, 1);
});

test('makePollStartContent sets max-selections', () => {
  const content = makePollStartContent('Pick two', ['Alpha', 'Beta', 'Gamma'], 2);
  assert.ok(content);
  const poll = parsePollStartContent(content as Record<string, unknown>);
  assert.equal(poll?.maxSelections, 2);
});

test('makePollStartContent clamps out-of-range max-selections', () => {
  const tooManyAnswers = Array.from({ length: 25 }, (_, index) => `Option ${index}`);
  const content = makePollStartContent('Many', tooManyAnswers, 999);
  const poll = parsePollStartContent(content as Record<string, unknown>);
  assert.equal(poll?.answers.length, 20);
  assert.equal(poll?.maxSelections, MAX_POLL_SELECTIONS);
});

test('normalizePollParts rejects incomplete polls and caps answers', () => {
  assert.equal(normalizePollParts('', ['a', 'b']), undefined);
  assert.equal(normalizePollParts('Question', ['one']), undefined);
  assert.equal(
    normalizePollParts(
      'Question',
      Array.from({ length: 25 }, (_, i) => `${i}`)
    )?.answers.length,
    20
  );
});

test('parsePollCommand supports slash-command style poll input with max selections', () => {
  const poll = parsePollCommand('Deploy now? | Yes | No | max=2');
  assert.equal(poll?.question, 'Deploy now?');
  assert.deepEqual(
    poll?.answers.map((answer) => answer.text),
    ['Yes', 'No']
  );
  assert.equal(poll?.maxSelections, 2);
  assert.ok(makePollStartContentFromCommand('Deploy now? | Yes | No | max=2'));
  assert.equal(parsePollCommand('One option only'), undefined);
});

test('poll response content references and deduplicates the poll event answers', () => {
  const content = makePollResponseContent('$poll', ['a1', 'a1', '']);
  assert.deepEqual(content['m.relates_to'], {
    rel_type: 'm.reference',
    event_id: '$poll',
  });
  assert.deepEqual(parsePollResponseContent(content as Record<string, unknown>), ['a1']);
});

test('summarizePollResponses counts latest vote per sender including multiple selections', () => {
  assert.deepEqual(
    summarizePollResponses(
      [
        { sender: '@a:example.org', ts: 1, answers: ['one'] },
        { sender: '@a:example.org', ts: 2, answers: ['two', 'one'] },
        { sender: '@b:example.org', ts: 3, answers: ['two'] },
      ],
      ['one', 'two']
    ),
    {
      one: 1,
      two: 2,
    }
  );
});

test('parsePollStartContent parses max_selections and camelCase legacy value', () => {
  const payload = parsePollStartContent({
    'm.poll.start': {
      question: { 'm.text': 'Question?' },
      kind: 'm.poll.disclosed',
      max_selections: '3',
      answers: [
        { id: 'a1', 'm.text': 'A' },
        { id: 'a2', 'm.text': 'B' },
        { id: 'a3', 'm.text': 'C' },
      ],
    },
  });
  assert.equal(payload?.maxSelections, 3);

  const legacy = parsePollStartContent({
    'm.poll.start': {
      question: { 'm.text': 'Legacy?' },
      kind: 'm.poll.disclosed',
      maxSelections: 5,
      answers: [
        { id: 'a1', 'm.text': 'A' },
        { id: 'a2', 'm.text': 'B' },
      ],
    },
  });
  assert.equal(legacy?.maxSelections, 2);
});

test('parsePollStartContent returns undefined for malformed payloads', () => {
  assert.equal(
    parsePollStartContent({
      'org.matrix.msc3381.poll.start': {
        question: { 'm.text': 'No answers' },
      },
    }),
    undefined
  );
  assert.equal(
    parsePollStartContent({
      'org.matrix.msc3381.poll.start': {
        question: { 'm.text': 'Bad max' },
        max_selections: 'not-a-number',
        answers: [{ id: 'a1', 'm.text': 'A' }],
      },
    }),
    undefined
  );
  assert.equal(
    parsePollStartContent({
      'org.matrix.msc3381.poll.start': {
        question: { 'm.text': 'One answer' },
        answers: [{ id: 'a1', 'm.text': 'A' }],
      },
    }),
    undefined
  );
});

test('parsePollCommand clamps max selections into valid bounds', () => {
  const poll = parsePollCommand('Question | A | B | max=99');
  assert.equal(poll?.maxSelections, 2);

  const pollUnder = parsePollCommand('Question | A | B | max=0');
  assert.equal(pollUnder?.maxSelections, DEFAULT_POLL_SELECTIONS);

  const pollMany = parsePollCommand(
    'Question | A | B | C | D | E | F | G | H | I | J | K | max=99'
  );
  assert.equal(pollMany?.maxSelections, MAX_POLL_SELECTIONS);
});
