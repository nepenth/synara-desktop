import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, test, type Locator, type Page } from '@playwright/test';
import {
  SHORT_VIEWPORT,
  VIEWPORT_HEIGHT,
  VIEWPORT_WIDTHS,
  expectInsideViewport,
  hideDevtools,
  isMobileWidth,
} from './viewportMatrix';

// "What's new" after an update, from the production app shell
// (e2e/settings-harness). `?whatsNew` starts the harness without a record, as
// a signed-in session updated from a version before the feature; `?signedOut`
// starts with no session, as a fresh install. Screenshots are written only
// when SETTINGS_SHOTS_DIR is set.
const SHOTS = process.env.SETTINGS_SHOTS_DIR;
const HARNESS = '/e2e/settings-harness/index.html';
const ROOM = '!review:example.test';
const ROOM_PATH = `#/home/${encodeURIComponent(ROOM)}/`;
// The card waits 1.5 s after the shell is ready (WHATS_NEW_SHOW_DELAY_MS).
const AFTER_DELAY_MS = 2500;

const readJson = (path: string) =>
  JSON.parse(readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8'));
const VERSION: string = readJson('../package.json').version;
const NOTES: { headline: string; items: { text: string; platforms: string[] }[] } = readJson(
  `../../release-notes/v${VERSION}.json`
);
const DESKTOP_ITEMS = NOTES.items
  .filter((item) => item.platforms.includes('desktop'))
  .map((item) => item.text);

const shot = async (page: Page, name: string) => {
  if (SHOTS) await page.screenshot({ path: `${SHOTS}/${name}.png` });
};

const card = (page: Page) => page.locator('[data-whats-new-card]');
const seenVersion = (page: Page) =>
  page.evaluate(
    () => JSON.parse(localStorage.getItem('platformSettings') ?? '{}').whatsNewSeenVersion ?? ''
  );

const open = async (page: Page, query: string, hash = ROOM_PATH) => {
  await page.goto(`${HARNESS}?${query}${hash}`);
  await hideDevtools(page);
};

const expectNoOverlap = async (a: Locator, b: Locator, label: string) => {
  const [boxA, boxB] = [await a.boundingBox(), await b.boundingBox()];
  expect(boxA, `${label}: card has a box`).not.toBeNull();
  expect(boxB, `${label}: has a box`).not.toBeNull();
  const overlap =
    boxA!.x < boxB!.x + boxB!.width &&
    boxB!.x < boxA!.x + boxA!.width &&
    boxA!.y < boxB!.y + boxB!.height &&
    boxB!.y < boxA!.y + boxA!.height;
  expect(overlap, `the card covers ${label}`).toBe(false);
};

for (const colorScheme of ['light', 'dark'] as const) {
  test(`an updated session sees the card once (${colorScheme})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme });
    await open(page, `whatsNew&composer${colorScheme === 'light' ? '&theme=light' : ''}`);
    await expect(card(page)).toBeVisible({ timeout: 10000 });
    await expect(card(page).getByRole('heading', { level: 2 })).toHaveText(
      `What's new in ${VERSION}`
    );
    await expect(card(page).getByRole('status')).toContainText(NOTES.headline);
    await expect(card(page).getByRole('listitem')).toHaveText(DESKTOP_ITEMS);
    await expect(card(page).getByRole('link', { name: 'Full release notes' })).toHaveAttribute(
      'href',
      `https://github.com/nepenth/synara-desktop/releases/tag/v${VERSION}`
    );
    // It appears without taking focus.
    expect(await card(page).evaluate((element) => element.contains(document.activeElement))).toBe(
      false
    );
    // Not recorded yet: only a dismissal, or closing the app after it was seen.
    expect(await seenVersion(page)).toBe('');
    await expectInsideViewport(card(page));
    await shot(page, `whats-new-card-${colorScheme}`);

    await card(page).getByRole('button', { name: "Close What's new" }).click();
    await expect(card(page)).toHaveCount(0);
    expect(await seenVersion(page)).toBe(VERSION);

    await page.reload();
    await page.waitForTimeout(AFTER_DELAY_MS);
    await expect(card(page)).toHaveCount(0);
  });
}

test('closing the window after the card was seen records it', async ({ page }) => {
  await open(page, 'whatsNew');
  await expect(card(page)).toBeVisible({ timeout: 10000 });
  // A reload unloads the page as quitting does (pagehide; the shell also
  // sends synara://before-quit).
  await page.reload();
  expect(await seenVersion(page)).toBe(VERSION);
  await page.waitForTimeout(AFTER_DELAY_MS);
  await expect(card(page)).toHaveCount(0);
});

test('closing the window before the card appeared leaves it for next time', async ({ page }) => {
  await open(page, 'whatsNew');
  await page.reload();
  expect(await seenVersion(page)).toBe('');
  await expect(card(page)).toBeVisible({ timeout: 10000 });
});

test('Escape in the card dismisses it and gives focus back', async ({ page }) => {
  await open(page, 'whatsNew&composer');
  await expect(card(page)).toBeVisible({ timeout: 10000 });
  const composer = page.getByRole('textbox', { name: 'Message Design review' });
  await composer.click();
  // Shift+Tab from the composer is not guaranteed to land on the card, so
  // move focus in the way a Tab would, with the composer as where it came from.
  await card(page).getByRole('button', { name: "Close What's new" }).focus();
  await page.keyboard.press('Escape');
  await expect(card(page)).toHaveCount(0);
  expect(await seenVersion(page)).toBe(VERSION);
  await expect(composer).toBeFocused();
});

