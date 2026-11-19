import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import {
  highlightTerms,
  matchRanges,
  matchSnippet,
  searchSettings,
  searchTerms,
  SettingsSearchEntry,
  SettingsSearchPage,
} from '../../../components/settings-layout/settingsSearch';
import { scanSettingTitles } from '../../../components/settings-layout/settingsTitleScan';
import { SettingsPages } from '../settingsPages';
import { APP_SETTINGS_SEARCH_INDEX, appSettingsSearchIndex } from '../settingsSearchIndex';
import { commonSettingsSearchIndex } from '../../common-settings/settingsSearchIndex';
import { APP_SETTINGS_PAGE_SOURCES, SHARED_PAGE_FILES } from './settingsPageSources';

const APP_PAGES: SettingsSearchPage<SettingsPages>[] = [
  { id: SettingsPages.GeneralPage, name: 'General' },
  { id: SettingsPages.AppearancePage, name: 'Appearance' },
  { id: SettingsPages.NotificationPage, name: 'Notifications' },
  { id: SettingsPages.MessagesPage, name: 'Messages' },
  { id: SettingsPages.MediaPage, name: 'Media & links' },
  { id: SettingsPages.PrivacyPage, name: 'Privacy' },
  { id: SettingsPages.AccountPage, name: 'Account' },
  { id: SettingsPages.SecurityPage, name: 'Security & recovery' },
  { id: SettingsPages.DevicesPage, name: 'Sessions' },
  { id: SettingsPages.AdvancedPage, name: 'Advanced' },
  { id: SettingsPages.DiagnosticsPage, name: 'Diagnostics' },
  { id: SettingsPages.AboutPage, name: 'About' },
];

const search = (query: string) => searchSettings(APP_PAGES, APP_SETTINGS_SEARCH_INDEX, query);
const titles = (query: string) =>
  search(query).flatMap((group) => group.results.map((result) => result.title));
const first = (query: string) => {
  const [group] = search(query);
  return { page: group?.pageName, title: group?.results[0]?.title };
};

test('"thread" finds Thread display on Appearance', () => {
  assert.deepEqual(first('thread'), { page: 'Appearance', title: 'Thread display' });
});

test('questions people ask find the right row first', () => {
  // The queries from the "where is GIF support?" report, and other phrasings.
  assert.deepEqual(first('enable gif'), { page: 'Media & links', title: 'GIF search' });
  assert.deepEqual(first('gif support'), { page: 'Media & links', title: 'GIF search' });
  assert.deepEqual(first('turn on gifs'), { page: 'Media & links', title: 'GIF search' });
  assert.deepEqual(first('gifs'), { page: 'Media & links', title: 'GIF search' });
  assert.deepEqual(first('read receipts off'), {
    page: 'Privacy',
    title: 'Hide typing & read receipts',
  });
  assert.deepEqual(first('dark mode'), { page: 'Appearance', title: 'Appearance' });
  assert.deepEqual(first('block someone'), { page: 'Privacy', title: 'Block a user' });
  assert.deepEqual(first('how do i change my password'), {
    page: 'Account',
    title: 'Password',
  });
  assert.deepEqual(first('link previews'), { page: 'Media & links', title: 'Link previews' });
  assert.deepEqual(first('developer tools'), {
    page: 'Advanced',
    title: 'Enable developer tools',
  });
  assert.deepEqual(first('widgets'), { page: 'Advanced', title: 'Experimental widgets' });
  assert.deepEqual(first('verify'), {
    page: 'Security & recovery',
    title: 'Verify this session',
  });
});

test('filler words are dropped while a meaningful word remains', () => {
  assert.deepEqual(searchTerms('enable gif'), ['gif']);
  assert.deepEqual(searchTerms('Turn ON gifs'), ['gifs']);
  assert.deepEqual(searchTerms('where is the setting for read receipts'), ['read', 'receipts']);
  // A query of only filler words is searched as typed.
  assert.deepEqual(searchTerms('settings'), ['settings']);
  assert.deepEqual(searchTerms('on off'), ['on', 'off']);
  assert.deepEqual(searchTerms('   '), []);
  // Highlighting covers the singular a plural matched.
  assert.deepEqual(highlightTerms('turn on gifs'), ['gifs', 'gif']);
});

