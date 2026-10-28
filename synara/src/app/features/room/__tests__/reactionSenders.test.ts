import test from 'node:test';
import assert from 'node:assert/strict';
import { reactionSenderName, reactionSendersSummary } from '../reactionSenders';

const names = new Map([
  ['@alice:example.org', 'Alice'],
  ['@bob:example.org', 'Bob'],
  ['@carol:example.org', 'Carol'],
  ['@dan:example.org', 'Dan'],
]);
const me = '@me:example.org';

const summary = (senderIds: string[], count = senderIds.length) =>
  reactionSendersSummary({ reactionKey: '👍', count, senderIds, names, ownUserId: me });

test('names one, two and three people in full', () => {
  assert.equal(summary(['@alice:example.org']), 'Alice reacted with 👍');
  assert.equal(
    summary(['@alice:example.org', '@bob:example.org']),
    'Alice and Bob reacted with 👍'
  );
  assert.equal(
    summary(['@alice:example.org', '@bob:example.org', '@carol:example.org']),
    'Alice, Bob and Carol reacted with 👍'
  );
  assert.equal(
    summary(['@alice:example.org', '@bob:example.org', '@carol:example.org', '@dan:example.org']),
    'Alice, Bob, Carol and Dan reacted with 👍'
  );
});

test('summarizes more than four people as others', () => {
  assert.equal(
    summary(
      [
        '@alice:example.org',
        '@bob:example.org',
        '@carol:example.org',
        '@dan:example.org',
        '@eve:example.org',
      ],
      5
    ),
    'Alice, Bob, Carol and 2 others reacted with 👍'
  );
});

test('puts your own reaction first as You', () => {
  assert.equal(summary(['@alice:example.org', me]), 'You and Alice reacted with 👍');
  assert.equal(summary([me]), 'You reacted with 👍');
});

test('falls back to the localpart, then the count', () => {
  assert.equal(summary(['@zed:example.org']), 'zed reacted with 👍');
  assert.equal(summary([], 2), '2 people reacted with 👍');
  assert.equal(summary([], 1), '1 person reacted with 👍');
  assert.equal(reactionSenderName('not-an-mxid', names), 'not-an-mxid');
});

test('counts senders the readback has not listed yet', () => {
  assert.equal(summary(['@alice:example.org'], 3), 'Alice and 2 others reacted with 👍');
});
