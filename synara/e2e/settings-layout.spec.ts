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

/** Vertical gaps (px) between consecutive nav menu items in a settings nav. */
const navGaps = (page: Page) =>
  page.evaluate(() => {
    const items = Array.from(
      document.querySelectorAll<HTMLElement>('[data-settings-nav] button, [data-settings-nav] a')
    ).map((el) => el.getBoundingClientRect());
    return items.slice(1).map((rect, index) => Math.round(rect.top - items[index].bottom));
  });

/** Distinct right edges (px) of setting-row controls in the open settings page. */
const controlRightEdges = (page: Page) =>
  page.evaluate(() => {
    const edges = Array.from(document.querySelectorAll<HTMLElement>('[data-setting-control]')).map(
      (el) => Math.round(el.getBoundingClientRect().right)
    );
    return Array.from(new Set(edges)).sort((a, b) => a - b);
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
        log(`settings-${name}-control-right-edges ${colorScheme}`, await controlRightEdges(page));
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
      log(`room-settings-nav-gaps ${colorScheme}`, await navGaps(page));
      for (const name of ['General', 'Permissions']) {
        await nav.getByRole('button', { name, exact: true }).click();
        await page.waitForTimeout(300);
        log(
          `room-settings-${name}-control-right-edges ${colorScheme}`,
          await controlRightEdges(page)
        );
        await shot(page, `room-settings-${name.toLowerCase()}-${colorScheme}`);
      }
    });
  });
}
