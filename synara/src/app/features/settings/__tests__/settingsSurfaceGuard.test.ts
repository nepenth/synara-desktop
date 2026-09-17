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
  assert.match(hook, /subscribeOwnProfileNativePush/);
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

test('settings left nav stacks categories with a gap on the reading plane', () => {
  const nav = settings.slice(
    settings.indexOf('<PageNavContent>'),
    settings.indexOf('<Text size="B400">Logout</Text>')
  );
  assert.match(nav, /<Box direction="Column" gap="(?:100|200|300|400)"/);
  assert.match(nav, /fill="None"/);
  assert.match(nav, /quietInteractiveSurface/);
  assert.match(nav, /variant="Surface"/);
});

test('native Appearance hides Message Layout and does not name other clients', () => {
  const messages = appearance.slice(
    appearance.indexOf('function MessageDisplay'),
    appearance.indexOf('title="Legacy Username Color"')
  );
  assert.match(messages, /!isNativeMatrixSession\(\)/);
  assert.match(messages, /<SelectMessageLayout \/>/);
  assert.match(messages, /Message Spacing/);
  assert.doesNotMatch(messages, /title="Message layout"/);
  assert.doesNotMatch(messages, /Other layouts are not available yet/);
  assert.doesNotMatch(appearance, /Element-like/);
  assert.doesNotMatch(appearance, /\bElement\b/);
  assert.equal(appearance.includes('Cinny'), false);
  assert.equal(appearance.includes('FluffyChat'), false);
  assert.doesNotMatch(appearance, /retired JS timeline/);
  assert.doesNotMatch(appearance, /<Text size="T300">Modern<\/Text>/);
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

test('General hosts Indexed message search with a session-reload note', () => {
  assert.match(general, /Indexed message search/);
  assert.match(general, /indexedMessageSearch/);
  assert.match(general, /session reload/);
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

const devices = readFileSync(
  join(process.cwd(), 'src/app/features/settings/devices/Devices.tsx'),
  'utf8'
);
const developerTools = readFileSync(
  join(process.cwd(), 'src/app/features/settings/developer-tools/DevelopTools.tsx'),
  'utf8'
);

test('General hosts the first-class Widgets card and Developer Tools does not duplicate it', () => {
  assert.match(general, /<Text size="L400">Widgets<\/Text>/);
  assert.match(general, /experimentalWidgetsEnabled/);
  assert.match(general, /agentWidgetEntries/);
  assert.match(general, /widgetsSettingsDescription/);
  assert.match(general, /Add agent widget/);
  assert.match(developerTools, /Widgets are configured under General/);
  assert.equal(developerTools.includes('Add agent widget'), false);
  assert.equal(developerTools.includes('setAgentWidgetEntries'), false);
  assert.equal(developerTools.includes('setExperimentalWidgetsEnabled'), false);
});

test('General hosts a Calls card with honest MatrixRTC transport status', () => {
  assert.match(general, /<Text size="L400">Calls<\/Text>/);
  assert.match(general, /rtcCallAvailabilityCopy/);
  assert.match(general, /snapshotRtcTransportsNative/);
  assert.equal(general.includes('set_call'), false);
});

test('X.509 identity settings live on Devices, not Developer Tools', () => {
  assert.match(devices, /X509IdentityCard/);
  assert.match(devices, /isNativeMatrixSession/);
  assert.equal(developerTools.includes('X509'), false);
  assert.equal(developerTools.includes('x509'), false);
  assert.equal(developerTools.includes('X.509'), false);
  const x509Card = readFileSync(
    join(process.cwd(), 'src/app/features/settings/devices/X509IdentityCard.tsx'),
    'utf8'
  );
  assert.match(x509Card, /shares encrypted room keys/);
  assert.match(x509Card, /Reload session/);
  assert.equal(x509Card.includes('settingsAtom'), false);
});
