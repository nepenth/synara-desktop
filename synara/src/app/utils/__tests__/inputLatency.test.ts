import test from 'node:test';
import assert from 'node:assert/strict';
import { isTypingKey, percentile, summarizeInputLatency } from '../inputLatency';

const key = (
  value: string,
  modifiers: Partial<Record<'ctrlKey' | 'metaKey' | 'altKey', boolean>> = {}
) => ({
  key: value,
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  ...modifiers,
});

test('nearest-rank percentiles', () => {
  const sorted = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
  assert.equal(percentile(sorted, 0.5), 5);
  assert.equal(percentile(sorted, 0.95), 10);
  assert.equal(percentile(sorted, 0), 1);
  assert.equal(percentile([], 0.5), 0);
});

test('no samples means nothing to report', () => {
  assert.equal(summarizeInputLatency([]), undefined);
});

test('a summary rounds and does not reorder the caller’s samples', () => {
  const samples = [30.4, 8.2, 12.6, 9.9];
  assert.deepEqual(summarizeInputLatency(samples), { count: 4, p50Ms: 10, p95Ms: 30, maxMs: 30 });
  assert.deepEqual(samples, [30.4, 8.2, 12.6, 9.9]);
});

test('only keys that change text are timed', () => {
  for (const value of ['a', 'Z', ' ', '7', 'Backspace', 'Delete', 'Enter']) {
    assert.equal(isTypingKey(key(value)), true, value);
  }
  for (const value of ['Shift', 'ArrowLeft', 'Tab', 'Escape', 'F5']) {
    assert.equal(isTypingKey(key(value)), false, value);
  }
  assert.equal(isTypingKey(key('a', { ctrlKey: true })), false);
  assert.equal(isTypingKey(key('a', { metaKey: true })), false);
  assert.equal(isTypingKey(key('a', { altKey: true })), false);
});
