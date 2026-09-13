import { expect, test } from '@playwright/test';

test('returning to a room paints a warm avatar immediately without new native IPC', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/avatar.html');
  const row = page.getByTestId('avatar-row');
  await expect(row).toHaveAttribute('data-current-src', /^blob:/);
  await expect(page.getByAltText('Sender avatar')).toBeVisible();
  await expect(page.getByTestId('downloads')).toHaveText('1');
  const src = await row.getAttribute('data-current-src');
  for (let index = 0; index < 5; index += 1) {
    await page.getByRole('button', { name: 'Switch room', exact: true }).click();
    await expect(row).toHaveCount(0);
    await page.getByRole('button', { name: 'Switch room', exact: true }).click();
    await expect(row).toHaveAttribute('data-first-src', src!);
    await expect(row).toHaveAttribute('data-current-src', src!);
    await expect(page.getByTestId('downloads')).toHaveText('1');
  }
});

test('avatar edits and account changes do not reuse the previous source', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/avatar.html');
  const row = page.getByTestId('avatar-row');
  await expect(row).toHaveAttribute('data-current-src', /^blob:/);
  const oldSrc = await row.getAttribute('data-current-src');
  await page.getByRole('button', { name: 'Change avatar', exact: true }).click();
  await expect(row).not.toHaveAttribute('data-current-src', oldSrc!);
  await expect(row).toHaveAttribute('data-current-src', /^blob:/);
  await expect(page.getByTestId('downloads')).toHaveText('2');
  const changedSrc = await row.getAttribute('data-current-src');
  await page.getByRole('button', { name: 'Switch account', exact: true }).click();
  await expect(row).not.toHaveAttribute('data-current-src', changedSrc!);
  await expect(row).toHaveAttribute('data-current-src', /^blob:/);
  await expect(page.getByTestId('downloads')).toHaveText('3');
});

test('returning at the idle cache limit retains the committed warm avatar', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/avatar.html?pressure');
  const row = page.getByTestId('avatar-row');
  await expect(row).toHaveAttribute('data-current-src', /^blob:/);
  const originalSrc = await row.getAttribute('data-current-src');
  await page.getByRole('button', { name: 'Switch room', exact: true }).click();
  await expect(page.getByTestId('cache-pressure')).toHaveText('Primed');
  await expect(row).toHaveAttribute('data-current-src', /^blob:/);
  await expect(page.getByTestId('downloads')).toHaveText('2');
  await page.getByRole('button', { name: 'Switch room', exact: true }).click();
  await expect(row).toHaveAttribute('data-first-src', originalSrc!);
  await expect(row).toHaveAttribute('data-current-src', originalSrc!);
  await expect(page.getByTestId('downloads')).toHaveText('2');
});

test('a failed mounted avatar owns the replacement lease after another avatar retries', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/avatar.html?recovery');
  await expect(page.locator('body')).toHaveAttribute('data-download-failed', 'true');
  await expect(page.getByTestId('downloads')).toHaveText('1');
  await expect(page.getByTestId('avatar-row')).toHaveAttribute('data-current-src', '');
  await page.getByRole('button', { name: 'Show retry avatar' }).click();
  const original = page.getByTestId('avatar-row').first();
  await expect(original).toHaveAttribute('data-current-src', /^blob:/);
  await expect(page.getByAltText('Sender avatar')).toHaveCount(2);
  const recoveredSrc = await original.getAttribute('data-current-src');
  await page.getByRole('button', { name: 'Hide retry avatar' }).click();
  await expect(page.getByTestId('avatar-row')).toHaveCount(1);
  // Give the last-release microtask (and any accidental retry) a browser frame.
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
  await expect(original).toHaveAttribute('data-current-src', recoveredSrc!);
  await expect(page.getByAltText('Sender avatar')).toBeVisible();
  await expect(page.getByTestId('downloads')).toHaveText('2');
});
