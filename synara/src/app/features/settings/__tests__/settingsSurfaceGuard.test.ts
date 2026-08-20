import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';

const settings = readFileSync(
  join(process.cwd(), 'src/app/features/settings/Settings.tsx'),
  'utf8'
);
const profile = readFileSync(
  join(process.cwd(), 'src/app/features/settings/account/Profile.tsx'),
  'utf8'
);
const hook = readFileSync(join(process.cwd(), 'src/app/hooks/useUserProfile.ts'), 'utf8');

test('user Settings no longer routes an Emojis & Stickers section', () => {
  assert.equal(settings.includes('EmojisStickers'), false);
  assert.equal(settings.includes('EmojisStickersPage'), false);
  assert.equal(settings.includes('Emojis & Stickers'), false);
});

test('user Settings avatar uses the native profile mxc instead of an HTTP thumbnail rewrite', () => {
  assert.equal(settings.includes('resolveMatrixThumbnailUrl'), false);
  assert.equal(profile.includes('resolveMatrixThumbnailUrl'), false);
  assert.match(hook, /getOwnProfileNative/);
  assert.match(profile, /profile\.avatarUrl/);
});
