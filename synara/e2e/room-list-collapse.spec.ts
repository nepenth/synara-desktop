import { expect, test, type Page } from '@playwright/test';

// The room list pane folds away (the list header's button, Cmd/Ctrl+Shift+D).
// It stays mounted but inert while hidden, and the page header then shows
// "Show room list" carrying the hidden list's unread by the Slack rule.
const open = (page: Page, query = '') =>
  page.goto(`/e2e/room-list-harness/index.html?navigationUnread&collapsible${query}`);
const pane = (page: Page) => page.locator('#synara-room-list-pane');
const hide = (page: Page) => pane(page).getByTestId('room-list-hide');
/** "Show room list" in the page header: present only while the list is hidden. */
const toggle = (page: Page) => page.locator('#synara-room-list-toggle');
const toggleBadge = (page: Page) =>
  page.getByTestId('room-list-badge').locator('[data-badge-role]');
const paneWidth = (page: Page) =>
  pane(page).evaluate((el) => Math.round(el.getBoundingClientRect().width));
/** The app's platform comes from the user agent: Desktop Safari is macOS. */
const pressToggleShortcut = async (page: Page) => {
  const mac = await page.evaluate(() => navigator.userAgent.includes('Mac'));
  await page.keyboard.press(mac ? 'Meta+Shift+D' : 'Control+Shift+D');
};
const paneScroller = (page: Page) =>
  pane(page).evaluateHandle((root) => {
    const all = [root, ...Array.from(root.querySelectorAll<HTMLElement>('*'))];
    return all.find(
      (el) => el.scrollHeight > el.clientHeight + 10 && getComputedStyle(el).overflowY !== 'visible'
    )!;
  });

test('the list header button folds the pane away and the page header brings it back', async ({
  page,
}) => {
  await open(page);
  await expect(pane(page).getByTestId('nav-search-field')).toBeVisible();
  await expect(hide(page)).toHaveAttribute('aria-expanded', 'true');
  await expect(hide(page)).toHaveAttribute('aria-controls', 'synara-room-list-pane');
  await expect(hide(page)).toHaveAccessibleName('Hide room list');
  await expect(toggle(page)).toHaveCount(0);
  const expandedWidth = await paneWidth(page);
  expect(expandedWidth).toBeGreaterThanOrEqual(196);

  await hide(page).click();
  await expect(toggle(page)).toHaveAttribute('aria-expanded', 'false');
  await expect(toggle(page)).toHaveAttribute('aria-controls', 'synara-room-list-pane');
  await expect(toggle(page)).toHaveAccessibleName('Show room list');
  await expect(pane(page)).toHaveAttribute('inert', '');
  await expect(pane(page)).toHaveAttribute('aria-hidden', 'true');
  await expect.poll(() => paneWidth(page)).toBe(0);
  // Hidden from assistive technology, and nothing painted: no stray divider.
  await expect(page.getByRole('navigation', { name: 'Rooms' })).toHaveCount(0);
  await expect(pane(page)).toHaveCSS('visibility', 'hidden');
  const lines = await page.evaluate(() => {
    const paneBox = document.getElementById('synara-room-list-pane')!.getBoundingClientRect();
    return Array.from(document.querySelectorAll<HTMLElement>('main *')).filter((el) => {
      const box = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      return (
        Math.abs(box.left - paneBox.left) < 3 &&
        box.width > 0 &&
        box.width <= 2 &&
        box.height > 100 &&
        style.visibility !== 'hidden' &&
        style.display !== 'none'
      );
    }).length;
  });
  expect(lines).toBe(0);
  // Nothing focused is left inside the inert pane (WebKit does not focus clicked buttons).
  expect(
    await page.evaluate(() =>
      document.getElementById('synara-room-list-pane')!.contains(document.activeElement)
    )
  ).toBe(false);

  await toggle(page).click();
  await expect(pane(page)).not.toHaveAttribute('inert');
  await expect(pane(page)).not.toHaveAttribute('aria-hidden');
  await expect.poll(() => paneWidth(page)).toBe(expandedWidth);
  await expect(page.getByRole('navigation', { name: 'Rooms' })).toBeVisible();
  await expect(toggle(page)).toHaveCount(0);
});

test('the pane keeps its scroll position and its resize handle', async ({ page }) => {
  await open(page);
  const before = await paneWidth(page);
  const scroller = await paneScroller(page);
  await scroller.evaluate((el) => {
    (el as HTMLElement).scrollTop = 200;
  });
  await hide(page).click();
  await expect.poll(() => paneWidth(page)).toBe(0);
  await toggle(page).click();
  await expect.poll(() => paneWidth(page)).toBe(before);
  expect(await scroller.evaluate((el) => (el as HTMLElement).scrollTop)).toBe(200);

  const handle = page.getByRole('button', { name: /Resize navigation pane/ });
  await handle.focus();
  await page.keyboard.press('ArrowRight');
  await expect.poll(() => paneWidth(page)).toBe(before + 12);
  expect(await page.evaluate(() => localStorage.getItem('synara.navigationPaneWidth'))).toBe(
    String(before + 12)
  );
});

