import { expect, Page, test } from '@playwright/test';

// How Settings opens, from the production app shell (e2e/settings-harness):
// the avatar in the bottom bar toggles it, it remembers the last page, links
// open a page and row, and the composer leads to GIF search. Screenshots are
// written only when SETTINGS_SHOTS_DIR is set.
const SHOTS = process.env.SETTINGS_SHOTS_DIR;
const HARNESS = '/e2e/settings-harness/index.html';
const ROOM = '!review:example.test';

const shot = async (page: Page, name: string) => {
  if (SHOTS) await page.screenshot({ path: `${SHOTS}/${name}.png` });
};

const avatar = (page: Page) => page.getByTestId('nav-bar-account');
const dialog = (page: Page) => page.getByRole('dialog', { name: 'Settings' });
const pageTitle = (page: Page) => dialog(page).getByRole('heading', { level: 2 });
const nav = (page: Page) => page.getByRole('navigation', { name: 'Settings sections' });

/** A 1×1 PNG for avatar requests, so the picture really loads. */
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==',
  'base64'
);
const serveMedia = async (page: Page) => {
  const png = { status: 200, contentType: 'image/png', body: PNG };
  await page.route('**/__synara-media/**', (route) => route.fulfill(png));
  // The bar resolves the avatar to the homeserver's thumbnail URL.
  await page.route('**/_matrix/**/thumbnail/**', (route) => route.fulfill(png));
};

