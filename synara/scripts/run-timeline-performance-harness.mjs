import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';

const MAX_RENDERED_EVENTS = 200;

const makeRows = (start, end) => {
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

const measureScenario = (eventCount) => {
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

const scenarios = [10_000, 50_000].map(measureScenario);

for (const scenario of scenarios) {
  assert.equal(typeof scenario.firstEventRow, 'number');
  assert.equal(typeof scenario.lastEventRow, 'number');
  assert.equal(scenario.renderedEvents, MAX_RENDERED_EVENTS);
  assert.ok(scenario.rows <= MAX_RENDERED_EVENTS + 3);
  assert.ok(
    scenario.durationMs < 25,
    `timeline harness too slow for ${scenario.eventCount}: ${scenario.durationMs.toFixed(2)}ms`
  );
}

console.table(
  scenarios.map((scenario) => ({
    events: scenario.eventCount,
    rendered_events: scenario.renderedEvents,
    rows: scenario.rows,
    duration_ms: Number(scenario.durationMs.toFixed(2)),
  }))
);
