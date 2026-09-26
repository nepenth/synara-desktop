import { expect, test } from '@playwright/test';

test('returning to a room paints the same protocol avatar without a byte download', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/avatar.html');
  const row = page.getByTestId('avatar-row');
  await expect(row).toHaveAttribute(
    'data-current-src',
    'synara-media://localhost/mxc%3A%2F%2Fexample.test%2Favatar-a'
  );
  const src = await row.getAttribute('data-current-src');
  for (let index = 0; index < 5; index += 1) {
    await page.getByRole('button', { name: 'Switch room', exact: true }).click();
    await expect(row).toHaveCount(0);
    await page.getByRole('button', { name: 'Switch room', exact: true }).click();
    await expect(row).toHaveAttribute('data-first-src', src!);
    await expect(row).toHaveAttribute('data-current-src', src!);
  }
});

test('an edited avatar changes the protocol URL and does not reuse the previous source', async ({
  page,
}) => {
  await page.goto('/e2e/room-list-harness/avatar.html');
  const row = page.getByTestId('avatar-row');
  const oldSrc = await row.getAttribute('data-current-src');
  await page.getByRole('button', { name: 'Change avatar', exact: true }).click();
  await expect(row).toHaveAttribute(
    'data-current-src',
    'synara-media://localhost/mxc%3A%2F%2Fexample.test%2Favatar-new'
  );
  await expect(row).not.toHaveAttribute('data-current-src', oldSrc!);
});
