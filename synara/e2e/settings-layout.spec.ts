import { expect, Page, test } from '@playwright/test';
import { SETTINGS_INVENTORY } from '../src/app/features/settings/__tests__/settingsInventory';

// Layout proof for Settings, Room Settings and the room header menu, mounted
// from the production app shell (e2e/settings-harness). Screenshots are
// written only when SETTINGS_SHOTS_DIR is set.
const SHOTS = process.env.SETTINGS_SHOTS_DIR;
// Record measurements without asserting (used to capture a baseline).
const MEASURE_ONLY = Boolean(process.env.SETTINGS_MEASURE_ONLY);
const HARNESS = '/e2e/settings-harness/index.html';
const ROOM = '!review:example.test';

const shot = async (page: Page, name: string) => {
  if (SHOTS) await page.screenshot({ path: `${SHOTS}/${name}.png` });
};

/**
 * Vertical gaps (px) between consecutive nav entries inside one nav group.
 * Groups are separated on purpose by their labels; entries within a group
 * must stay compact.
 */
const navGaps = (page: Page) =>
  page.evaluate(() => {
    const groups = Array.from(
      document.querySelectorAll<HTMLElement>('[data-settings-nav] [role="group"]')
    );
    return groups.flatMap((group) => {
      const items = Array.from(
        group.querySelectorAll<HTMLElement>('button[data-settings-nav-item]')
      ).map((el) => (el.parentElement ?? el).getBoundingClientRect());
      return items.slice(1).map((rect, index) => Math.round(rect.top - items[index].bottom));
    });
  });

/** Distinct right edges (px) of setting-row controls in the open settings page. */
const controlRightEdges = (page: Page) =>
  page.evaluate(() => {
    const edges = Array.from(
      document.querySelectorAll<HTMLElement>(
        '[data-settings-section-card] > [data-sequence-card] [data-setting-control]'
      )
    ).map((el) => Math.round(el.getBoundingClientRect().right));
    return Array.from(new Set(edges)).sort((a, b) => a - b);
  });

/**
 * Page column widths, sections without exactly one card, and section cards per
 * section for the open settings page.
 */
const pageStructure = (page: Page) =>
  page.evaluate(() => {
    const column = document.querySelector<HTMLElement>('[data-settings-page]');
    const sections = Array.from(document.querySelectorAll<HTMLElement>('[data-settings-section]'));
    return {
      maxWidth: column ? getComputedStyle(column).maxWidth : 'missing',
      sections: sections.length,
      cardsPerSection: sections.map(
        (section) =>
          Array.from(section.children).filter((child) =>
            child.hasAttribute('data-settings-section-card')
          ).length
      ),
    };
  });

const log = (label: string, value: unknown) => console.log(`${label} ${JSON.stringify(value)}`);

