import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { SettingsPages } from '../settingsPages';
import { APP_SETTINGS_SEARCH_INDEX } from '../settingsSearchIndex';
import { SETTINGS_INVENTORY } from './settingsInventory';
import { APP_SETTINGS_PAGE_SOURCES } from './settingsPageSources';

const PAGE_IDS: Record<string, SettingsPages> = {
  General: SettingsPages.GeneralPage,
  Appearance: SettingsPages.AppearancePage,
  Notifications: SettingsPages.NotificationPage,
  Messages: SettingsPages.MessagesPage,
  'Media & links': SettingsPages.MediaPage,
  Privacy: SettingsPages.PrivacyPage,
  Account: SettingsPages.AccountPage,
  'Security & recovery': SettingsPages.SecurityPage,
  Sessions: SettingsPages.DevicesPage,
  Advanced: SettingsPages.AdvancedPage,
  Diagnostics: SettingsPages.DiagnosticsPage,
  About: SettingsPages.AboutPage,
};

const read = (file: string) => readFileSync(join(process.cwd(), file), 'utf8');
const pageSource = (page: SettingsPages) =>
  APP_SETTINGS_PAGE_SOURCES[page].map(({ file }) => read(file)).join('\n');

const key = (page: SettingsPages, section: string, title: string) =>
  `${page} › ${section} › ${title}`;

test('the inventory names every page in nav order, in four groups', () => {
  assert.deepEqual(
    SETTINGS_INVENTORY.map((page) => `${page.group}: ${page.page}`),
    [
      'App: General',
      'App: Appearance',
      'App: Notifications',
      'Chat: Messages',
      'Chat: Media & links',
      'Chat: Privacy',
      'Account: Account',
      'Account: Security & recovery',
      'Account: Sessions',
      'Advanced: Advanced',
      'Advanced: Diagnostics',
      'Advanced: About',
    ]
  );
  const settings = read('src/app/features/settings/Settings.tsx');
  const navOrder = Array.from(settings.matchAll(/name: '([^']+)'/g)).map((match) => match[1]);
  assert.deepEqual(navOrder, Object.keys(PAGE_IDS));
  const groups = Array.from(settings.matchAll(/label: '([^']+)'/g)).map((match) => match[1]);
  assert.deepEqual(groups, ['App', 'Chat', 'Account', 'Advanced']);
  assert.doesNotMatch(settings, /Developer tools/);
});

test('every indexed setting is in the inventory on the same page and section, and back', () => {
  const pinned = new Set<string>();
  SETTINGS_INVENTORY.forEach(({ page, sections }) => {
    sections.forEach((section) => {
      pinned.add(key(PAGE_IDS[page], section.title, section.title));
      section.rows
        .filter((row) => row.indexed !== false)
        .forEach((row) => pinned.add(key(PAGE_IDS[page], section.title, row.title)));
    });
  });
  const indexed = new Set(
    APP_SETTINGS_SEARCH_INDEX.map((entry) => key(entry.page, entry.section, entry.title))
  );
  const missing = Array.from(pinned).filter((entry) => !indexed.has(entry));
  const unpinned = Array.from(indexed).filter((entry) => !pinned.has(entry));
  assert.deepEqual(missing, [], 'pinned settings missing from the search index');
  assert.deepEqual(unpinned, [], 'indexed settings missing from the inventory');
  assert.equal(indexed.size, APP_SETTINGS_SEARCH_INDEX.length, 'duplicate index entries');
});

test('every setting still reads and writes the same stored key, on its new page', () => {
  SETTINGS_INVENTORY.forEach(({ page, sections }) => {
    const source = pageSource(PAGE_IDS[page]);
    sections.forEach((section) => {
      section.rows.forEach((row) => {
        [row.stores ?? []].flat().forEach((store) => {
          const match = /^(settings|platformSettings)\.(\w+)$/.exec(store);
          if (!match) return;
          const atom = match[1] === 'settings' ? 'settingsAtom' : 'desktopPlatformSettingsAtom';
          assert.match(
            source,
            new RegExp(`useSetting\\(\\s*${atom},\\s*'${match[2]}'\\s*\\)`),
            `${page} › ${section.title} › ${row.title} no longer uses ${store}`
          );
        });
      });
    });
  });
});

