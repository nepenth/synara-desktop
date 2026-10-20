import assert from 'node:assert/strict';
import test from 'node:test';
import {
  BulkActionError,
  bulkFailureMessage,
  listBulkFailureLabels,
  runBulkAction,
} from '../bulkActions';
import { rateLimitedActions } from '../matrix';

test('a failed item does not stop the rest of a bulk action', async () => {
  const done: string[] = [];
  const failures = await runBulkAction(['a', 'b', 'c'], async (item) => {
    if (item === 'b') throw { code: 'forbidden', message: 'Not allowed' };
    done.push(item);
  });
  assert.deepEqual(done, ['a', 'c']);
  assert.deepEqual(
    failures.map((failure) => failure.item),
    ['b']
  );
  assert.deepEqual(failures[0]?.error, { code: 'forbidden', message: 'Not allowed' });
});

test('bulk failure copy says how many succeeded and which failed', () => {
  const copy = { done: 'Declined', verb: 'decline', noun: 'invites' };
  assert.equal(
    bulkFailureMessage(3, ['Spam Room'], copy),
    "Declined 2 of 3 invites. Couldn't decline Spam Room. Try again."
  );
  assert.equal(bulkFailureMessage(2, ['A', 'B'], copy), "Couldn't decline A and B. Try again.");
  assert.equal(listBulkFailureLabels(['A', 'B', 'C']), 'A, B and C');
  assert.equal(listBulkFailureLabels(['A', 'B', 'C', 'D', 'E']), 'A, B, C and 2 more');
  const error = new BulkActionError('message', [{ item: 'x', error: new Error('boom') }]);
  assert.equal(error.message, 'message');
  assert.equal(error.failures.length, 1);
});

test('rateLimitedActions reports failures instead of swallowing them', async () => {
  const seen: number[] = [];
  const failures = await rateLimitedActions([1, 2, 3], async (item) => {
    seen.push(item);
    // Core rejects with a plain object, never a MatrixError.
    if (item === 2) throw { code: 'space_child_failed', message: 'Space child write failed.' };
  });
  assert.deepEqual(seen, [1, 2, 3]);
  assert.deepEqual(
    failures.map((failure) => failure.item),
    [2]
  );
  assert.deepEqual(await rateLimitedActions([1], async () => undefined), []);
});
