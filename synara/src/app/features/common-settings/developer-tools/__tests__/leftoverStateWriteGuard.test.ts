import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const read = (relativePath: string) =>
  readFileSync(join(process.cwd(), 'src/app', relativePath), 'utf8');

test('leftover encryptable state writes fail-closed to the native owner', () => {
  const aliases = read('hooks/useRoomAliases.ts');
  const editor = read('features/common-settings/developer-tools/StateEventEditor.tsx');
  const send = read('features/common-settings/developer-tools/SendRoomEvent.tsx');

  for (const [label, source] of [
    ['useRoomAliases', aliases],
    ['StateEventEditor', editor],
    ['SendRoomEvent', send],
  ] as const) {
    assert.match(source, /sendLeftoverStateEvent/, `${label} must native-or-block leftover writes`);
  }

  assert.match(aliases, /StateEvent\.RoomCanonicalAlias/);
});

test('slash commands write room state through Core, not the renderer', () => {
  // /acl, /myroomnick and /myroomavatar run in Core's composer core
  // (crates/synara-core/src/app/composer), which writes through its room owner.
  assert.equal(existsSync(join(process.cwd(), 'src/app/hooks/useCommands.ts')), false);
  const roomInput = read('features/room/RoomInput.tsx');
  assert.match(roomInput, /runComposerCommand\(/);
  assert.doesNotMatch(roomInput, /StateEvent\.RoomServerAcl|StateEvent\.RoomMember/);
});
