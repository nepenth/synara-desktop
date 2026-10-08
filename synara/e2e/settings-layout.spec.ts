import { expect, Page, test } from '@playwright/test';

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
      for (const name of ['General', 'Appearance', 'Notifications', 'Account', 'About']) {
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
        await shot(page, `settings-${name.toLowerCase()}-${colorScheme}`);
      }
    });

    test('room header menu and room settings', async ({ page }) => {
      await page.goto(`${HARNESS}#/home/${ROOM}/`);
      await page.getByRole('button', { name: 'More Options' }).click();
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
