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

test('appearance does not offer Twitter Emoji and does not advertise unused mint as the current accent', () => {
  const general = readFileSync(
    join(process.cwd(), 'src/app/features/settings/general/General.tsx'),
    'utf8'
  );
  const accent = readFileSync(join(process.cwd(), 'src/app/utils/themeAccent.ts'), 'utf8');
  assert.equal(general.includes('Twitter Emoji'), false);
  assert.equal(general.includes('twitterEmoji'), false);
  assert.equal(general.includes('#6bdbb8'), false);
  assert.equal(accent.includes('#6bdbb8'), false);
  assert.match(general, /themeDefaultAccentColor/);
  assert.match(general, /Sample/);
});

test('native General does not offer Compact or Bubble layouts that the native timeline ignores', () => {
  const general = readFileSync(
    join(process.cwd(), 'src/app/features/settings/general/General.tsx'),
    'utf8'
  );
  assert.match(general, /isNativeMatrixSession\(\)/);
  assert.match(general, /native timeline uses a single Element-like layout/);
});

test('native Notifications owns homeserver push rules instead of a unavailable stub', () => {
  assert.match(notifications, /isNativeMatrixSession/);
  assert.match(notifications, /NativePushRulesEditor/);
  assert.equal(notifications.includes('not available in this native session'), false);
  const nativeBranch = notifications.slice(
    notifications.indexOf('isNativeMatrixSession()'),
    notifications.indexOf(') : (')
  );
  assert.equal(nativeBranch.includes('Account > Block'), false);
});
