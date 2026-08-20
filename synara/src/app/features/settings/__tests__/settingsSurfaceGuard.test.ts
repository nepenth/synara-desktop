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
const notifications = readFileSync(
  join(process.cwd(), 'src/app/features/settings/notifications/Notifications.tsx'),
  'utf8'
);

test('user Settings no longer routes an Emojis & Stickers section', () => {
  assert.equal(settings.includes('EmojisStickers'), false);
  assert.equal(settings.includes('EmojisStickersPage'), false);
  assert.equal(settings.includes('Emojis & Stickers'), false);
});

test('user Settings avatar uses the native profile mxc instead of an HTTP thumbnail rewrite', () => {
  assert.equal(settings.includes('resolveMatrixThumbnailUrl'), false);
  assert.equal(profile.includes('resolveMatrixThumbnailUrl'), false);
  assert.match(hook, /getOwnProfileNative/);
  assert.match(hook, /OWN_PROFILE_CHANGED_EVENT/);
  assert.match(profile, /profile\.avatarUrl/);
  assert.match(profile, /notifyOwnProfileChanged/);
});

test('native Notifications does not point users at an unavailable Block Users editor', () => {
  assert.match(notifications, /isNativeMatrixSession/);
  assert.match(notifications, /not available in this native session/);
  const nativeBranch = notifications.slice(
    notifications.indexOf('isNativeMatrixSession()'),
    notifications.indexOf(') : (')
  );
  assert.equal(nativeBranch.includes('Account > Block'), false);
});
