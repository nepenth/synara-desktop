import assert from 'node:assert/strict';
import test from 'node:test';
import {
  describeAgentQuestionReply,
  ownAgentQuestionReplies,
  settleAgentQuestion,
  toggleAgentQuestionChoice,
  type NativeTimelineAgentQuestion,
} from '../agentQuestions';
import { answerAgentQuestionWithNativeOwner } from '../../features/room/nativeReactionOwner';

const question = (overrides: Partial<NativeTimelineAgentQuestion> = {}) =>
  ({
    source: 'structured',
    question: 'Pick a colour',
    choices: [
      { index: 1, label: 'Red', recommended: true },
      { index: 2, label: 'Blue', recommended: false },
      { index: 3, label: 'Green', recommended: false },
    ],
    multiSelect: false,
    allowOther: true,
    allowSkip: true,
    state: 'pending',
    canAnswer: true,
    ...overrides,
  }) satisfies NativeTimelineAgentQuestion;

test('Hermes-settled state wins over everything this device knows', () => {
  const now = 1_000;
  assert.deepEqual(
    settleAgentQuestion(question({ state: 'answered', answer: 'Blue' }), {
      now,
      localAnswer: { kind: 'skip' },
    }),
    { kind: 'answered', text: 'Blue' }
  );
  assert.deepEqual(settleAgentQuestion(question({ state: 'skipped' }), { now }), {
    kind: 'skipped',
  });
  assert.deepEqual(settleAgentQuestion(question({ state: 'expired' }), { now }), {
    kind: 'expired',
  });
  assert.deepEqual(settleAgentQuestion(question({ state: 'cancelled' }), { now }), {
    kind: 'cancelled',
  });
});

test('own reaction, local send and own reply settle a pending card', () => {
  const now = 1_000;
  assert.deepEqual(
    settleAgentQuestion(question({ ownAnswer: { kind: 'choice', index: 2 } }), { now }),
    { kind: 'answered', text: 'Blue' }
  );
  assert.deepEqual(settleAgentQuestion(question({ ownAnswer: { kind: 'skipped' } }), { now }), {
    kind: 'skipped',
  });
  assert.deepEqual(
    settleAgentQuestion(question({ multiSelect: true }), {
      now,
      localAnswer: { kind: 'choices', indices: [3, 1] },
    }),
    { kind: 'answered', text: 'Red, Green' }
  );
  assert.deepEqual(
    settleAgentQuestion(question({ source: 'fallback' }), { now, ownReplyBody: '2' }),
    { kind: 'answered', text: 'Blue' }
  );
  assert.deepEqual(settleAgentQuestion(question(), { now }), { kind: 'pending' });
});

test('a pending card past its expiry reads as expired unless already answered', () => {
  const expiring = question({ expiresAtMs: 500 });
  assert.deepEqual(settleAgentQuestion(expiring, { now: 499 }), { kind: 'pending' });
  assert.deepEqual(settleAgentQuestion(expiring, { now: 500 }), { kind: 'expired' });
  assert.deepEqual(settleAgentQuestion(expiring, { now: 900, ownReplyBody: 'Teal' }), {
    kind: 'answered',
    text: 'Teal',
  });
});

test('replies read like Hermes reads them', () => {
  const card = question();
  assert.deepEqual(describeAgentQuestionReply(card, ' skip '), { kind: 'skipped' });
  assert.deepEqual(describeAgentQuestionReply(card, '1, 3'), {
    kind: 'answered',
    text: 'Red, Green',
  });
  assert.deepEqual(describeAgentQuestionReply(card, '9'), { kind: 'answered', text: '9' });
  assert.deepEqual(describeAgentQuestionReply(card, 'Something else'), {
    kind: 'answered',
    text: 'Something else',
  });
  assert.deepEqual(describeAgentQuestionReply(question({ allowSkip: false }), 'skip'), {
    kind: 'answered',
    text: 'skip',
  });
  assert.deepEqual(describeAgentQuestionReply(card, '   '), { kind: 'pending' });
});

test('multi-select toggles stay sorted and unique', () => {
  assert.deepEqual(toggleAgentQuestionChoice([], 3), [3]);
  assert.deepEqual(toggleAgentQuestionChoice([3], 1), [1, 3]);
  assert.deepEqual(toggleAgentQuestionChoice([1, 3], 3), [1]);
});

test('own replies are keyed by card and ignore other senders', () => {
  const replies = ownAgentQuestionReplies(
    [
      { kind: 'message', senderId: '@me:x', body: '1', reply: { eventId: '$card' } },
      { kind: 'message', senderId: '@bob:x', body: '2', reply: { eventId: '$card' } },
      { kind: 'message', senderId: '@me:x', body: 'no reply' },
      { kind: 'message', senderId: '@me:x', body: 'Teal', reply: { eventId: '$other' } },
    ],
    '@me:x'
  );
  assert.equal(replies.get('$card'), '1');
  assert.equal(replies.get('$other'), 'Teal');
  assert.equal(ownAgentQuestionReplies([], undefined).size, 0);
});

test('the native owner route forwards the answer and checks the readback', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const result = await answerAgentQuestionWithNativeOwner(
    { roomId: '!r:x', eventId: '$card', answer: { kind: 'choice', index: 2 } },
    async (command, args) => {
      calls.push({ command, args });
      return {
        available: true,
        value: { roomId: '!r:x', eventId: '$card', status: 'sent', route: 'reaction' },
      };
    }
  );
  assert.equal(result.route, 'reaction');
  assert.deepEqual(calls, [
    {
      command: 'matrix_agent_question_answer',
      args: { roomId: '!r:x', eventId: '$card', answer: { kind: 'choice', index: 2 } },
    },
  ]);
  await assert.rejects(
    answerAgentQuestionWithNativeOwner(
      { roomId: '!r:x', eventId: '$card', answer: { kind: 'skip' } },
      async () => ({
        available: true,
        value: { roomId: '!r:x', eventId: '$elsewhere', status: 'sent', route: 'reaction' },
      })
    )
  );
  await assert.rejects(
    answerAgentQuestionWithNativeOwner(
      { roomId: '!r:x', eventId: '$card', answer: { kind: 'skip' } },
      async () => ({ available: false })
    )
  );
});
