import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import {
  GIF_SEARCH_SETTING,
  lastViewedSettingsPage,
  rememberSettingsPage,
  settingsPageFromSlug,
  SettingsPages,
  verificationSettingsLocation,
} from '../settingsPages';
import { getSettingsPath } from '../../../pages/pathUtils';
import { parseSynaraRouteDestination } from '../../../routes/synaraRoutes';
import {
  APP_SETTINGS_REOPEN_GUARD_MS,
  appSettingsJustClosed,
  noteAppSettingsClosed,
} from '../../../state/appSettings';

test('every page has a route slug, and unknown slugs open nothing', () => {
  assert.equal(settingsPageFromSlug('media-links'), SettingsPages.MediaPage);
  assert.equal(settingsPageFromSlug('security'), SettingsPages.SecurityPage);
  assert.equal(settingsPageFromSlug('developer-tools'), undefined);
  assert.equal(settingsPageFromSlug(undefined), undefined);
  assert.equal(new Set(Object.values(SettingsPages)).size, Object.values(SettingsPages).length);
});

test('settings paths keep /settings/ and name a page and row', () => {
  assert.equal(getSettingsPath(), '/settings/');
  assert.equal(getSettingsPath('privacy'), '/settings/privacy/');
  assert.equal(
    getSettingsPath(GIF_SEARCH_SETTING.page, GIF_SEARCH_SETTING.setting),
    '/settings/media-links/?setting=GIF+search'
  );
  assert.deepEqual(parseSynaraRouteDestination('/settings/'), { kind: 'settings' });
  assert.deepEqual(parseSynaraRouteDestination('/settings/advanced/'), {
    kind: 'settings',
    section: 'advanced',
  });
});

test('the last page viewed is remembered for this run only', () => {
  assert.equal(lastViewedSettingsPage(), undefined);
  rememberSettingsPage(SettingsPages.PrivacyPage);
  rememberSettingsPage(undefined);
  assert.equal(lastViewedSettingsPage(), SettingsPages.PrivacyPage);
});

test('verifying this session leads to Security & recovery; other sessions to Sessions', () => {
  assert.deepEqual(verificationSettingsLocation('this-device'), {
    page: SettingsPages.SecurityPage,
    setting: 'Verify this session',
  });
  assert.deepEqual(verificationSettingsLocation('other-devices'), {
    page: SettingsPages.DevicesPage,
    setting: 'Other sessions',
  });
  const devices = readFileSync(
    join(process.cwd(), 'src/app/features/settings/devices/Devices.tsx'),
    'utf8'
  );
  assert.match(devices, /title="Verify this session"/);
  const others = readFileSync(
    join(process.cwd(), 'src/app/features/settings/devices/OtherDevices.tsx'),
    'utf8'
  );
  assert.match(others, /title="Other sessions"/);
});

test('a close from outside the dialog does not reopen Settings from the same press', () => {
  noteAppSettingsClosed(1000);
  assert.equal(appSettingsJustClosed(1000 + 50), true);
  assert.equal(appSettingsJustClosed(1000 + APP_SETTINGS_REOPEN_GUARD_MS + 1), false);
});

test('the GIF search location names the rendered row', () => {
  const media = readFileSync(
    join(process.cwd(), 'src/app/features/settings/media/MediaLinks.tsx'),
    'utf8'
  );
  assert.match(media, new RegExp(`'${GIF_SEARCH_SETTING.setting}'`));
  assert.equal(GIF_SEARCH_SETTING.page, SettingsPages.MediaPage);
});
