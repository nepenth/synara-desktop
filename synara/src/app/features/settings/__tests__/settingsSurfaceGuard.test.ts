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

const appearance = readFileSync(
  join(process.cwd(), 'src/app/features/settings/appearance/Appearance.tsx'),
  'utf8'
);
const general = readFileSync(
  join(process.cwd(), 'src/app/features/settings/general/General.tsx'),
  'utf8'
);
const about = readFileSync(
  join(process.cwd(), 'src/app/features/settings/about/About.tsx'),
  'utf8'
);

test('appearance does not offer Twitter Emoji and does not advertise unused mint as the current accent', () => {
  const accent = readFileSync(join(process.cwd(), 'src/app/utils/themeAccent.ts'), 'utf8');
  assert.equal(appearance.includes('Twitter Emoji'), false);
  assert.equal(appearance.includes('twitterEmoji'), false);
  assert.equal(appearance.includes('#6bdbb8'), false);
  assert.equal(accent.includes('#6bdbb8'), false);
  assert.match(appearance, /themeDefaultAccentColor/);
  assert.match(appearance, /Sample/);
});

test('native Appearance does not offer Compact or Bubble layouts that the native timeline ignores', () => {
  assert.match(appearance, /isNativeMatrixSession\(\)/);
  assert.match(appearance, /native timeline uses a single Element-like layout/);
});

test('Appearance is its own Settings page and General no longer hosts theme or layout controls', () => {
  assert.match(settings, /SettingsPages\.AppearancePage/);
  assert.match(settings, /name: 'Appearance'/);
  assert.match(settings, /<AppearanceSettings requestClose=/);
  assert.equal(general.includes('useTheme'), false);
  assert.equal(general.includes('Message Layout'), false);
  assert.equal(general.includes('legacyUsernameColor'), false);
  assert.match(appearance, /Message Layout/);
  assert.match(appearance, /Legacy Username Color/);
});

test('maintenance actions live once, under General > Storage, not duplicated in About', () => {
  assert.match(general, /<Text size="L400">Storage<\/Text>/);
  assert.match(general, /Clear Cache & Reload/);
  assert.match(general, /isDesktopPlatform\(\) && <SecretStoreTile \/>/);
  assert.equal(about.includes('Clear Cache'), false);
  assert.equal(about.includes('UpdateSettingsTile'), false);
  assert.equal(about.includes('Options'), false);
  // Update checking has exactly one home: General > Software Updates.
  assert.match(general, /Software Updates/);
  assert.match(general, /<UpdateSettingsTile \/>/);
});

test('General settings use positive phrasing for media loading', () => {
  assert.match(general, /title="Load Media Automatically"/);
  assert.equal(general.includes('Disable Media Auto Load'), false);
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