test('the avatar opens Settings, and pressing it again closes it', async ({ page }) => {
  await page.goto(`${HARNESS}#/home/`);
  await expect(avatar(page)).toHaveAccessibleName('Settings');
  await expect(avatar(page)).toHaveAttribute('aria-expanded', 'false');
  await avatar(page).click();
  await expect(dialog(page)).toBeVisible();
  // No menu any more: Settings itself, on General the first time.
  await expect(page.getByRole('menu')).toHaveCount(0);
  await expect(pageTitle(page)).toHaveText('General');
  await expect(avatar(page)).toHaveAttribute('aria-expanded', 'true');
  // Log out is still at the foot of the Settings nav.
  await expect(dialog(page).getByRole('button', { name: 'Log out' })).toBeVisible();

  // The backdrop covers the avatar; a press where it is closes Settings and
  // the same press must not open it again.
  const box = (await avatar(page).boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  await expect(dialog(page)).toHaveCount(0);
  await page.waitForTimeout(600);
  await expect(dialog(page)).toHaveCount(0);
  await expect(avatar(page)).toHaveAttribute('aria-expanded', 'false');
  await expect(avatar(page)).toBeFocused();

  // Escape closes too, and focus returns to the avatar.
  await avatar(page).click();
  await expect(dialog(page)).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(dialog(page)).toHaveCount(0);
  await expect(avatar(page)).toBeFocused();
});

test('Settings reopens on the last page viewed', async ({ page }) => {
  await page.goto(`${HARNESS}#/home/`);
  await avatar(page).click();
  await expect(pageTitle(page)).toHaveText('General');
  await nav(page).getByRole('button', { name: 'Privacy', exact: true }).click();
  await expect(pageTitle(page)).toHaveText('Privacy');
  await page.keyboard.press('Escape');
  await expect(dialog(page)).toHaveCount(0);
  await page.waitForTimeout(500);
  await avatar(page).click();
  await expect(pageTitle(page)).toHaveText('Privacy');
  // The route without a page also returns there.
  await page.keyboard.press('Escape');
  await page.goto(`${HARNESS}#/settings/`);
  await expect(pageTitle(page)).not.toHaveText('');
});

test('a link opens Settings at a page and row', async ({ page }) => {
  await page.goto(`${HARNESS}#/settings/media-links/?setting=GIF%20search`);
  await expect(pageTitle(page)).toHaveText('Media & links');
  await expect(page.locator('[data-setting-title="GIF search"]').first()).toBeInViewport();
  await page.goto(`${HARNESS}#/settings/advanced/`);
  await expect(pageTitle(page)).toHaveText('Advanced');
  // An unknown page falls back to the remembered or first page.
  await page.goto(`${HARNESS}#/settings/developer-tools/`);
  await expect(pageTitle(page)).not.toHaveText('Developer tools');
});

test('an unverified session: the avatar warns and opens Security & recovery', async ({ page }) => {
  await page.goto(`${HARNESS}?unverified#/home/`);
  await expect(avatar(page)).toHaveAccessibleName('Settings, this device is not verified');
  await avatar(page).click();
  await expect(pageTitle(page)).toHaveText('Security & recovery');
  await expect(page.locator('[data-setting-title="Verify this session"]').first()).toBeVisible();
  await shot(page, 'settings-avatar-unverified');
});

test('the avatar keeps its picture moving between Home and Activity', async ({ page }) => {
  await serveMedia(page);
  await page.goto(`${HARNESS}?avatar#/home/`);
  const picture = avatar(page).locator('img');
  await expect(picture).toHaveAttribute('data-image-loaded', 'true');
  // Record every time the initial tile ("R" for Reviewer) is drawn in the bar.
  await page.evaluate(() => {
    const win = window as unknown as { fallbackSeen: number };
    win.fallbackSeen = 0;
    new MutationObserver(() => {
      const button = document.querySelector('[data-testid="nav-bar-account"]');
      if (button && !button.querySelector('img')) win.fallbackSeen += 1;
    }).observe(document.body, { childList: true, subtree: true });
  });
  for (const place of ['activity', 'approvals', 'home', 'activity', 'home']) {
    await page.getByTestId(`nav-bar-${place}`).click();
    await page.waitForTimeout(150);
    await expect(picture).toBeVisible();
  }
  expect(
    await page.evaluate(() => (window as unknown as { fallbackSeen: number }).fallbackSeen)
  ).toBe(0);
});

test('the composer keeps its GIF item with GIF search off, and /gif leads to the setting', async ({
  page,
}) => {
  // A provider is configured but GIF search is off (after "Not now").
  await page.addInitScript(() => {
    const stored = JSON.parse(localStorage.getItem('settings') ?? '{}');
    localStorage.setItem(
      'settings',
      JSON.stringify({
        ...stored,
        gifSearchEnabled: false,
        gifOnboardingDismissed: true,
        // Written by this version, so the 2.5 "GIFs on" migration leaves it.
        previewDefaultsVersion: 3,
      })
    );
  });
  // A provider with a key, as a packaged build has (a placeholder key: no
  // GIF request is made in this test).
  await page.route('**/config.json', async (route) => {
    const response = await route.fetch();
    const config = await response.json();
    config.gifPicker = { ...config.gifPicker, enabled: true, apiKey: 'harness-placeholder' };
    await route.fulfill({ response, json: config });
  });
  await page.goto(`${HARNESS}?composer#/home/${ROOM}/`);
  const more = page.getByRole('button', { name: 'More message actions' });
  await more.click();
  const gif = page
    .getByRole('menuitem', { name: 'GIF' })
    .or(page.getByRole('button', { name: 'GIF', exact: true }));
  await expect(gif).toBeVisible();
  await shot(page, 'composer-plus-menu-gif-off');
  await gif.click();
  const prompt = page.getByTestId('gif-enable-prompt');
  await expect(prompt).toContainText('Enable GIFs?');
  await shot(page, 'composer-enable-gifs-prompt');
  await prompt.getByRole('button', { name: 'Open settings' }).click();
  await expect(pageTitle(page)).toHaveText('Media & links');
  await expect(page.locator('[data-setting-title="GIF search"]').first()).toBeInViewport();
  await page.keyboard.press('Escape');
  await expect(dialog(page)).toHaveCount(0);

  // /gif says where the switch is, with an action that opens it.
  const composer = page.getByRole('textbox', { name: /message/i }).first();
  await composer.click();
  await page.keyboard.type('/gif cats');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Enter');
  const alert = page.getByRole('alert').filter({ hasText: 'GIF search' });
  await expect(alert).toContainText('Turn on GIF search in Settings → Media & links to use /gif.');
  await shot(page, 'composer-gif-command-off');
  await alert.getByRole('button', { name: 'Open settings' }).click();
  await expect(pageTitle(page)).toHaveText('Media & links');
});
