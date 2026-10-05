import { expect, test } from '@playwright/test';

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
  await page.getByRole('button', { name: 'Direct Messages', exact: true }).click();
  const destination = page.getByTestId('destination-rooms');
  await expect(destination).toContainText('!dm:example.test');
  await expect(destination).toContainText('!other-dm:example.test');
  await expect(destination).not.toContainText('!home:example.test');
  await expect(dm).toContainText('5');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(dm).toContainText('3');
  await page.getByRole('button', { name: 'Mark second DM read' }).click();
  await expect(dm).not.toContainText(/\d/);
});

test('Home aggregate continues updating and clearing while Direct Messages is selected', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&directSelected');
  const home = page.getByTestId('home-rail');
  const dm = page.getByTestId('dm-rail');
  await expect(dm.locator('button')).toHaveAttribute('aria-current', 'page');
  await page.getByRole('button', { name: 'Receive Home' }).click();
  await expect(home).toContainText('4');
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Mark Home read' }).click();
  await expect(home).not.toContainText(/\d/);
});

test('DM aggregate preserves mention-only, marked-unread, and mute behavior', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread');
  const dm = page.getByTestId('dm-rail');
  await page.getByRole('button', { name: 'Mark DM unread' }).click();
  await expect(dm).toContainText('1');
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Mention only DM' }).click();
  await expect(dm).toContainText('1');
  const mentionBadge = dm.locator('span').filter({ hasText: /^1$/ }).last();
  const mentionColor = await mentionBadge.evaluate(
    (el) => getComputedStyle(el.parentElement!).backgroundColor
  );
  await page.getByRole('button', { name: 'Mark DM read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await expect(dm).toContainText('2');
  const ordinaryColor = await dm
    .locator('span')
    .filter({ hasText: /^2$/ })
    .last()
    .evaluate((el) => getComputedStyle(el.parentElement!).backgroundColor);
  expect(mentionColor).not.toEqual(ordinaryColor);
  await page.getByRole('button', { name: 'Mute DM' }).click();
  await expect(dm).not.toContainText(/\d/);
});

test('rail Mark as Read clears exactly its badge and destination scope', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?navigationUnread&laggingMDirect');
  const home = page.getByTestId('home-rail');
  const dm = page.getByTestId('dm-rail');
  await page.getByRole('button', { name: 'Receive DM', exact: true }).click();
  await page.getByRole('button', { name: 'Receive Home', exact: true }).click();
  await expect(dm).toContainText('2');
  await expect(home).toContainText('4');
  await page
    .getByRole('button', { name: 'Direct Messages', exact: true })
    .click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as Read', exact: true }).click();
  await expect(dm).not.toContainText(/\d/);
  await expect(home).toContainText('4');
  await expect(page.locator('body')).toHaveAttribute(
    'data-read-rooms',
    JSON.stringify(['!dm:example.test', '!other-dm:example.test'])
  );
  await page.getByRole('button', { name: 'Home', exact: true }).click({ button: 'right' });
  await page.getByRole('button', { name: 'Mark as Read', exact: true }).click();
  await expect(home).not.toContainText(/\d/);
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
