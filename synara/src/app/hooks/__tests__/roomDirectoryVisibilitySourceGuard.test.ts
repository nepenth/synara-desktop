import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const read = (relativePath: string): string =>
  readFileSync(join(process.cwd(), relativePath), 'utf8');

test('directory visibility has one native room-profile route and no JS visibility importer', () => {
  const hook = read('src/app/hooks/useRoomDirectoryVisibility.ts');
  const owner = read('src/app/features/common-settings/general/nativeRoomProfileOwner.ts');
  const nativeProfile = read('src/app/features/common-settings/general/nativeRoomProfile.ts');
  const caller = read('src/app/features/common-settings/general/RoomPublish.tsx');

  assert.match(hook, /getRoomDirectoryVisibilityNative/);
  assert.match(hook, /setRoomDirectoryVisibilityNative/);
  assert.match(hook, /getActiveSession\(\)\?\.sessionGeneration/);
  assert.match(hook, /\[roomId, sessionGeneration\]/);
  assert.match(hook, /data\.sessionGeneration === sessionGeneration/);
  assert.match(hook, /\{ status: AsyncStatus\.Loading \}/);
  assert.ok(
    hook.indexOf('await setRoomDirectoryVisibilityNative') <
      hook.indexOf(
        'await loadVisibility()',
        hook.indexOf('await setRoomDirectoryVisibilityNative')
      ),
    'the UI must read authoritative visibility after the native write'
  );
  for (const source of [hook, owner, nativeProfile]) {
    assert.doesNotMatch(source, /matrix-js-sdk/);
    assert.doesNotMatch(source, /fetch\(|axios|authedRequest|mx\.http/);
  }
  assert.doesNotMatch(
    hook,
    /useMatrixClient|mx\.getRoomDirectoryVisibility|mx\.setRoomDirectoryVisibility/
  );
  assert.match(caller, /useRoomDirectoryVisibility/);
  assert.match(caller, /canEditCanonical/);
  assert.match(caller, /validRule/);
  assert.match(caller, /AsyncStatus\.Loading/);
  assert.match(caller, /AsyncStatus\.Error/);
  assert.doesNotMatch(caller, /mx\.getRoomDirectoryVisibility|mx\.setRoomDirectoryVisibility/);
});
