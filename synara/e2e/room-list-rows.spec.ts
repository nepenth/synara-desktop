import { expect, test, type Locator, type Page } from '@playwright/test';

const open = (page: Page, light = false) =>
  page.goto(`/e2e/room-list-harness/index.html?roomRows${light ? '&light' : ''}`);
const row = (page: Page, roomId: string) => page.locator(`[data-room-id="${roomId}"]`);
const nameOf = (r: Locator) => r.getByRole('link');
const weight = (r: Locator) =>
  nameOf(r)
    .locator('span')
    .last()
    .evaluate((el) => getComputedStyle(el).fontWeight);
const color = (r: Locator) =>
  nameOf(r)
    .locator('span')
    .last()
    .evaluate((el) => getComputedStyle(el).color);
const cssVar = (page: Page, name: string) =>
  page.evaluate(
    (variable) =>
      getComputedStyle(document.querySelector('main')!).getPropertyValue(variable).trim(),
    name
  );

test('a 56-room list keeps 32px rows, 15px names, bold unread and medium read', async ({
  page,
}) => {
  await open(page);
  const rows = page.getByTestId('room-row');
  await expect(rows).toHaveCount(56);
  const heights = await rows.evaluateAll((nodes) =>
    nodes.map((node) => Math.round(node.getBoundingClientRect().height))
  );
  expect(heights).toEqual(heights.map(() => 32));

  const unread = row(page, '!unread:example.test');
  const read = row(page, '!room-3:example.test');
  await expect(nameOf(unread).locator('span').last()).toHaveCSS('font-size', '15px');
  await expect(nameOf(unread).locator('span').last()).toHaveCSS('line-height', '20px');
  expect(await weight(unread)).toBe('700');
  expect(await weight(read)).toBe('500');
  // Plain unread carries no count badge: weight alone marks it.
  await expect(unread.locator('[data-badge-role]')).toHaveCount(0);
  await expect(read.locator('[data-badge-role]')).toHaveCount(0);

  // Section headers: 28px, 14px semibold.
  const header = page.getByRole('button', { name: 'Rooms', exact: true });
  const box = await header.boundingBox();
  expect(Math.round(box!.height)).toBe(28);
  await expect(header.locator('span').first()).toHaveCSS('font-size', '14px');
  await expect(header.locator('span').first()).toHaveCSS('font-weight', '600');
});

test('mentions and DMs get the red count; it survives hover and caps at 99+', async ({ page }) => {
  await open(page);
  const mention = row(page, '!mention:example.test');
  const badge = mention.locator('[data-badge-role="mention"]');
  await expect(badge).toHaveText('2');
  const critical = await badge.evaluate((el) => getComputedStyle(el).backgroundColor);
  // Dark theme Critical.Main is #E69D9D.
  expect(critical).toBe('rgb(230, 157, 157)');

  // Hovering reveals the kebab beside the badge; the badge stays mounted.
  await mention.hover();
  await expect(badge).toBeVisible();
  await expect(badge).toHaveText('2');
  const kebab = mention.getByRole('button', { name: 'More options' });
  await expect(kebab).toHaveCSS('opacity', '1');
  await expect(kebab).toBeVisible();
  // At rest the kebab slot is still laid out (no reflow) but invisible.
  await page.mouse.move(700, 700);
  const resting = mention.getByRole('button', { name: 'More options' });
  await expect(resting).toBeAttached();
  const slot = resting.locator('xpath=..');
  await expect(slot).toHaveCSS('opacity', '0');

  const dm = row(page, '!dm:example.test');
  await expect(dm.locator('[data-badge-role="mention"]')).toHaveText('4');
  await expect(row(page, '!dm-read:example.test').locator('[data-badge-role]')).toHaveCount(0);
  await expect(row(page, '!many:example.test').locator('[data-badge-role="mention"]')).toHaveText(
    '99+'
  );
});

test('muted, private and favourite rooms are distinguishable at rest', async ({ page }) => {
  await open(page);
  const muted = row(page, '!muted:example.test');
  const tertiary = await cssVar(page, '--synara-content-tertiary');
  const secondary = await cssVar(page, '--synara-content-secondary');
  expect(tertiary).not.toBe('');
  const toRgb = (hex: string) => {
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
    return `rgb(${r}, ${g}, ${b})`;
  };
  expect(await color(muted)).toBe(toRgb(tertiary));
  expect(await weight(muted)).toBe('500');
  await expect(muted.locator('[data-room-state="muted"]')).toHaveCount(1);
  await expect(muted.locator('[data-badge-role]')).toHaveCount(0);
  expect(await color(row(page, '!room-3:example.test'))).toBe(toRgb(secondary));

  const priv = row(page, '!private:example.test');
  await expect(priv.locator('[data-room-glyph="lock"]')).toHaveCount(1);
  await expect(priv.locator('[data-room-glyph="lock"] svg')).toHaveCount(1);
  await expect(priv).not.toContainText('#');
  await expect(row(page, '!room-3:example.test').locator('[data-room-glyph="hash"]')).toHaveText(
    '#'
  );
  const lockBox = await priv.locator('[data-room-glyph="lock"]').boundingBox();
  expect(Math.round(lockBox!.width)).toBe(18);

  // Favourites: star marker in lists that ask for it; the Favorites section
  // row itself does not repeat the star.
  await expect(row(page, '!fav:example.test').locator('[data-room-state="favorite"]')).toHaveCount(
    0
  );
  await expect(page.getByRole('button', { name: 'Favorites', exact: true })).toBeVisible();

  // Long names truncate inside the row instead of growing it.
  const long = row(page, '!long:example.test');
  expect(Math.round((await long.boundingBox())!.height)).toBe(32);
  await expect(long.locator('[data-badge-role="mention"]')).toHaveText('1');
});

test('the selected row is flat: tint and accent bar, no shadow, bold name', async ({ page }) => {
  await open(page);
  const fav = row(page, '!fav:example.test');
  const item = fav.locator('[data-selected="true"]');
  await expect(item).toHaveCount(1);
  await expect(item).toHaveCSS('box-shadow', 'none');
  expect(await weight(fav)).toBe('700');
  const selectedBg = await item.evaluate((el) => getComputedStyle(el).backgroundColor);
  expect(selectedBg).not.toBe('rgba(0, 0, 0, 0)');
  const hoverTarget = row(page, '!room-5:example.test').locator('[class]').first();
  await hoverTarget.hover();
  await expect(hoverTarget).toHaveCSS('box-shadow', 'none');
  await expect(hoverTarget).toHaveCSS('border-top-color', 'rgba(0, 0, 0, 0)');
});

test('collapsing Rooms keeps unread and selected rows visible', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Rooms', exact: true }).click();
  const rows = page.getByTestId('room-row');
  // 1 favourite + mention, unread, private-unread, long, dm, many (muted and read rows hide).
  await expect(rows).toHaveCount(7);
  await expect(row(page, '!muted:example.test')).toHaveCount(0);
  await expect(row(page, '!unread:example.test')).toHaveCount(1);
});

test('light theme keeps the same roles', async ({ page }) => {
  await open(page, true);
  const badge = row(page, '!mention:example.test').locator('[data-badge-role="mention"]');
  // Light theme Critical.Main is #9D0F0F with white digits.
  await expect(badge).toHaveCSS('background-color', 'rgb(157, 15, 15)');
  await expect(badge).toHaveCSS('color', 'rgb(255, 255, 255)');
  await expect(page.getByTestId('nav-search-field')).toHaveText('Search messages');
});