test('a single word keeps exact substring matching', () => {
  const found = titles('gif');
  assert.ok(found.includes('GIF search'));
  assert.ok(found.includes('Play GIF links inline'));
  assert.ok(
    found.every((title) => {
      const entry = APP_SETTINGS_SEARCH_INDEX.find((item) => item.title === title)!;
      return [entry.title, entry.description ?? '', ...(entry.synonyms ?? [])]
        .join(' ')
        .toLowerCase()
        .includes('gif');
    })
  );
});

test('synonyms and descriptions find settings whose titles do not say so', () => {
  assert.ok(titles('dark').includes('Theme'));
  assert.ok(titles('dark').includes('Dark theme'));
  assert.ok(titles('side panel').includes('Thread display'));
  assert.ok(titles('read receipts').includes('Hide typing & read receipts'));
  assert.ok(titles('block').includes('Block a user'));
  assert.ok(titles('keychain').includes('Native session store'));
  assert.ok(titles('matrixrtc').includes('Call transport'));
  // Old names still find the renamed rows.
  assert.ok(titles('devices').includes('Other sessions'));
  assert.ok(titles('desktop shortcuts').includes('Keyboard shortcuts'));
  assert.ok(
    titles('storage').includes('Reload application') || titles('storage').includes('Maintenance')
  );
});

test('one or two words must all match; longer queries need most of them', () => {
  assert.deepEqual(titles('NOTIFICATION SOUND'), ['Notification sound']);
  assert.deepEqual(titles('thread zebra'), []);
  assert.deepEqual(search(''), []);
  assert.deepEqual(search('   '), []);
  // Three words, one unknown: still found.
  assert.equal(first('message spacing zebra').title, 'Message spacing');
});

test('all words outrank most words; title outranks synonyms and descriptions', () => {
  const ranked = search('theme').find((group) => group.pageName === 'Appearance')!.results;
  const titleMatches = ranked.filter((result) => result.title?.toLowerCase().includes('theme'));
  assert.deepEqual(ranked.slice(0, titleMatches.length), titleMatches);

  const ranking = searchSettings(
    [{ id: 'p', name: 'Page' }],
    [
      { page: 'p', title: 'Only description', description: 'alpha beta' },
      { page: 'p', title: 'Synonym', synonyms: ['alpha beta'] },
      { page: 'p', title: 'Alpha beta' },
      { page: 'p', title: 'Alpha', description: 'beta gamma delta' },
    ] as SettingsSearchEntry<string>[],
    'alpha beta'
  )[0].results.map((result) => result.title);
  assert.deepEqual(ranking, ['Alpha beta', 'Synonym', 'Alpha', 'Only description']);
});