for (const colorScheme of ['light', 'dark'] as const) {
  test.describe(`${colorScheme} theme`, () => {
    test.use({ colorScheme });

    test('app settings navigation and pages', async ({ page }) => {
      await page.goto(`${HARNESS}#/settings/`);
      const nav = page.locator('[data-settings-nav]').first();
      await expect(nav).toBeVisible();
      const gaps = await navGaps(page);
      log(`settings-nav-gaps ${colorScheme}`, gaps);
      if (!MEASURE_ONLY) expect(Math.max(...gaps)).toBeLessThanOrEqual(8);
      for (const name of [
        'General',
        'Appearance',
        'Notifications',
        'Messages',
        'Media & links',
        'Privacy',
        'Account',
        'Advanced',
        'About',
      ]) {
        await nav.getByRole('button', { name, exact: true }).click();
        await page.waitForTimeout(300);
        const edges = await controlRightEdges(page);
        const structure = await pageStructure(page);
        log(`settings-${name}-control-right-edges ${colorScheme}`, edges);
        log(`settings-${name}-structure ${colorScheme}`, structure);
        if (!MEASURE_ONLY) {
          expect(structure.maxWidth).toBe('720px');
          expect(structure.sections).toBeGreaterThan(0);
          expect(structure.cardsPerSection.every((count) => count === 1)).toBe(true);
          if (edges.length > 0) expect(edges[edges.length - 1] - edges[0]).toBeLessThanOrEqual(1);
        }
        await shot(
          page,
          `settings-${name.toLowerCase().replace(/[^a-z0-9]+/g, '-')}-${colorScheme}`
        );
      }
    });

    test('room header menu and room settings', async ({ page }) => {
      await page.goto(`${HARNESS}#/home/${ROOM}/`);
      await page
        .locator('header')
        .getByRole('button', { name: /more options/i })
        .click();
      await expect(page.getByText('Copy Link')).toBeVisible();
      const backgrounds = await page.evaluate(() =>
        ['Invite', 'Copy Link', 'Room Settings', 'Jump to Time'].map((label) => {
          const item = Array.from(document.querySelectorAll('button')).find(
            (button) => button.textContent?.trim() === label
          );
          return [label, item ? getComputedStyle(item).backgroundColor : 'missing'];
        })
      );
      log(`room-menu-backgrounds ${colorScheme}`, backgrounds);
      if (!MEASURE_ONLY) expect(new Set(backgrounds.map(([, color]) => color)).size).toBe(1);
      await shot(page, `room-menu-${colorScheme}`);

      await page.getByRole('button', { name: 'Room Settings' }).click();
      const nav = page.locator('[data-settings-nav]').first();
      await expect(nav).toBeVisible();
      const roomGaps = await navGaps(page);
      log(`room-settings-nav-gaps ${colorScheme}`, roomGaps);
      if (!MEASURE_ONLY) expect(Math.max(...roomGaps)).toBeLessThanOrEqual(8);
      for (const name of ['General', 'Members', 'Permissions']) {
        await nav.getByRole('button', { name, exact: true }).click();
        await page.waitForTimeout(300);
        const edges = await controlRightEdges(page);
        const structure = await pageStructure(page);
        log(`room-settings-${name}-control-right-edges ${colorScheme}`, edges);
        log(`room-settings-${name}-structure ${colorScheme}`, structure);
        if (!MEASURE_ONLY) {
          expect(structure.maxWidth).toBe('720px');
          expect(structure.cardsPerSection.every((count) => count === 1)).toBe(true);
          if (edges.length > 0) expect(edges[edges.length - 1] - edges[0]).toBeLessThanOrEqual(1);
        }
        await shot(page, `room-settings-${name.toLowerCase()}-${colorScheme}`);
      }
    });
  });
}

test('settings search finds individual settings and opens them', async ({ page }) => {
  await page.goto(`${HARNESS}#/settings/`);
  const search = page.getByRole('textbox', { name: 'Search settings' });
  await expect(search).toBeVisible();

  await search.fill('thread');
  const results = page.locator('[data-settings-search-results]');
  await expect(results.locator('[data-settings-search-result="Thread display"]')).toBeVisible();
  await expect(results.getByRole('group', { name: 'Appearance' })).toBeVisible();

  // Enter opens the first result: Appearance, scrolled to the Thread display row.
  await search.press('Enter');
  await expect(page.locator('[data-setting-title="Thread display"]').first()).toBeInViewport();
  await shot(page, 'settings-search-thread');

  // Arrow keys reach the results.
  await search.fill('notification sound');
  await search.press('ArrowDown');
  const sound = results.locator('[data-settings-search-result="Notification sound"]');
  await expect(sound).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('[data-setting-title="Notification sound"]').first()).toBeInViewport();

  // No match keeps the empty state; Escape clears the search and restores the nav.
  await search.fill('zzzz qqqq');
  await expect(page.getByText('No settings match “zzzz qqqq”.')).toBeVisible();
  await search.press('Escape');
  await expect(search).toHaveValue('');
  await expect(page.locator('nav[aria-label="Settings sections"]')).toBeVisible();
});

