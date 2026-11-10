import { expect, test, type Page } from '@playwright/test';

// Window shortcuts on the production Home pane with 400 rooms: previous/next
// room and next/previous unread walk the list's data, so they reach rows the
// virtual list has not rendered, and scroll them into view. Then F6 focus
// cycling and the shortcut help sheet, both from the one registry.
const open = (page: Page) =>
  page.goto('/e2e/room-list-harness/index.html?navigationUnread&collapsible&manyRooms');
const pane = (page: Page) => page.locator('#synara-room-list-pane');
const row = (page: Page, roomId: string) =>
  pane(page).locator(`a[data-room-nav-link][data-room-nav-id="${roomId}"]`);
const current = (page: Page) => pane(page).locator('a[data-room-nav-link][aria-current="page"]');
const isMac = (page: Page) => page.evaluate(() => navigator.userAgent.includes('Mac'));

test('next unread and previous room reach rows far outside the rendered window', async ({
  page,
}) => {
  await open(page);
  await expect(row(page, '!filler-000:example.test')).toBeVisible();
  // Far below the fold: not rendered at all yet.
  await expect(row(page, '!filler-350:example.test')).toHaveCount(0);
  await expect(row(page, '!filler-399:example.test')).toHaveCount(0);

  await row(page, '!filler-005:example.test').click();
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-005:example.test');

  // Typing in the composer: the jump works and the composer keeps focus.
  const composer = page.getByRole('textbox', { name: 'Composer', exact: true });
  await composer.click();
  await page.keyboard.press('Alt+Shift+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-350:example.test');
  await expect(row(page, '!filler-350:example.test')).toBeInViewport();
  await expect(composer).toBeFocused();

  await page.keyboard.press('Alt+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-351:example.test');
  await expect(current(page)).toBeInViewport();
  // The only unread room is behind us: the jump wraps to it.
  await page.keyboard.press('Alt+Shift+ArrowDown');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-350:example.test');

  // Previous room from a focused row.
  await row(page, '!filler-350:example.test').focus();
  await page.keyboard.press('Alt+ArrowUp');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-349:example.test');
  // Focus moves with the selection when it was on a row.
  await expect(current(page)).toBeFocused();
});

test('with no room open, previous room opens the last row of the list', async ({ page }) => {
  await open(page);
  await expect(row(page, '!filler-000:example.test')).toBeVisible();
  await page.getByRole('textbox', { name: 'Composer', exact: true }).click();
  await page.keyboard.press('Alt+ArrowUp');
  // The last Direct messages row, below 400 rooms.
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!other-dm:example.test');
  await expect(current(page)).toBeInViewport();
  await page.keyboard.press('Alt+ArrowUp');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!dm:example.test');
  await page.keyboard.press('Alt+ArrowUp');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!home:example.test');
  await expect(current(page)).toBeInViewport();
});

test('F6 and Shift+F6 cycle focus between the list, the timeline and the composer', async ({
  page,
}) => {
  await open(page);
  await row(page, '!filler-002:example.test').click();
  const composer = page.getByRole('textbox', { name: 'Rich composer' });
  const timeline = page.getByRole('region', { name: 'Messages' });
  await composer.click();
  // No thread pane: from the composer, F6 wraps to the list's open room.
  await page.keyboard.press('F6');
  await expect(current(page)).toBeFocused();
  await page.keyboard.press('F6');
  await expect(timeline).toBeFocused();
  await page.keyboard.press('F6');
  await expect(composer).toBeFocused();
  await page.keyboard.press('Shift+F6');
  await expect(timeline).toBeFocused();
  await page.keyboard.press('Shift+F6');
  await expect(current(page)).toBeFocused();
});

test('the help sheet lists the registry by group and closes again', async ({ page }) => {
  await open(page);
  await expect(row(page, '!filler-000:example.test')).toBeVisible();
  const help = (await isMac(page)) ? 'Meta+/' : 'Control+/';
  await page.keyboard.press(help);
  const sheet = page.getByRole('dialog', { name: 'Keyboard shortcuts' });
  await expect(sheet).toBeVisible();
  for (const group of ['General', 'Navigation', 'Rooms and threads', 'View']) {
    await expect(sheet.getByRole('heading', { name: group, exact: true })).toBeVisible();
  }
  await expect(sheet.locator('[data-shortcut-id="next-unread-room"]')).toContainText(
    'Next unread room'
  );
  await expect(sheet.locator('[data-shortcut-id="next-unread-room"] kbd')).toHaveText(
    (await isMac(page)) ? '⌥⇧↓' : 'Alt+Shift+↓'
  );
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);
  await page.keyboard.press(help);
  await expect(sheet).toBeVisible();
  await page.keyboard.press(help);
  await expect(sheet).toHaveCount(0);
});

test('vim-style list keys: j/k, g g and G move focus through the whole list, Enter opens', async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem('platformSettings', JSON.stringify({ vimListKeys: 'on' }))
  );
  await open(page);
  await row(page, '!filler-005:example.test').click();
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-005:example.test');
  await row(page, '!filler-005:example.test').focus();
  const focused = () =>
    page.evaluate(() => document.activeElement?.getAttribute('data-room-nav-id') ?? null);

  await page.keyboard.press('j');
  await expect.poll(focused).toBe('!filler-006:example.test');
  await page.keyboard.press('k');
  await page.keyboard.press('k');
  await expect.poll(focused).toBe('!filler-004:example.test');
  // Moving focus does not open the room; Enter does.
  await expect(current(page)).toHaveAttribute('data-room-nav-id', '!filler-005:example.test');

  // G: the last row, far below the rendered window.
  await page.keyboard.press('Shift+G');
  await expect.poll(focused).toBe('!other-dm:example.test');
  await expect(row(page, '!other-dm:example.test')).toBeInViewport();
  // g g: the first row.
  await page.keyboard.press('g');
  await page.keyboard.press('g');
  await expect
    .poll(async () => {
      const first = await pane(page).locator('a[data-room-nav-link]').first();
      return (await focused()) === (await first.getAttribute('data-room-nav-id'));
    })
    .toBe(true);
  await page.keyboard.press('j');
  const target = await focused();
  await page.keyboard.press('Enter');
  await expect(current(page)).toHaveAttribute('data-room-nav-id', target!);

  // Never while typing: j goes into the composer.
  const composer = page.getByRole('textbox', { name: 'Composer', exact: true });
  await composer.click();
  await page.keyboard.type('jk');
  await expect(composer).toHaveValue('jk');
  await expect(composer).toBeFocused();
});

test('vim-style list keys are off by default outside tiling mode', async ({ page }) => {
  await open(page);
  await row(page, '!filler-005:example.test').click();
  await row(page, '!filler-005:example.test').focus();
  await page.keyboard.press('j');
  await expect(row(page, '!filler-005:example.test')).toBeFocused();
});
