import assert from 'node:assert/strict';
import test from 'node:test';
import {
  canPaginateTimelineForward,
  clearTimelinePaginationError,
  createTimelinePaginationErrorMessage,
  isTimelinePaginationLoading,
  resolveTimelineHistoryOverlay,
  setTimelinePaginationError,
  shouldPaginateOnWheel,
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

test('timeline pagination error message helper reads structured command errors', () => {
  assert.equal(
    createTimelinePaginationErrorMessage({ message: 'homeserver timeout' }),
    'homeserver timeout'
  );
});

test('pagination loading is true for local in-flight even when native is still available', () => {
  assert.equal(
    isTimelinePaginationLoading({ nativeState: 'available', inFlight: true, hasError: false }),
    true
  );
  assert.equal(
    isTimelinePaginationLoading({ nativeState: 'loading', inFlight: false, hasError: false }),
    true
  );
  assert.equal(
    isTimelinePaginationLoading({ nativeState: 'loading', inFlight: true, hasError: true }),
    false
  );
});

test('history overlay prefers error over loading and hides the spinner', () => {
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'loading',
      inFlight: true,
      error: 'homeserver timeout',
      atEdge: true,
      canPaginate: true,
    }),
    { kind: 'error', message: 'homeserver timeout' }
  );
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'available',
      inFlight: true,
      atEdge: false,
      canPaginate: true,
    }),
    { kind: 'hidden' }
  );
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'available',
      inFlight: false,
      atEdge: true,
      canPaginate: true,
    }),
    { kind: 'load_more' }
  );
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'available',
      inFlight: false,
      atEdge: true,
      canPaginate: true,
      hasSparseLoadButton: true,
    }),
    { kind: 'hidden' }
  );
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'loading',
      inFlight: true,
      atEdge: true,
      canPaginate: true,
    }),
    { kind: 'loading' }
  );
});

test('live bottom never requests forward pagination', () => {
  assert.equal(
    canPaginateTimelineForward({ atLiveBottom: true, positionKind: 'live_bottom' }),
    false
  );
  assert.equal(
    canPaginateTimelineForward({ atLiveBottom: false, positionKind: 'live_bottom' }),
    false
  );
  assert.equal(canPaginateTimelineForward({ atLiveBottom: true, positionKind: 'restored' }), false);
  assert.equal(canPaginateTimelineForward({ atLiveBottom: true, positionKind: 'focused' }), true);
  assert.equal(canPaginateTimelineForward({ atLiveBottom: true, positionKind: 'unread' }), true);
  assert.equal(
    canPaginateTimelineForward({
      atLiveBottom: false,
      positionKind: 'restored',
      followingLive: true,
    }),
    false
  );
  assert.equal(
    shouldPaginateOnWheel({
      deltaY: 120,
      atLiveBottom: true,
      positionKind: 'restored',
    }),
    false
  );
  assert.equal(
    shouldPaginateOnWheel({
      deltaY: -120,
      atLiveBottom: true,
      positionKind: 'live_bottom',
    }),
    true
  );
  assert.equal(
    shouldPaginateOnWheel({
      deltaY: 120,
      atLiveBottom: false,
      positionKind: 'focused',
    }),
    true
  );
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'available',
      inFlight: true,
      atEdge: false,
      canPaginate: true,
    }),
    { kind: 'hidden' }
  );
});

test('live tail hides forward overlay even when native forward is loading at the edge', () => {
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'loading',
      inFlight: false,
      atEdge: true,
      canPaginate: false,
    }),
    { kind: 'hidden' }
  );
  assert.deepEqual(
    resolveTimelineHistoryOverlay({
      nativeState: 'loading',
      inFlight: true,
      atEdge: true,
      canPaginate: false,
    }),
    { kind: 'hidden' }
  );
});
