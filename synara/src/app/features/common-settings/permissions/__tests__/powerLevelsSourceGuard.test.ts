import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const directory = dirname(fileURLToPath(import.meta.url));
const read = (relativePath: string) => readFileSync(join(directory, relativePath), 'utf8');

test('permissions bulk editors have no JS state-event writer or legacy branch', () => {
  const sources = [
    read('../PowersEditor.tsx'),
    read('../PermissionGroups.tsx'),
    read('../../../room-settings/permissions/Permissions.tsx'),
    read('../../../space-settings/permissions/Permissions.tsx'),
  ];
  const source = sources.join('\n');

  assert.doesNotMatch(source, /sendStateEvent|setStateEvent/);
  assert.doesNotMatch(source, /matrix_room_set_power_level(?:['"`(,]|\s)/);
  assert.doesNotMatch(source, /Legacy[A-Za-z]*|\blegacy\b|isNative\s*\?/);
  assert.match(source, /usePowerLevelTags/);
  assert.match(source, /usePowerLevels/);
});

test('native power-level owner contains both exact command names and no JS SDK import', () => {
  const source = read('../nativeRoomPowerLevelsOwner.ts');
  assert.match(source, /matrix_room_set_power_levels/);
  assert.match(source, /matrix_room_set_power_level_tags/);
  assert.doesNotMatch(source, /matrix-js-sdk|sendStateEvent|setStateEvent|Legacy|\blegacy\b/);
});
