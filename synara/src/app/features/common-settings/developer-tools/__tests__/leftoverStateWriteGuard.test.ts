import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const directory = dirname(fileURLToPath(import.meta.url));
const read = (relativePath: string) => readFileSync(join(directory, relativePath), 'utf8');

test('leftover encryptable state writes fail-closed to the native owner', () => {
  const aliases = read('../../../../hooks/useRoomAliases.ts');
  const editor = read('../StateEventEditor.tsx');
  const send = read('../SendRoomEvent.tsx');
  const commands = read('../../../../hooks/useCommands.ts');

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
