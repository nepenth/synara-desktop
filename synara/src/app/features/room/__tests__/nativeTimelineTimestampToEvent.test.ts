import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const owner = readFileSync('src/app/features/room/nativeTimelineTimestampToEvent.ts', 'utf8');
const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
const roomView = readFileSync('src/app/features/room/RoomView.tsx', 'utf8');
const schemas = [
  '../src-tauri/gen/schemas/desktop-schema.json',
  '../src-tauri/gen/schemas/linux-schema.json',
  '../src-tauri/gen/schemas/macOS-schema.json',
].map((path) => readFileSync(path, 'utf8'));

test('date-rail jumps resolve timestamps through Core, not the JS Matrix client', () => {
  assert.match(owner, /matrix_timeline_timestamp_to_event/);
  assert.match(owner, /\{ roomId, timestampMs \}/);
  assert.match(presenter, /timestampToEventWithNativeOwner/);
  assert.match(presenter, /setFocusEventId\(found\.eventId\)/);
  assert.match(roomView, /roomCreatedTs=\{roomCreatedTs\}/);
  assert.doesNotMatch(presenter, /useMatrixClient/);
  assert.doesNotMatch(owner, /timestampToEvent\(/);
  for (const schema of schemas) {
    assert.match(schema, /allow-matrix-timeline-timestamp-to-event/);
    assert.match(schema, /matrix_timeline_timestamp_to_event/);
  }
});
