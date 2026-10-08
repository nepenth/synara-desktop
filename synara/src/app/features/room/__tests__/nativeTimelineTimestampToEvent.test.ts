import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const owner = readFileSync('src/app/features/room/nativeTimelineTimestampToEvent.ts', 'utf8');
const jumpToTime = readFileSync('src/app/features/room/jump-to-time/JumpToTime.tsx', 'utf8');
const schemas = [
  '../src-tauri/gen/schemas/desktop-schema.json',
  '../src-tauri/gen/schemas/linux-schema.json',
  '../src-tauri/gen/schemas/macOS-schema.json',
].map((path) => readFileSync(path, 'utf8'));

test('jump-to-time resolves timestamps through Core, not the JS Matrix client', () => {
  assert.match(owner, /matrix_timeline_timestamp_to_event/);
  assert.match(owner, /\{ roomId, timestampMs \}/);
  assert.doesNotMatch(owner, /useMatrixClient/);
  assert.doesNotMatch(owner, /timestampToEvent\(/);
  assert.match(jumpToTime, /timestampToEventWithNativeOwner\(room\.roomId, newTs\)/);
  assert.doesNotMatch(jumpToTime, /mx\.timestampToEvent/);
  for (const schema of schemas) {
    assert.match(schema, /allow-matrix-timeline-timestamp-to-event/);
    assert.match(schema, /matrix_timeline_timestamp_to_event/);
  }
});
