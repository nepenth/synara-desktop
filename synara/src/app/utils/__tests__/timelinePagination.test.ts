import assert from 'node:assert/strict';
import test from 'node:test';
import {
  clearTimelinePaginationError,
  createTimelinePaginationErrorMessage,
  setTimelinePaginationError,
  shouldShowTimelinePaginationLoader,
} from '../timelinePagination';

test('timeline pagination error setter stores direction-specific messages', () => {
  const next = setTimelinePaginationError({}, 'backward', new Error('network down'));
  assert.deepEqual(next, { backward: 'network down' });
});

test('timeline pagination error setter falls back to generic message', () => {
  const next = setTimelinePaginationError({}, 'forward', 'boom');
  assert.deepEqual(next, { forward: 'Failed to load messages.' });
});

test('timeline pagination loader hides when direction has an error', () => {
  assert.equal(
    shouldShowTimelinePaginationLoader(true, { backward: 'network down' }, 'backward'),
    false
  );
  assert.equal(
    shouldShowTimelinePaginationLoader(true, { backward: 'network down' }, 'forward'),
    true
  );
});

test('timeline pagination error clear removes only the requested direction', () => {
  const cleared = clearTimelinePaginationError(
    { backward: 'network down', forward: 'timeout' },
    'backward'
  );
  assert.deepEqual(cleared, { forward: 'timeout' });
});

test('timeline pagination error message helper preserves Error text', () => {
  assert.equal(createTimelinePaginationErrorMessage(new Error('rate limited')), 'rate limited');
});
