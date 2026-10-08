import { Page, test } from '@playwright/test';

// Full-height screenshots of every App, Room and Space settings page in both
// themes, for design review. Runs only when SETTINGS_SHOTS_DIR is set.
const SHOTS = process.env.SETTINGS_SHOTS_DIR;
const HARNESS = '/e2e/settings-harness/index.html';
const ROOM = '!review:example.test';

test.skip(!SHOTS, 'SETTINGS_SHOTS_DIR is not set');
test.use({ viewport: { width: 1280, height: 2000 } });
test.describe.configure({ timeout: 240000 });

const slug = (name: string) => name.toLowerCase().replace(/[^a-z0-9]+/g, '-');

const navNames = (page: Page) =>
  page
    .locator('[data-settings-nav]')
    .first()
    .locator('button, a')
    .evaluateAll((items) =>
      items.map((item) => item.textContent?.trim() ?? '').filter((text) => text.length > 0)
    );

const captureAll = async (page: Page, prefix: string, theme: string) => {
  const nav = page.locator('[data-settings-nav]').first();
  await nav.waitFor();
  for (const name of await navNames(page)) {
    await nav.getByRole('button', { name, exact: true }).click();
    await page.waitForTimeout(400);
    await page.screenshot({ path: `${SHOTS}/${prefix}-${slug(name)}-${theme}.png` });
  }
};

for (const theme of ['light', 'dark'] as const) {
  test(`app settings pages (${theme})`, async ({ page }) => {
    await page.goto(`${HARNESS}?theme=${theme}#/settings/`);
    await captureAll(page, 'app', theme);
  });

  test(`room settings pages (${theme})`, async ({ page }) => {
    await page.goto(`${HARNESS}?theme=${theme}#/home/${ROOM}/`);
    await page.getByRole('button', { name: 'More Options' }).click();
    await page.getByRole('button', { name: 'Room Settings' }).click();
    await captureAll(page, 'room', theme);
  });

  test(`space settings pages (${theme})`, async ({ page }) => {
    await page.goto(
      `${HARNESS}?theme=${theme}&space=1#/${encodeURIComponent('!studio:example.test')}/lobby/`
    );
    await page.getByRole('button', { name: 'More Options' }).click();
    await page.getByRole('button', { name: 'Space Settings' }).click();
    await captureAll(page, 'space', theme);
  });
}
