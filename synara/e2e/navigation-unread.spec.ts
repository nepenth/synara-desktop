import { expect, test, type Locator } from '@playwright/test';

// Slack rule: a red count only for mentions and unread DMs, a dot for plain
// unread. Home is one list of rooms and DMs (NAV-12) and the bottom bar's only
// way to them, so its badge carries every DM's attention.
const badge = (rail: Locator) => rail.locator('[data-badge-role]');
const dot = (rail: Locator) => rail.locator('[data-badge-role="neutral"]');
const red = (rail: Locator) => rail.locator('[data-badge-role="mention"]');

test('the Home badge counts unread DMs even if m.direct readback lags', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&laggingMDirect');
  const home = page.getByTestId('home-rail');
  await expect(home.locator('button')).toHaveAttribute('aria-current', 'page');
  await expect(badge(home)).toHaveCount(0);
  // No separate Direct messages item on the bar.
  await expect(page.getByRole('button', { name: 'Direct messages', exact: true })).toHaveCount(0);
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await expect(red(home)).toHaveText('2');
  await page.getByRole('button', { name: 'Receive second DM' }).click();
  await expect(red(home)).toHaveText('5');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(red(home)).toHaveText('3');
  await page.getByRole('button', { name: 'Mark second DM read' }).click();
  await expect(badge(home)).toHaveCount(0);
});

test('Home aggregate shows a dot for plain unread and a red count for mentions', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&directSelected');
  const home = page.getByTestId('home-rail');
  // `/direct/` routes live in the Home pane: Home stays the current tab.
  await expect(home.locator('button')).toHaveAttribute('aria-current', 'page');
  await page.getByRole('button', { name: 'Receive Home' }).click();
  await expect(dot(home)).toHaveCount(1);
  await expect(home).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Mention Home' }).click();
  const mention = home.locator('[data-badge-role="mention"]');
  await expect(mention).toHaveText('1');
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  // Room mentions plus unread DM messages: one attention count.
  await expect(mention).toHaveText('3');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(mention).toHaveText('1');
  // Dark theme Critical.Main.
  await expect(mention).toHaveCSS('background-color', 'rgb(230, 157, 157)');
  // Top-right of the icon.
  const icon = await home.locator('button svg').first().boundingBox();
  const pill = await mention.boundingBox();
  expect(pill!.x + pill!.width / 2).toBeGreaterThan(icon!.x + icon!.width / 2);
  expect(pill!.y + pill!.height / 2).toBeLessThan(icon!.y + icon!.height / 2);
  // The item's name carries the count; the badge itself is hidden from AT.
  await expect(home.locator('button')).toHaveAccessibleName('Home, 1 mention or direct message');
  await page.getByRole('button', { name: 'Mark Home read' }).click();
  await expect(badge(home)).toHaveCount(0);
});

test('DM attention in the Home badge keeps mention-only, marked-unread and mute', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread');
  const home = page.getByTestId('home-rail');
  await page.getByRole('button', { name: 'Mark DM unread' }).click();
  await expect(red(home)).toHaveText('1');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(badge(home)).toHaveCount(0);
  await page.getByRole('button', { name: 'Mention only DM' }).click();
  await expect(red(home)).toHaveText('1');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(badge(home)).toHaveCount(0);
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  // An ordinary unread DM is attention too: same red role as a mention.
  await expect(red(home)).toHaveText('2');
  await page.getByRole('button', { name: 'Mute DM' }).click();
  await expect(home).not.toContainText(/\d/);
  await expect(badge(home)).toHaveCount(0);
});

test('Home Mark as read clears its whole list, rooms and DMs', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&laggingMDirect');
  const home = page.getByTestId('home-rail');
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await page.getByRole('button', { name: 'Receive Home', exact: true }).click();
  await expect(red(home)).toHaveText('2');
  await home.locator('button').click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as read', exact: true }).click();
  await expect(badge(home)).toHaveCount(0);
  await expect(page.locator('body')).toHaveAttribute(
    'data-read-rooms',
    JSON.stringify(['!dm:example.test', '!other-dm:example.test', '!home:example.test'])
  );
});

test('generic room and thread navigation agrees with DM membership during m.direct lag', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&laggingMDirect');
  await expect(page.getByTestId('destination-rooms')).toContainText('!home:example.test');
  await page.getByRole('button', { name: 'Open DM via navigation', exact: true }).click();
  await expect(page.getByTestId('current-route')).toHaveText('/direct/!dm%3Aexample.test');
  await expect(page.getByTestId('destination-rooms')).toContainText('!dm:example.test');
  await page.getByRole('button', { name: 'Open DM thread via navigation', exact: true }).click();
  await expect(page.getByTestId('current-route')).toHaveText(
    '/direct/!dm%3Aexample.test/thread/%24thread'
  );
  await page.getByRole('button', { name: 'Open Home via navigation', exact: true }).click();
  await expect(page.getByTestId('current-route')).toHaveText('/home/!home%3Aexample.test');
  await expect(page.getByTestId('destination-rooms')).toContainText('!home:example.test');
  await expect(page.getByTestId('destination-rooms')).not.toContainText('!dm:example.test');
});

test('DMs open from the Direct messages section of the one Home list', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&homePane');
  const pane = page.getByTestId('home-pane');
  const homeRail = page.getByTestId('home-rail');
  const section = pane.getByRole('button', { name: 'Direct messages', exact: true });
  await expect(pane.getByRole('button', { name: 'Rooms', exact: true })).toBeVisible();
  await expect(section).toHaveAttribute('aria-expanded', 'true');
  // Rooms come first; the DM section starts below the fold of a long list.
  await expect(section).not.toBeInViewport();
  await section.scrollIntoViewIfNeeded();
  await expect(section).toBeInViewport();

  // A DM opened from the merged list keeps its route and its selected row.
  const dmRow = pane.locator('a[href="/direct/!dm%3Aexample.test"]');
  await dmRow.click();
  await expect(page.getByTestId('current-route')).toHaveText('/direct/!dm%3Aexample.test');
  await expect(pane.locator('[data-selected="true"]')).toHaveCount(1);
  await expect(pane.locator('[data-selected="true"]')).toContainText('!dm:example.test');
  await expect(homeRail.locator('button')).toHaveAttribute('aria-current', 'page');

  // A Home room from the same list moves the selection, not the bar.
  await pane.locator('a[href="/home/!home%3Aexample.test"]').click();
  await expect(page.getByTestId('current-route')).toHaveText('/home/!home%3Aexample.test');
  await expect(pane.locator('[data-selected="true"]')).toContainText('!home:example.test');
  await expect(homeRail.locator('button')).toHaveAttribute('aria-current', 'page');

  // The pane's search field opens the global search.
  const field = pane.getByTestId('nav-search-field');
  await expect(field).toHaveAttribute('aria-expanded', 'false');
  await field.click();
  await expect(field).toHaveAttribute('aria-expanded', 'true');
});
