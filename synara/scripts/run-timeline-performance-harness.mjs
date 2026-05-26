import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';

const makeRows = (eventCount) => {
  const rows = [{ kind: 'loader', key: 'loader:backward' }];
  for (let i = 0; i < eventCount; i += 1) {
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

const measureScenario = (eventCount) => {
  const startedAt = performance.now();
  const rows = makeRows(eventCount);
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
    firstEventRow: eventIndexToRowIndex.get(0),
    lastEventRow: eventIdToRowIndex.get(`$synthetic-${eventCount - 1}`),
  };
};

const scenarios = [10_000, 50_000].map(measureScenario);

for (const scenario of scenarios) {
  assert.equal(typeof scenario.firstEventRow, 'number');
  assert.equal(typeof scenario.lastEventRow, 'number');
  assert.ok(scenario.rows <= scenario.eventCount + Math.ceil(scenario.eventCount / 250) + 2);
  assert.ok(
    scenario.durationMs < (scenario.eventCount === 10_000 ? 75 : 300),
    `timeline harness too slow for ${scenario.eventCount}: ${scenario.durationMs.toFixed(2)}ms`
  );
}

console.table(
  scenarios.map((scenario) => ({
    events: scenario.eventCount,
    rows: scenario.rows,
    duration_ms: Number(scenario.durationMs.toFixed(2)),
  }))
);
