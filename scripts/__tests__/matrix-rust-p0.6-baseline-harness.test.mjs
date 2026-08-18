import assert from 'node:assert/strict';
import test from 'node:test';

import {
  percentile,
  runTimelineBaseline,
  summarizeDurations,
} from '../matrix-rust-p0.6-baseline-harness.mjs';

test('percentile nearest-rank for small samples', () => {
  const sorted = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
  assert.equal(percentile(sorted, 50), 5);
  assert.equal(percentile(sorted, 95), 10);
  assert.equal(percentile(sorted, 100), 10);
  assert.equal(percentile([42], 50), 42);
  assert.equal(percentile([42], 95), 42);
});

test('percentile rejects empty or invalid p', () => {
  assert.throws(() => percentile([], 50));
  assert.throws(() => percentile([1], 0));
  assert.throws(() => percentile([1], 101));
});

test('summarizeDurations computes ordered stats', () => {
  const stats = summarizeDurations([0.2, 0.1, 0.3, 0.15, 0.25]);
  assert.equal(stats.n, 5);
  assert.equal(stats.min_ms, 0.1);
  assert.equal(stats.max_ms, 0.3);
  assert.equal(stats.p50_ms, 0.2);
  assert.ok(stats.mean_ms > 0.19 && stats.mean_ms < 0.21);
});

test('runTimelineBaseline multi-iteration stays under budget', () => {
  const result = runTimelineBaseline({ iterations: 5 });
  assert.equal(result.iterations, 5);
  assert.equal(result.scenarios.length, 2);
  for (const scenario of result.scenarios) {
    assert.equal(scenario.n, 5);
    assert.ok(scenario.p50_ms < 25);
    assert.ok(scenario.max_ms < 250);
    assert.equal(scenario.rendered_events, 200);
    assert.equal(scenario.samples_ms.length, 5);
  }
});

test('runTimelineBaseline still rejects a sustained budget regression', () => {
  assert.throws(
    () => runTimelineBaseline({ iterations: 5, scenarios: [10_000], budgetMs: Number.EPSILON }),
    /median too slow/
  );
});
