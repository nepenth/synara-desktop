import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import {
  matchRanges,
  matchSnippet,
  searchSettings,
  SettingsSearchEntry,
  SettingsSearchPage,
} from '../../../components/settings-layout/settingsSearch';
import { scanSettingTitles } from '../../../components/settings-layout/settingsTitleScan';
import { SettingsPages } from '../settingsPages';
import { APP_SETTINGS_SEARCH_INDEX } from '../settingsSearchIndex';
import { commonSettingsSearchIndex } from '../../common-settings/settingsSearchIndex';

const APP_PAGES: SettingsSearchPage<SettingsPages>[] = [
  { id: SettingsPages.GeneralPage, name: 'General' },
  { id: SettingsPages.AppearancePage, name: 'Appearance' },
  { id: SettingsPages.NotificationPage, name: 'Notifications' },
  { id: SettingsPages.AccountPage, name: 'Account' },
  { id: SettingsPages.DevicesPage, name: 'Devices' },
  { id: SettingsPages.DiagnosticsPage, name: 'Diagnostics' },
  { id: SettingsPages.DeveloperToolsPage, name: 'Developer Tools' },
  { id: SettingsPages.AboutPage, name: 'About' },
];

const search = (query: string) => searchSettings(APP_PAGES, APP_SETTINGS_SEARCH_INDEX, query);
const titles = (query: string) =>
  search(query).flatMap((group) => group.results.map((result) => result.title));

test('"thread" finds Thread Display on Appearance', () => {
  const groups = search('thread');
  assert.equal(groups[0]?.pageName, 'Appearance');
  assert.equal(groups[0]?.results[0]?.title, 'Thread Display');
});

test('synonyms and descriptions find settings whose titles do not say so', () => {
  assert.ok(titles('dark').includes('Theme'));
  assert.ok(titles('dark').includes('Dark Theme'));
  assert.ok(titles('side panel').includes('Thread Display'));
  assert.ok(titles('read receipts').includes('Hide Typing & Read Receipts'));
  assert.ok(titles('block').includes('Select User'));
  assert.ok(titles('keychain').includes('Native Session Store'));
});

test('every word must match, case-insensitively', () => {
  assert.deepEqual(titles('NOTIFICATION SOUND'), ['Notification Sound']);
  assert.deepEqual(titles('thread zebra'), []);
  assert.deepEqual(search(''), []);
  assert.deepEqual(search('   '), []);
});

test('title matches rank above synonym and description matches', () => {
  const ranked = search('theme').find((group) => group.pageName === 'Appearance')!.results;
  const titleMatches = ranked.filter((result) => result.title?.toLowerCase().includes('theme'));
  assert.deepEqual(ranked.slice(0, titleMatches.length), titleMatches);
});

test('a page name lists its settings, and a page with none still appears', () => {
  const about = search('about');
  assert.equal(about.length, 1);
  assert.equal(about[0].pageName, 'About');
  assert.equal(about[0].results[0].title, 'Credits');

  const bare = searchSettings(
    [{ id: 'members', name: 'Members', keywords: ['people'] }],
    [] as SettingsSearchEntry<string>[],
    'people'
  );
  assert.equal(bare.length, 1);
  assert.equal(bare[0].results[0].title, undefined);
});

test('matches are highlighted and long descriptions are cut around the match', () => {
  assert.deepEqual(matchRanges('Thread Display', ['thread']), [[0, 6]]);
  assert.deepEqual(matchRanges('Side side', ['side']), [
    [0, 4],
    [5, 9],
  ]);
  const long = `${'a'.repeat(120)} inline ${'b'.repeat(120)}`;
  const snippet = matchSnippet(long, ['inline']);
  assert.ok(snippet.includes('inline'));
  assert.ok(snippet.startsWith('…') && snippet.endsWith('…'));
  assert.equal(matchSnippet('Short text', ['short']), 'Short text');
});

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === '__tests__' ? [] : walk(path);
    return path.endsWith('.tsx') ? [path] : [];
  });

const assertIndexed = <T>(dir: string, page: T, index: SettingsSearchEntry<T>[]) => {
  const indexed = new Set(index.filter((entry) => entry.page === page).map((entry) => entry.title));
  walk(dir).forEach((file) => {
    scanSettingTitles(readFileSync(file, 'utf8')).forEach(({ candidates }) => {
      assert.ok(
        candidates.some((candidate) => indexed.has(candidate)),
        `${file} renders "${candidates.join('" / "')}" but the settings search index does not list it for that page`
      );
    });
  });
};

test('the App Settings index lists every title its pages render', () => {
  const pages: Array<[string, SettingsPages]> = [
    ['general', SettingsPages.GeneralPage],
    ['appearance', SettingsPages.AppearancePage],
    ['notifications', SettingsPages.NotificationPage],
    ['account', SettingsPages.AccountPage],
    ['devices', SettingsPages.DevicesPage],
    ['diagnostics', SettingsPages.DiagnosticsPage],
    ['developer-tools', SettingsPages.DeveloperToolsPage],
    ['about', SettingsPages.AboutPage],
  ];
  pages.forEach(([dir, page]) =>
    assertIndexed(`src/app/features/settings/${dir}`, page, APP_SETTINGS_SEARCH_INDEX)
  );
});

test('the Room and Space Settings indexes list every title their pages render', () => {
  const pages = { general: 0, permissions: 2, emojis: 3, developer: 4 };
  (['room', 'space'] as const).forEach((kind) => {
    const index = commonSettingsSearchIndex(kind, pages);
    assertIndexed(`src/app/features/${kind}-settings/general`, pages.general, index);
    assertIndexed('src/app/features/common-settings/general', pages.general, index);
    assertIndexed('src/app/features/common-settings/permissions', pages.permissions, index);
    assertIndexed('src/app/features/common-settings/emojis-stickers', pages.emojis, index);
    assertIndexed('src/app/features/common-settings/developer-tools', pages.developer, index);
  });
});

test('Room and Space indexes name their own access and upgrade rows', () => {
  const pages = { general: 0, permissions: 2, emojis: 3, developer: 4 };
  const room = commonSettingsSearchIndex('room', pages).map((entry) => entry.title);
  const space = commonSettingsSearchIndex('space', pages).map((entry) => entry.title);
  assert.ok(room.includes('Room Access') && room.includes('Upgrade Room'));
  assert.ok(space.includes('Space Access') && space.includes('Upgrade Space'));
});