test('settings search answers the questions people ask', async ({ page }) => {
  await page.goto(`${HARNESS}#/settings/`);
  const search = page.getByRole('textbox', { name: 'Search settings' });
  const results = page.locator('[data-settings-search-results]');
  const firstResult = results.locator('[data-settings-search-result]').first();
  const cases: Array<[string, string, string]> = [
    ['enable gif', 'Media & links', 'GIF search'],
    ['gif support', 'Media & links', 'GIF search'],
    ['turn on gifs', 'Media & links', 'GIF search'],
    ['read receipts off', 'Privacy', 'Hide typing & read receipts'],
    ['dark mode', 'Appearance', 'Appearance'],
    ['block someone', 'Privacy', 'Block a user'],
  ];
  for (const [query, pageName, title] of cases) {
    await search.fill(query);
    await expect(firstResult).toHaveAttribute('data-settings-search-result', title);
    await expect(results.getByRole('group').first()).toHaveAttribute('aria-label', pageName);
    if (query === 'enable gif') await shot(page, 'settings-search-enable-gif');
  }
  await search.fill('enable gif');
  await search.press('Enter');
  await expect(page.getByRole('heading', { level: 2 })).toHaveText('Media & links');
  await expect(page.locator('[data-setting-title="GIF search"]').first()).toBeInViewport();
});

test('every setting renders on its pinned page and section', async ({ page }) => {
  await page.goto(`${HARNESS}#/settings/`);
  const nav = page.getByRole('navigation', { name: 'Settings sections' });
  await expect(nav).toBeVisible();
  await shot(page, 'settings-nav');
  const navNames = await nav
    .locator('button[data-settings-nav-item]')
    .evaluateAll((items) => items.map((item) => item.textContent?.trim() ?? ''));
  // The harness is a Linux desktop app, so Diagnostics shows.
  expect(navNames).toEqual(SETTINGS_INVENTORY.map((entry) => entry.page));
  const groups = await nav
    .getByRole('group')
    .evaluateAll((items) => items.map((item) => item.getAttribute('aria-label')));
  expect(groups).toEqual(['App', 'Chat', 'Account', 'Advanced']);

  for (const inventoryPage of SETTINGS_INVENTORY) {
    await nav.getByRole('button', { name: inventoryPage.page, exact: true }).click();
    await expect(page.getByRole('heading', { level: 2 })).toHaveText(inventoryPage.page);
    await page.waitForTimeout(300);
    // Every titled row and section on the page, with its enclosing section.
    const rendered = await page.locator('[data-settings-page]').evaluate((column) =>
      Array.from(column.querySelectorAll<HTMLElement>('[data-setting-title]')).map((element) => {
        const section = element.hasAttribute('data-settings-section')
          ? element
          : element.closest<HTMLElement>('[data-settings-section]');
        return `${section?.dataset.settingTitle ?? '?'} › ${element.dataset.settingTitle}`;
      })
    );
    const pinned = new Set<string>();
    const required: string[] = [];
    inventoryPage.sections.forEach((section) => {
      pinned.add(`${section.title} › ${section.title}`);
      if (!section.when) required.push(`${section.title} › ${section.title}`);
      section.rows.forEach((row) => {
        pinned.add(`${section.title} › ${row.title}`);
        if (!section.when && !row.when && !row.labelTitle) {
          required.push(`${section.title} › ${row.title}`);
        }
      });
    });
    // Nothing renders that the inventory does not pin, where it pins it.
    // The Matrix ID row is titled with the user ID itself.
    const unpinned = rendered.filter((entry) => !pinned.has(entry) && !/ › @/.test(entry));
    expect(unpinned, `${inventoryPage.page}: rendered but not pinned`).toEqual([]);
    // Every unconditional pinned row renders. Conditional rows are checked by
    // the unit tests against the code that decides them.
    const missing = required.filter((entry) => !rendered.includes(entry));
    expect(missing, `${inventoryPage.page}: pinned but not rendered`).toEqual([]);
  }
});

test('room settings search finds room rows', async ({ page }) => {
  await page.goto(`${HARNESS}#/home/${ROOM}/`);
  await page
    .locator('header')
    .getByRole('button', { name: /more options/i })
    .click();
  await page.getByRole('button', { name: 'Room Settings' }).click();
  const search = page.getByRole('textbox', { name: 'Search settings' });
  await search.fill('history');
  await expect(
    page.locator('[data-settings-search-result="Message History Visibility"]')
  ).toBeVisible();
});
