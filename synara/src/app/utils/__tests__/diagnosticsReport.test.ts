import assert from 'node:assert/strict';
import test from 'node:test';
import { compactDiagnosticsReport, MAX_CLIPBOARD_REPORT_CHARS } from '../diagnosticsReport';

test('compact diagnostics keeps small reports unchanged', () => {
  const report = JSON.stringify({ schemaVersion: 1, entries: [{ sequence: 1 }] });
  assert.equal(compactDiagnosticsReport(report), report);
});

test('compact diagnostics drops the oldest entries and preserves valid JSON', () => {
  const entries = Array.from({ length: 2_000 }, (_, sequence) => ({
    sequence,
    event: 'room-timeline.unexpected-scroll-jump',
    padding: 'x'.repeat(180),
  }));
  const compact = compactDiagnosticsReport(
    JSON.stringify({ schemaVersion: 1, generatedAtMs: 1, entries }, null, 2)
  );
  const parsed = JSON.parse(compact) as {
    clipboardTruncated: boolean;
    entries: Array<{ sequence: number }>;
  };

  assert.ok(compact.length <= MAX_CLIPBOARD_REPORT_CHARS);
  assert.equal(parsed.clipboardTruncated, true);
  assert.ok(parsed.entries.length > 0);
  assert.ok(parsed.entries.length < entries.length);
  assert.equal(parsed.entries.at(-1)?.sequence, entries.at(-1)?.sequence);
});

test('compact diagnostics bounds legacy non-JSON report tails', () => {
  const report = `${'discard-me'.repeat(40_000)}\n${'tail'.repeat(70_000)}`;
  const compact = compactDiagnosticsReport(report);
  assert.ok(compact.length <= MAX_CLIPBOARD_REPORT_CHARS);
  assert.equal(compact.includes('discard-me'), false);
});