test('Cmd/Ctrl+Shift+D toggles from the composer and from the list', async ({ page }) => {
  await open(page);
  // Typing in the composer: the shortcut works and the composer keeps focus.
  const composer = page.getByRole('textbox', { name: 'Composer', exact: true });
  await composer.click();
  await pressToggleShortcut(page);
  await expect(toggle(page)).toHaveAttribute('aria-expanded', 'false');
  await expect(composer).toBeFocused();
  await expect(composer).toHaveValue('');
  await pressToggleShortcut(page);
  await expect(toggle(page)).toHaveCount(0);
  await expect(hide(page)).toHaveAttribute('aria-expanded', 'true');
  await expect(composer).toBeFocused();

  const rich = page.getByRole('textbox', { name: 'Rich composer' });
  await rich.click();
  await pressToggleShortcut(page);
  await expect(toggle(page)).toHaveAttribute('aria-expanded', 'false');
  await expect(rich).toHaveText('Draft');
  await pressToggleShortcut(page);
  await expect(toggle(page)).toHaveCount(0);

  // Hiding with focus in the list hands focus to "Show room list", not <body>.
  const search = pane(page).getByTestId('nav-search-field');
  await search.focus();
  await pressToggleShortcut(page);
  await expect(toggle(page)).toBeFocused();
  // Showing from the keyboard puts focus back in the list.
  await pressToggleShortcut(page);
  await expect(search).toBeFocused();

  // With a room open, its row takes focus.
  const room = page.getByRole('link', { name: /!filler-0:example\.test/ });
  await room.click();
  // WebKit does not focus a clicked link.
  await room.focus();
  await pressToggleShortcut(page);
  await expect(toggle(page)).toBeFocused();
  await pressToggleShortcut(page);
  await expect(pane(page).locator('a[aria-current="page"]')).toBeFocused();

  // The header buttons hand focus across too: hide → show → back in the list.
  await hide(page).focus();
  await page.keyboard.press('Enter');
  await expect(toggle(page)).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(pane(page).locator('a[aria-current="page"]')).toBeFocused();
});

test('the hidden state persists across a reload, without animating in', async ({ page }) => {
  await open(page);
  await hide(page).click();
  await expect.poll(() => paneWidth(page)).toBe(0);
  expect(
    await page.evaluate(
      () => JSON.parse(localStorage.getItem('platformSettings')!).roomListCollapsed
    )
  ).toBe(true);
  await page.reload();
  await expect(toggle(page)).toHaveAttribute('aria-expanded', 'false');
  expect(await paneWidth(page)).toBe(0);
  await expect(pane(page)).toHaveAttribute('inert', '');
  await toggle(page).click();
  await page.reload();
  await expect(hide(page)).toHaveAttribute('aria-expanded', 'true');
  await expect(toggle(page)).toHaveCount(0);
  expect(await paneWidth(page)).toBeGreaterThan(0);
});

test('with the list hidden, "Show room list" carries Home unread by the Slack rule', async ({
  page,
}) => {
  await open(page);
  await page.getByRole('button', { name: 'Receive Home' }).click();
  await hide(page).click();
  // The bottom bar hides with the list, so the toggle is where unread shows.
  await expect(toggleBadge(page)).toHaveAttribute('data-badge-role', 'neutral');
  await expect(toggleBadge(page)).not.toContainText(/\d/);
  await expect(toggle(page)).toHaveAccessibleName('Show room list, unread messages');
  await page.getByRole('button', { name: 'Receive DM' }).click();
  await expect(toggleBadge(page)).toHaveAttribute('data-badge-role', 'mention');
  await expect(toggleBadge(page)).toHaveText('2');
  await expect(toggle(page)).toHaveAccessibleName('Show room list, 2 mentions and direct messages');
  await page.getByRole('button', { name: 'Mark all read' }).click();
  await expect(toggleBadge(page)).toHaveCount(0);
  await expect(toggle(page)).toHaveAccessibleName('Show room list');
});

test('the pane slides over 200ms, and instantly with reduced motion', async ({ page }) => {
  await open(page);
  await expect(pane(page)).toHaveCSS('transition-duration', /0\.2s/);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(pane(page)).toHaveCSS('transition-duration', '0s');
  // The width jumps on the next frame instead of easing.
  const widthAfterAFrame = await page.evaluate(async () => {
    document.querySelector<HTMLElement>('[data-testid="room-list-hide"]')!.click();
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    return document.getElementById('synara-room-list-pane')!.getBoundingClientRect().width;
  });
  expect(widthAfterAFrame).toBe(0);
  await expect(pane(page)).toHaveCSS('visibility', 'hidden');
});
