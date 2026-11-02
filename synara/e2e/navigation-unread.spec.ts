import { expect, test, type Locator } from '@playwright/test';

// Rail rule (Slack): a red count only for mentions and unread DMs, a dot for
// plain unread. Home is one list of rooms and DMs (NAV-12), so its badge
// counts both; the DM tab keeps its own DM count.
const badge = (rail: Locator) => rail.locator('[data-badge-role]');
const dot = (rail: Locator) => rail.locator('[data-badge-role="neutral"]');
const red = (rail: Locator) => rail.locator('[data-badge-role="mention"]');

test('DM aggregate updates while Home is selected even if m.direct readback lags', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&laggingMDirect');
  const home = page.getByTestId('home-rail');
  const dm = page.getByTestId('dm-rail');
  await expect(home.locator('button')).toHaveAttribute('aria-current', 'page');
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await expect(dm).toContainText('2');
  // The merged Home badge carries the same DM attention.
  await expect(red(home)).toHaveText('2');
  await page.getByRole('button', { name: 'Receive second DM' }).click();
  await expect(dm).toContainText('5');
  await expect(red(home)).toHaveText('5');
  await page.getByRole('button', { name: 'Direct messages', exact: true }).click();
  const destination = page.getByTestId('destination-rooms');
  await expect(destination).toContainText('!dm:example.test');
  await expect(destination).toContainText('!other-dm:example.test');
  await expect(destination).not.toContainText('!home:example.test');
  await expect(dm).toContainText('5');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(dm).toContainText('3');
  await page.getByRole('button', { name: 'Mark second DM read' }).click();
  await expect(dm).not.toContainText(/\d/);
  await expect(badge(dm)).toHaveCount(0);
  await expect(badge(home)).toHaveCount(0);
});

test('Home aggregate shows a dot for plain unread and a red count for mentions', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&directSelected');
  const home = page.getByTestId('home-rail');
  const dm = page.getByTestId('dm-rail');
  // `/direct/` routes live in the Home pane: Home stays the current tab and
  // the Direct tab is a filter, never a second destination.
  await expect(home.locator('button')).toHaveAttribute('aria-current', 'page');
  await expect(dm.locator('button')).not.toHaveAttribute('aria-current', 'page');
  await page.getByRole('button', { name: 'Receive Home' }).click();
  await expect(dot(home)).toHaveCount(1);
  await expect(home).not.toContainText(/\d/);
  await expect(dm).not.toContainText(/\d/);
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
  // Bottom-right of the tile.
  const tile = await home.locator('button').boundingBox();
  const pill = await mention.boundingBox();
  expect(pill!.x + pill!.width / 2).toBeGreaterThan(tile!.x + tile!.width / 2);
  expect(pill!.y + pill!.height / 2).toBeGreaterThan(tile!.y + tile!.height / 2);
  await page.getByRole('button', { name: 'Mark Home read' }).click();
  await expect(badge(home)).toHaveCount(0);
});

test('DM aggregate preserves mention-only, marked-unread, and mute behavior', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread');
  const dm = page.getByTestId('dm-rail');
  await page.getByRole('button', { name: 'Mark DM unread' }).click();
  await expect(dm).toContainText('1');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Mention only DM' }).click();
  await expect(dm.locator('[data-badge-role="mention"]')).toHaveText('1');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  // An ordinary unread DM is attention too: same red role as a mention.
  await expect(dm.locator('[data-badge-role="mention"]')).toHaveText('2');
  await page.getByRole('button', { name: 'Mute DM' }).click();
  await expect(dm).not.toContainText(/\d/);
  await expect(badge(dm)).toHaveCount(0);
});

test('rail Mark as Read clears exactly its badge and destination scope', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&laggingMDirect');
  const home = page.getByTestId('home-rail');
  const dm = page.getByTestId('dm-rail');
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await page.getByRole('button', { name: 'Receive Home', exact: true }).click();
  await expect(dm).toContainText('2');
  await expect(red(home)).toHaveText('2');
  await page
    .getByRole('button', { name: 'Direct messages', exact: true })
    .click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as Read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  // Only the plain unread room is left in the merged Home scope: a dot.
  await expect(dot(home)).toHaveCount(1);
  await expect(page.locator('body')).toHaveAttribute(
    'data-read-rooms',
    JSON.stringify(['!dm:example.test', '!other-dm:example.test'])
  );
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await expect(red(home)).toHaveText('2');
  // Home's Mark as Read covers its whole list: rooms and DMs.
  await page.getByRole('button', { name: 'Home', exact: true }).click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as Read', exact: true }).click();
  await expect(badge(home)).toHaveCount(0);
  await expect(badge(dm)).toHaveCount(0);
  await expect(page.locator('body')).toHaveAttribute(
    'data-read-rooms',
    JSON.stringify([
      '!dm:example.test',
      '!other-dm:example.test',
      '!dm:example.test',
      '!other-dm:example.test',
      '!home:example.test',
    ])
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

test('the Direct tab expands and scrolls to Direct messages in the one Home list', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&homePane');
  const pane = page.getByTestId('home-pane');
  const homeRail = page.getByTestId('home-rail');
  const section = pane.getByRole('button', { name: 'Direct messages', exact: true });
  await expect(pane.getByRole('button', { name: 'Rooms', exact: true })).toBeVisible();
  await expect(section).toHaveAttribute('aria-expanded', 'true');
  // Rooms come first; the DM section starts below the fold of a long list.
  await expect(section).not.toBeInViewport();

  // Collapse it, then ask for it from the rail.
  await section.scrollIntoViewIfNeeded();
  await section.click();
  await expect(section).toHaveAttribute('aria-expanded', 'false');
  await pane.getByRole('button', { name: 'Rooms', exact: true }).scrollIntoViewIfNeeded();
  await page.getByTestId('dm-rail').locator('button').click();
  await expect(page.getByTestId('current-route')).toHaveText('/direct/');
  await expect(section).toHaveAttribute('aria-expanded', 'true');
  await expect(section).toBeInViewport();
  await expect(homeRail.locator('button')).toHaveAttribute('aria-current', 'page');

  // A DM opened from the merged list keeps its route and its selected row.
  const dmRow = pane.locator('a[href="/direct/!dm%3Aexample.test"]');
  await dmRow.click();
  await expect(page.getByTestId('current-route')).toHaveText('/direct/!dm%3Aexample.test');
  await expect(pane.locator('[data-selected="true"]')).toHaveCount(1);
  await expect(pane.locator('[data-selected="true"]')).toContainText('!dm:example.test');
  await expect(homeRail.locator('button')).toHaveAttribute('aria-current', 'page');

  // From a Home room, the Direct tab reopens the last DM.
  await pane.locator('a[href="/home/!home%3Aexample.test"]').click();
  await expect(page.getByTestId('current-route')).toHaveText('/home/!home%3Aexample.test');
  await page.getByTestId('dm-rail').locator('button').click();
  await expect(page.getByTestId('current-route')).toHaveText('/direct/!dm%3Aexample.test');
  await expect(section).toBeInViewport();
});