test('a fresh install records the version and never shows the card', async ({ page }) => {
  await open(page, 'whatsNew&signedOut', '#/home/');
  await expect(page).toHaveURL(/#\/login/);
  await expect.poll(() => seenVersion(page)).toBe(VERSION);
  await expect(card(page)).toHaveCount(0);
  // Signed in later: nothing to show for this version.
  await open(page, 'whatsNew');
  await page.waitForTimeout(AFTER_DELAY_MS);
  await expect(card(page)).toHaveCount(0);
});

test('the card waits for the verification prompt and hides behind Settings', async ({ page }) => {
  await open(page, 'whatsNew&verifyPrompt=1', '#/home/');
  const prompt = page.getByRole('dialog', { name: 'Verify this device' });
  await expect(prompt).toBeVisible();
  await page.waitForTimeout(AFTER_DELAY_MS);
  await expect(card(page)).toHaveCount(0);
  await prompt.getByRole('button', { name: 'Later', exact: true }).click();
  await expect(card(page)).toBeVisible();

  await page.getByTestId('nav-bar-account').click();
  await expect(page.getByRole('dialog', { name: 'Settings' })).toBeVisible();
  await expect(card(page)).toHaveCount(0);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog', { name: 'Settings' })).toHaveCount(0);
  await expect(card(page)).toBeVisible();
});

test('Settings › About shows the same notes', async ({ page }) => {
  await open(page, '', '#/settings/about/');
  const section = page.locator('[data-settings-section][data-setting-title="What\'s new"]');
  await expect(section).toBeVisible();
  await expect(section).toContainText(`${VERSION}: ${NOTES.headline}`);
  await expect(section.getByRole('listitem')).toHaveText(DESKTOP_ITEMS);
  await expect(section.getByRole('link', { name: 'Full release notes' })).toHaveAttribute(
    'href',
    `https://github.com/nepenth/synara-desktop/releases/tag/v${VERSION}`
  );
  await expectInsideViewport(section);
  await shot(page, 'whats-new-about');

  // Settings search finds it.
  await page.getByRole('textbox', { name: 'Search settings' }).fill('release notes');
  await expect(page.locator('[data-settings-search-result="What\'s new"]')).toBeVisible();
});

test('the update prompt lists the offered version’s notes', async ({ page }) => {
  await open(page, 'updatePrompt', '#/settings/general/');
  await page.getByRole('button', { name: 'Check for updates' }).click();
  const prompt = page.getByRole('alertdialog', { name: /Synara 3\.2\.0 is available/ });
  await expect(prompt).toBeVisible();
  await expect(prompt).toContainText('Version 3.2.0 is available.');
  const notes = prompt.locator('[data-update-notes]');
  await expect(notes).toContainText('Easier settings, cleaner timestamps');
  await expect(notes.getByRole('listitem')).toHaveCount(4);
  await expect(notes.getByRole('listitem').first()).toHaveText(
    'Settings are grouped by what you want to do, and search understands phrases like "enable gif".'
  );
  await expectInsideViewport(prompt);
  await shot(page, 'whats-new-update-prompt');
});

const layouts = [
  ...VIEWPORT_WIDTHS.map((width) => ({ width, height: VIEWPORT_HEIGHT })),
  SHORT_VIEWPORT,
];
for (const viewport of layouts) {
  test(`the card clears the composer and the bottom bar at ${viewport.width}x${viewport.height}`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await open(page, 'whatsNew&composer&rooms=60');
    await expect(card(page)).toBeVisible({ timeout: 10000 });
    await expectInsideViewport(card(page));
    await expectNoOverlap(
      card(page),
      page.getByRole('textbox', { name: 'Message Design review' }),
      'the composer'
    );
    if (!isMobileWidth(viewport.width)) {
      await expectNoOverlap(card(page), page.getByTestId('nav-bottom-bar'), 'the bottom bar');
      await expectNoOverlap(
        card(page),
        page.locator('header').getByRole('button', { name: 'More Options' }),
        'the room header'
      );
    }
    if (viewport.width === 380)
      await shot(page, `whats-new-card-${viewport.width}x${viewport.height}`);

    // The list on its own (Mobile) or beside the room: the bar stays clear.
    // In-app navigation; a reload would record the card as seen.
    await page.evaluate(() => {
      window.location.hash = '#/home/';
    });
    await expect(page.getByTestId('nav-bottom-bar')).toBeVisible();
    await expect(card(page)).toBeVisible();
    await expectInsideViewport(card(page));
    await expectNoOverlap(card(page), page.getByTestId('nav-bottom-bar'), 'the bottom bar');
  });
}

test('tiling mode with the room list collapsed keeps the card clear', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: VIEWPORT_HEIGHT });
  await page.addInitScript(() => {
    if (sessionStorage.getItem('seeded')) return;
    sessionStorage.setItem('seeded', '1');
    localStorage.setItem(
      'platformSettings',
      JSON.stringify({ tilingMode: 'on', roomListCollapsed: true })
    );
  });
  await open(page, 'whatsNew&composer');
  await expect(card(page)).toBeVisible({ timeout: 10000 });
  await expectInsideViewport(card(page));
  await expectNoOverlap(
    card(page),
    page.getByRole('textbox', { name: 'Message Design review' }),
    'the composer'
  );
  await expectNoOverlap(
    card(page),
    page.locator('header').getByRole('button', { name: 'More Options' }),
    'the room header'
  );
  await shot(page, 'whats-new-card-tiling-collapsed');
});
