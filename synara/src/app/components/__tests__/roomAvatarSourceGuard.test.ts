import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const source = readFileSync(
  join(process.cwd(), 'src/app/components/room-avatar/RoomAvatar.tsx'),
  'utf8'
);

test('RoomAvatar presentation boundary has no direct matrix-js-sdk importer', () => {
  assert.doesNotMatch(source, /matrix-js-sdk/);
  assert.match(source, /RoomJoinRulePresentation/);
  assert.match(source, /getRoomIconSrc/);
  assert.match(source, /useNativeMatrixMediaSrc/);
});
