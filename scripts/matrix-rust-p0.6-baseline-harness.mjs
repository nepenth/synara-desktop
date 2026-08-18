#!/usr/bin/env node
/**
 * P0.6 baseline aggregator for automated timeline row-mapping harness.
 *
 * Re-runs the same synthetic mapping logic as
 * `synara/scripts/run-timeline-performance-harness.mjs` for N iterations and
 * emits machine-readable p50/p95. Does not change product behavior.
 *
 * Usage:
 *   node scripts/matrix-rust-p0.6-baseline-harness.mjs [--iterations N] [--json]
 */
import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';

const MAX_RENDERED_EVENTS = 200;
const DEFAULT_ITERATIONS = 50;
const BUDGET_MS = 25;
const SCENARIOS = [10_000, 50_000];

export const makeRows = (start, end) => {
  const rows = [{ kind: 'loader', key: 'loader:backward' }];
  for (let i = start; i < end; i += 1) {
    if (i > 0 && i % 250 === 0) {
      rows.push({ kind: 'divider', key: `divider:day:${i}` });
    }
    rows.push({
      kind: 'event',
      key: `$synthetic-${i}`,
      eventId: `$synthetic-${i}`,
      eventIndex: i,
    });
  }
  rows.push({ kind: 'bottom', key: 'bottom' });
  return rows;
};

/** Same mapping path measured by the product timeline performance harness. */
export const measureScenarioOnce = (eventCount) => {
  const rangeStart = Math.max(0, eventCount - MAX_RENDERED_EVENTS);
  const rangeEnd = eventCount;
  const startedAt = performance.now();
  const rows = makeRows(rangeStart, rangeEnd);
  const eventIdToRowIndex = new Map();
  const eventIndexToRowIndex = new Map();
  rows.forEach((row, rowIndex) => {
    if (row.kind !== 'event') return;
    eventIdToRowIndex.set(row.eventId, rowIndex);
    eventIndexToRowIndex.set(row.eventIndex, rowIndex);
  });
  const durationMs = performance.now() - startedAt;
  return {
    eventCount,
    rows: rows.length,
    durationMs,
    renderedEvents: rangeEnd - rangeStart,
    firstEventRow: eventIndexToRowIndex.get(rangeStart),
    lastEventRow: eventIdToRowIndex.get(`$synthetic-${eventCount - 1}`),
  };
};

/**
 * Nearest-rank percentile on a sorted ascending sample.
 * p in (0, 100]; empty array throws.
 */
export const percentile = (sortedAscending, p) => {
  if (!Array.isArray(sortedAscending) || sortedAscending.length === 0) {
    throw new Error('percentile requires a non-empty array');
  }
  if (p <= 0 || p > 100) {
    throw new Error('percentile p must be in (0, 100]');
  }
  const rank = Math.ceil((p / 100) * sortedAscending.length) - 1;
  const index = Math.min(sortedAscending.length - 1, Math.max(0, rank));
  return sortedAscending[index];
};

export const summarizeDurations = (durationsMs) => {
  const sorted = [...durationsMs].sort((a, b) => a - b);
  const sum = sorted.reduce((acc, v) => acc + v, 0);
  return {
    n: sorted.length,
    min_ms: sorted[0],
    max_ms: sorted[sorted.length - 1],
    mean_ms: sum / sorted.length,
    p50_ms: percentile(sorted, 50),
    p95_ms: percentile(sorted, 95),
    samples_ms: sorted,
  };
};

export const runTimelineBaseline = ({
  iterations = DEFAULT_ITERATIONS,
  scenarios = SCENARIOS,
  budgetMs = BUDGET_MS,
} = {}) => {
  if (!Number.isInteger(iterations) || iterations < 1) {
    throw new Error('iterations must be a positive integer');
  }

  const scenarioResults = [];

  for (const eventCount of scenarios) {
    const durations = [];
    let last = null;
    for (let i = 0; i < iterations; i += 1) {
      const sample = measureScenarioOnce(eventCount);
      assert.equal(typeof sample.firstEventRow, 'number');
      assert.equal(typeof sample.lastEventRow, 'number');
      assert.equal(sample.renderedEvents, MAX_RENDERED_EVENTS);
      assert.ok(sample.rows <= MAX_RENDERED_EVENTS + 3);
      durations.push(sample.durationMs);
      last = sample;
    }
    const stats = summarizeDurations(durations);
    // A single wall-clock sample can include scheduler preemption, especially
    // when CI runs Rust and Node jobs concurrently. Gate the median so a real
    // sustained regression still fails without making host contention flaky.
    assert.ok(
      stats.p50_ms < budgetMs,
      `timeline harness median too slow for ${eventCount}: ${stats.p50_ms.toFixed(2)}ms (budget ${budgetMs}ms)`
    );
    scenarioResults.push({
      metric_id: `M-TIMELINE-MAP-${eventCount}`,
      label: `synthetic timeline row map (${eventCount} events, rendered window ${MAX_RENDERED_EVENTS})`,
      event_count: eventCount,
      rendered_events: MAX_RENDERED_EVENTS,
      rows: last.rows,
      budget_ms: budgetMs,
      unit: 'ms',
      ...stats,
      // Keep full sample arrays out of compact console JSON unless requested
      samples_ms: stats.samples_ms.map((v) => Number(v.toFixed(6))),
    });
  }

  return {
    harness: 'matrix-rust-p0.6-baseline-harness',
    product_harness: 'synara/scripts/run-timeline-performance-harness.mjs',
    measured_surface:
      'synthetic timeline row key/index mapping only (not end-to-end UX latency)',
    iterations,
    budget_ms: budgetMs,
    scenarios: scenarioResults,
    captured_at: new Date().toISOString(),
  };
};

const parseArgs = (argv) => {
  let iterations = DEFAULT_ITERATIONS;
  let json = false;
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--json') {
      json = true;
    } else if (arg === '--iterations') {
      iterations = Number.parseInt(argv[i + 1], 10);
      i += 1;
    } else if (arg.startsWith('--iterations=')) {
      iterations = Number.parseInt(arg.slice('--iterations='.length), 10);
    } else if (arg === '--help' || arg === '-h') {
      console.log(
        'Usage: node scripts/matrix-rust-p0.6-baseline-harness.mjs [--iterations N] [--json]'
      );
      process.exit(0);
    }
  }
  return { iterations, json };
};

const main = () => {
  const { iterations, json } = parseArgs(process.argv.slice(2));
  const result = runTimelineBaseline({ iterations });

  if (json) {
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  console.log(
    `P0.6 timeline mapping baseline — iterations=${result.iterations} budget_ms=${result.budget_ms}`
  );
  console.table(
    result.scenarios.map((s) => ({
      events: s.event_count,
      rendered: s.rendered_events,
      rows: s.rows,
      n: s.n,
      min_ms: Number(s.min_ms.toFixed(3)),
      p50_ms: Number(s.p50_ms.toFixed(3)),
      p95_ms: Number(s.p95_ms.toFixed(3)),
      max_ms: Number(s.max_ms.toFixed(3)),
      mean_ms: Number(s.mean_ms.toFixed(3)),
    }))
  );
  console.log(
    'Note: this is an automated proxy for the virtualization row-mapping layer only.'
  );
};

const entryPath = process.argv[1] ? process.argv[1].replace(/\\/g, '/') : '';
const isDirectRun =
  entryPath.endsWith('/matrix-rust-p0.6-baseline-harness.mjs') ||
  entryPath.endsWith('matrix-rust-p0.6-baseline-harness.mjs');

if (isDirectRun) {
  main();
}
