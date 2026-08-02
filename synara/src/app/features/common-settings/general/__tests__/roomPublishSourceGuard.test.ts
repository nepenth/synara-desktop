import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const source = readFileSync(
  join(process.cwd(), 'src/app/features/common-settings/general/RoomPublish.tsx'),
  'utf8'
);

test('RoomPublish is SDK-neutral and native-only for join-rule gating', () => {
  for (const forbidden of [
    /matrix-js-sdk/,
    /MatrixError/,
    /RoomJoinRulesEventContent/,
    /(?:^|[^A-Za-z])JoinRule(?:[^A-Za-z]|$)/,
    /useMatrixClient/,
    /useStateEvent/,
    /room\.client/,
    /getSafeUserId/,
    /sendStateEvent/,
  ]) {
    assert.equal(forbidden.test(source), false, `RoomPublish retains ${forbidden}`);
  }
  assert.match(source, /useNativeRoomJoinRule/);
  assert.match(source, /roomId: string/);
  assert.match(source, /isSpace: boolean/);
  assert.match(source, /knock_restricted/);
});
