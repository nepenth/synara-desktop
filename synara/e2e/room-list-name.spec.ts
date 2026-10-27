import { expect, test } from '@playwright/test';

test('a mounted room row follows summary name changes', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?roomName');
  const row = page.getByTestId('room-name-row');
  await expect(row).toHaveText('# Empty Room');
  await page.getByRole('button', { name: 'Name arrives' }).click();
  await expect(row).toHaveText('# Project Room');
  await page.getByRole('button', { name: 'Rename' }).click();
  await expect(row).toHaveText('# Renamed Room');
});
