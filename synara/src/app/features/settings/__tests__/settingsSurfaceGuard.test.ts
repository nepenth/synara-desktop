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
  // App, Room and Space settings share one grouped nav; selection uses the
  // room list's NavItem so the selected state matches everywhere.
  assert.match(settings, /<SettingsNav/);
  assert.match(settings, /label: 'App'/);
  assert.match(settings, /label: 'Chat'/);
  const layout = readFileSync(
    join(process.cwd(), 'src/app/components/settings-layout/SettingsLayout.tsx'),
    'utf8'
  );
  assert.match(layout, /<NavItem\s+key=\{item\.name\}\s+variant="Background"/);
  // `selected` styles the row; the button inside carries aria-current (D58).
  assert.match(layout, /selected=\{selected\}/);
  assert.doesNotMatch(layout, /aria-selected/);
  assert.match(layout, /aria-current=\{selected \? 'page' : undefined\}/);
  assert.match(layout, /data-settings-nav/);
});

test('Log out stays in the settings nav footer', () => {
  assert.match(settings, /footer=\{/);
  assert.match(settings, /<Text size="B400">Log out<\/Text>/);
  assert.match(settings, /<LogoutDialog/);
});

test('native Appearance hides Message Layout and does not name other clients', () => {
  const messages = appearance.slice(
    appearance.indexOf('function MessageDisplay'),
    appearance.indexOf('title="Legacy username color"')
  );
  assert.match(messages, /!isNativeMatrixSession\(\)/);
  assert.match(messages, /<SelectMessageLayout \/>/);
  assert.match(messages, /Message spacing/);
  // Title Case is gone from the Appearance rows.
  assert.doesNotMatch(messages, /title="Message Layout"/);
  assert.doesNotMatch(messages, /Other layouts are not available yet/);
  assert.doesNotMatch(appearance, /Element-like/);
  assert.doesNotMatch(appearance, /\bElement\b/);
  assert.equal(appearance.includes('Cinny'), false);
  assert.equal(appearance.includes('FluffyChat'), false);
  assert.doesNotMatch(appearance, /retired JS timeline/);
  assert.doesNotMatch(appearance, /<Text size="T300">Modern<\/Text>/);
});

test('Appearance is its own Settings page and General hosts no theme, layout or chat rows', () => {
  assert.match(settings, /SettingsPages\.AppearancePage/);
  assert.match(settings, /name: 'Appearance'/);
  assert.match(settings, /<AppearanceSettings requestClose=/);
  assert.equal(general.includes('useTheme'), false);
  assert.equal(general.includes('Message Layout'), false);
  assert.equal(general.includes('legacyUsernameColor'), false);
  assert.match(appearance, /Message layout/);
  assert.match(appearance, /Legacy username color/);
  // Thread display is a message display choice, next to spacing.
  const display = appearance.slice(appearance.indexOf('function MessageDisplay'));
  assert.match(display, /<SettingsSection title="Message display">/);
  assert.match(display, /<ThreadDisplaySetting \/>/);
  // General is the app itself: updates, date and time, keyboard shortcuts.
  assert.match(
    general,
    /<SettingsPage title="General" requestClose=\{requestClose\}>\s*<SoftwareUpdatesSection \/>\s*<DateAndTime \/>\s*<DesktopShortcutsSection \/>\s*<\/SettingsPage>/
  );
  ['gifSearchEnabled', 'hideActivity', 'mediaAutoLoad', 'enterForNewline', 'syncDrafts'].forEach(
    (key) => assert.equal(general.includes(key), false, key)
  );
});

const advanced = readFileSync(
  join(process.cwd(), 'src/app/features/settings/advanced/Advanced.tsx'),
  'utf8'
);
const media = readFileSync(
  join(process.cwd(), 'src/app/features/settings/media/MediaLinks.tsx'),
  'utf8'
);

test('maintenance actions live once, last on Advanced, not duplicated in About', () => {
  assert.match(advanced, /<SettingsSection title="Maintenance">/);
  assert.match(advanced, /Reload application/);
  assert.match(advanced, /<Maintenance \/>\s*<\/SettingsPage>/);
  // Reload keeps its critical styling.
  assert.match(advanced, /onClick=\{\(\) => reloadApplication\(\)\}\s*variant="Critical"/);
  // The session store is diagnostics content, so it lives on Diagnostics (once).
  const integration = readFileSync(
    join(process.cwd(), 'src/app/features/settings/diagnostics/DesktopIntegration.tsx'),
    'utf8'
  );
  assert.match(integration, /Native session store/);
  assert.equal(general.includes('SecretStoreTile'), false);
  assert.equal(about.includes('Clear Cache'), false);
  assert.equal(about.includes('UpdateSettingsTile'), false);
  assert.equal(about.includes('Options'), false);
  // Update checking has exactly one home: General > Updates, first on the page.
  assert.match(general, /<SettingsSection title="Updates">/);
  assert.match(general, /<UpdateSettingsTile \/>/);
});

test('Media & links uses positive phrasing for media loading and owns GIF search', () => {
  assert.match(media, /title="Load media automatically"/);
  assert.equal(media.includes('Disable Media Auto Load'), false);
  assert.match(media, /<SettingsSection title="GIFs">/);
  assert.match(media, /gifSearchEnabled/);
  // With no provider configured the row says so and the switch is disabled.
  assert.match(media, /GIF search is not configured by this client\./);
  assert.match(media, /disabled=\{!gifProviderAvailable\}/);
});

test('Advanced hosts Indexed message search with a session-reload note', () => {
  assert.match(advanced, /Indexed message search/);
  assert.match(advanced, /indexedMessageSearch/);
  assert.match(advanced, /session reload/);
});

test('native Notifications owns homeserver push rules instead of a unavailable stub', () => {
  assert.match(notifications, /<NativePushRulesEditor \/>/);
  assert.equal(notifications.includes('not available in this native session'), false);
  // The native editor is the only push-rule surface; the js-sdk editors are gone.
  assert.doesNotMatch(notifications, /AllMessagesNotifications|KeywordMessagesNotifications/);
  assert.equal(notifications.includes('Account > Block'), false);
});

const devices = readFileSync(
  join(process.cwd(), 'src/app/features/settings/devices/Devices.tsx'),
  'utf8'
);

test('Advanced › Experimental hosts the Widgets editor with its Beta badge', () => {
  assert.match(advanced, /<SettingsSection\s+title="Experimental"/);
  assert.match(advanced, /title="Experimental widgets"/);
  assert.match(advanced, />\s*Beta\s*</);
  assert.match(advanced, /experimentalWidgetsEnabled/);
  assert.match(advanced, /agentWidgetEntries/);
  assert.match(advanced, /widgetsSettingsDescription/);
  assert.match(advanced, /Add agent widget/);
  // Encrypted state events still push the setting to Core when the page opens.
  assert.match(advanced, /pushEncryptedStateEventsSetting\(encryptedStateEvents\)/);
});

test('Advanced hosts a Calls card with honest MatrixRTC status and the transport row', () => {
  assert.match(advanced, /<SettingsSection title="Calls">/);
  assert.match(advanced, /rtcCallAvailabilityCopy/);
  assert.match(advanced, /snapshotRtcTransportsNative/);
  assert.match(advanced, /refreshRtcTransportsNative/);
  assert.match(advanced, /title="Call transport"/);
  assert.equal(advanced.includes('set_call'), false);
  assert.equal(profile.includes('MatrixRTC'), false);
});

test('X.509 identity settings live on Security & recovery, not Advanced', () => {
  assert.match(devices, /X509IdentityCard/);
  assert.match(devices, /isNativeMatrixSession/);
  assert.equal(advanced.includes('X509'), false);
  assert.equal(advanced.includes('x509'), false);
  assert.equal(advanced.includes('X.509'), false);
  const x509Card = readFileSync(
    join(process.cwd(), 'src/app/features/settings/devices/X509IdentityCard.tsx'),
    'utf8'
  );
  assert.match(x509Card, /shares encrypted room keys/);
  assert.match(x509Card, /Reload session/);
  assert.equal(x509Card.includes('settingsAtom'), false);
});
