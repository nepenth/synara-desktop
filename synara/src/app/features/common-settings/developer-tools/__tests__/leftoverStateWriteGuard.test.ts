import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const read = (relativePath: string) =>
  readFileSync(join(process.cwd(), 'src/app', relativePath), 'utf8');

test('leftover encryptable state writes fail-closed to the native owner', () => {
  const aliases = read('hooks/useRoomAliases.ts');
  const editor = read('features/common-settings/developer-tools/StateEventEditor.tsx');
  const send = read('features/common-settings/developer-tools/SendRoomEvent.tsx');
  const commands = read('hooks/useCommands.ts');

  for (const [label, source] of [
    ['useRoomAliases', aliases],
    ['StateEventEditor', editor],
    ['SendRoomEvent', send],
    ['useCommands', commands],
  ] as const) {
    assert.match(source, /sendLeftoverStateEvent/, `${label} must native-or-block leftover writes`);
  }

  assert.match(aliases, /StateEvent\.RoomCanonicalAlias/);
  assert.match(commands, /StateEvent\.RoomServerAcl/);
  assert.match(commands, /sendLeftoverStateEvent\(/);
  assert.match(commands, /StateEvent\.RoomMember/);
  const memberNick = commands.split('[Command.MyRoomNick]')[1]?.split('[Command.MyRoomAvatar]')[0];
  assert.ok(memberNick);
  assert.match(memberNick, /c\.sendStateEvent/);
  assert.doesNotMatch(memberNick, /sendLeftoverStateEvent/);
});