test('the page with the best match comes first', () => {
  assert.equal(search('gif')[0].pageName, 'Media & links');
  assert.equal(search('reload')[0].pageName, 'Advanced');
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
  assert.deepEqual(matchRanges('Thread display', ['thread']), [[0, 6]]);
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

test('rows that render only in some builds are indexed with the same condition', () => {
  const when = (title: string) => APP_SETTINGS_SEARCH_INDEX.find((e) => e.title === title)?.when;
  // Never rendered in the desktop app (native sessions only there).
  assert.equal(when('Message layout'), 'nonNativeSession');
  assert.equal(when('Account data'), 'nonNativeSession');
  assert.equal(
    APP_SETTINGS_SEARCH_INDEX.some((entry) => entry.title === 'Global'),
    false
  );
  const desktopApp = appSettingsSearchIndex((condition) => condition !== 'nonNativeSession').map(
    (entry) => entry.title
  );
  assert.equal(desktopApp.includes('Message layout'), false);
  assert.equal(desktopApp.includes('Account data'), false);
  assert.ok(desktopApp.includes('GIF search'));
  assert.equal(when('Updates'), 'desktop');
  assert.equal(when('Experimental widgets'), 'synaraDesktop');
  assert.equal(when('Voice and video calls'), 'synaraDesktop');
  assert.equal(when('Tiling mode'), 'linuxShell');
  // The page code shows those rows under the same conditions.
  const read = (path: string) => readFileSync(join(process.cwd(), path), 'utf8');
  const appearance = read('src/app/features/settings/appearance/Appearance.tsx');
  assert.match(
    appearance,
    /!isNativeMatrixSession\(\) && \(\s*<SequenceCard[\s\S]{0,200}Message layout/
  );
  const advanced = read('src/app/features/settings/advanced/Advanced.tsx');
  assert.match(advanced, /developerTools && !isNativeMatrixSession\(\) && \(\s*<AccountData/);
});

const scan = (file: string, infoCards = false) =>
  scanSettingTitles(
    readFileSync(join(process.cwd(), file), 'utf8'),
    infoCards ? ['SettingTile', 'SettingsSection', 'InfoCard'] : undefined
  );

test('the App Settings index lists every title its pages render, components included', () => {
  (
    Object.entries(APP_SETTINGS_PAGE_SOURCES) as Array<
      [SettingsPages, { file: string; infoCards?: boolean }[]]
    >
  ).forEach(([page, sources]) => {
    sources.forEach(({ file, infoCards }) => {
      const pages = SHARED_PAGE_FILES[file] ?? [page];
      const indexed = new Set(
        APP_SETTINGS_SEARCH_INDEX.filter((entry) => pages.includes(entry.page)).map(
          (entry) => entry.title
        )
      );
      scan(file, infoCards).forEach(({ candidates, component }) => {
        // InfoCards are only rows where the file is listed for them.
        if (component === 'InfoCard' && !infoCards) return;
        assert.ok(
          candidates.some((candidate) => indexed.has(candidate)),
          `${file} renders "${candidates.join('" / "')}" but the settings search index does not list it for that page`
        );
      });
    });
  });
  // The rows rendered from components/ are covered.
  const security = APP_SETTINGS_SEARCH_INDEX.filter((e) => e.page === SettingsPages.SecurityPage);
  assert.ok(security.some((entry) => entry.title === 'Secure Backup & Recovery'));
  assert.ok(security.some((entry) => entry.title === 'Encryption Backup'));
});

test('index descriptions match the descriptions the pages render', () => {
  const sources = Object.values(APP_SETTINGS_PAGE_SOURCES).flat();
  const seen = new Set<string>();
  const differences: string[] = [];
  sources.forEach(({ file, infoCards }) => {
    if (seen.has(file)) return;
    seen.add(file);
    scan(file, infoCards).forEach(({ candidates, descriptions }) => {
      if (descriptions.length === 0) return;
      APP_SETTINGS_SEARCH_INDEX.filter((entry) => candidates.includes(entry.title)).forEach(
        (entry) => {
          // A conditional description may match either variant.
          if (entry.description === undefined || !descriptions.includes(entry.description)) {
            differences.push(
              `"${entry.title}" is described in the index as "${entry.description}" but ${file} renders "${descriptions.join('" / "')}"`
            );
          }
        }
      );
    });
  });
  assert.deepEqual(differences, []);
});

const walk = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === '__tests__' ? [] : walk(path);
    return path.endsWith('.tsx') ? [path] : [];
  });

test('every App Settings page file is listed as a page source', () => {
  const listed = new Set(
    Object.values(APP_SETTINGS_PAGE_SOURCES)
      .flat()
      .map((source) => source.file)
  );
  const shells = new Set([
    'src/app/features/settings/Settings.tsx',
    'src/app/features/settings/AppSettingsRenderer.tsx',
  ]);
  walk('src/app/features/settings').forEach((file) => {
    if (shells.has(file)) return;
    assert.ok(listed.has(file), `${file} is not listed in settingsPageSources.ts`);
  });
});

const assertIndexed = <T>(dir: string, page: T | T[], index: SettingsSearchEntry<T>[]) => {
  const pages = Array.isArray(page) ? page : [page];
  const indexed = new Set(
    index.filter((entry) => pages.includes(entry.page)).map((entry) => entry.title)
  );
  walk(dir).forEach((file) => {
    scanSettingTitles(readFileSync(file, 'utf8')).forEach(({ candidates }) => {
      assert.ok(
        candidates.some((candidate) => indexed.has(candidate)),
        `${file} renders "${candidates.join('" / "')}" but the settings search index does not list it for that page`
      );
    });
  });
};

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