test('every row title and section title is rendered by its page', () => {
  SETTINGS_INVENTORY.forEach(({ page, sections }) => {
    const source = pageSource(PAGE_IDS[page]);
    sections.forEach((section) => {
      assert.ok(
        source.includes(`'${section.title}'`) || source.includes(`"${section.title}"`),
        `${page} does not render section "${section.title}"`
      );
      section.rows.forEach((row) => {
        // Computed titles are checked by their literal copy.
        const literal = row.title === 'Reset identity' ? 'IDENTITY_RESET_COPY.title' : row.title;
        assert.ok(
          source.includes(literal),
          `${page} › ${section.title} does not render "${row.title}"`
        );
      });
    });
  });
});

test('a stored setting has exactly one home', () => {
  const homes = new Map<string, string[]>();
  SETTINGS_INVENTORY.forEach(({ page, sections }) =>
    sections.forEach((section) =>
      section.rows.forEach((row) =>
        [row.stores ?? []].flat().forEach((store) => {
          if (!/^(settings|platformSettings)\./.test(store)) return;
          homes.set(store, [...(homes.get(store) ?? []), `${page} › ${section.title}`]);
        })
      )
    )
  );
  // themeId is set by both the Appearance switch and the Theme picker, on one section.
  const shared = Array.from(homes.entries()).filter(
    ([store, places]) => new Set(places).size > 1 && store !== 'settings.themeId'
  );
  assert.deepEqual(shared, []);
});

test('the regroup moved these settings, and only these', () => {
  const moved = SETTINGS_INVENTORY.flatMap(({ page, sections }) =>
    sections.flatMap((section) =>
      section.rows
        .map((row) => ({ row, was: row.was ?? section.was }))
        .filter(({ was }) => was !== `${page} › ${section.title}`)
        .map(({ row, was }) => `${row.title}: ${was} → ${page} › ${section.title}`)
    )
  );
  assert.deepEqual(moved, [
    'Message layout: Appearance › Messages → Appearance › Message display',
    'Message spacing: Appearance › Messages → Appearance › Message display',
    'Thread display: Appearance › Text → Appearance › Message display',
    'Legacy username color: Appearance › Messages → Appearance › Message display',
    'Show approval details: Notifications › System → Notifications › Agent notifications',
    'Enter adds a new line: General › Composer → Messages › Composer',
    'Markdown formatting: General › Composer → Messages › Composer',
    'Hide membership changes: General › Messages → Messages › Timeline',
    'Hide profile changes: General › Messages → Messages › Timeline',
    'Sync drafts across devices: General › Messages → Messages › Drafts',
    'Load media automatically: General › Messages → Media & links › Media',
    'GIF search: General › Messages → Media & links › GIFs',
    'Play GIF links inline: General › Messages → Media & links › GIFs',
    'Link previews: General › Messages → Media & links › Links',
    'Show X posts: General › Messages → Media & links › Links',
    'Shorten long links: General › Messages → Media & links › Links',
    'Hide typing & read receipts: General › Composer → Privacy › Activity',
    'Block a user: Account › Blocked users → Privacy › Blocked users',
    'Indexed message search: General › Messages → Advanced › Experimental',
    'Show hidden events: General › Messages → Advanced › Experimental',
    'Experimental widgets: General › Widgets → Advanced › Experimental',
    'Encrypted state events (experimental): Developer tools › Options → Advanced › Experimental',
    'Voice and video calls: General › Calls → Advanced › Calls',
    'Call transport: Account › Profile (as "MatrixRTC") → Advanced › Calls',
    'Enable developer tools: Developer tools › Options → Advanced › Developer',
    'Account data: Developer tools › Account data (row "Global") → Advanced › Developer',
    'Reload application: General › Storage → Advanced › Maintenance',
  ]);
});
