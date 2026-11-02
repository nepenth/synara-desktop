import { expect, test, type Locator } from '@playwright/test';

// Rail rule (Slack): Home shows a red count only for mentions and a dot for
// plain unread; every unread DM is attention, so the DM tab always counts.
const badge = (rail: Locator) => rail.locator('[data-badge-role]');
const dot = (rail: Locator) => rail.locator('[data-badge-role="neutral"]');

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
  await expect(home).not.toContainText('2');
  await page.getByRole('button', { name: 'Receive second DM' }).click();
  await expect(dm).toContainText('5');
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
});

test('Home aggregate shows a dot for plain unread and a red count for mentions', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&directSelected');
  const home = page.getByTestId('home-rail');
  const dm = page.getByTestId('dm-rail');
  await expect(dm.locator('button')).toHaveAttribute('aria-current', 'page');
  await page.getByRole('button', { name: 'Receive Home' }).click();
  await expect(dot(home)).toHaveCount(1);
  await expect(home).not.toContainText(/\d/);
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Mention Home' }).click();
  const mention = home.locator('[data-badge-role="mention"]');
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
  await expect(dot(home)).toHaveCount(1);
  await page
    .getByRole('button', { name: 'Direct messages', exact: true })
    .click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as Read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  await expect(dot(home)).toHaveCount(1);
  await expect(page.locator('body')).toHaveAttribute(
    'data-read-rooms',
    JSON.stringify(['!dm:example.test', '!other-dm:example.test'])
  );
  await page.getByRole('button', { name: 'Home', exact: true }).click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as Read', exact: true }).click();
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
