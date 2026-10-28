import { expect, test } from '@playwright/test';

// In the composer the picker sits in a popout that sizes to its content; the
// search field once collapsed to a few pixels there.
test('the GIF search field fills the picker row inside the composer popout', async ({ page }) => {
  await page.goto('/e2e/room-list-harness/index.html?gifPicker&popout');
  const search = page.getByRole('textbox', { name: 'Search GIFs' });
  await expect(search).toBeVisible();
  const field = await search.boundingBox();
  const button = await page.getByRole('button', { name: 'Search GIFs' }).boundingBox();
  expect(field?.width ?? 0).toBeGreaterThan(250);
  expect(button?.width ?? 0).toBeGreaterThan(24);
});
