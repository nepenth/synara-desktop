import { expect, test } from '@playwright/test';

test('the typed update hook, dialog focus trap and PDF worker run without Node polyfills', async ({
  page,
}) => {
  const errors: string[] = [];
  const workers: string[] = [];
  const warnings: string[] = [];
  page.on('worker', (worker) => workers.push(worker.url()));
  page.on('console', (message) => {
    if (message.type() === 'warning') warnings.push(message.text());
  });
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html');
  await page.getByRole('button', { name: 'Update 0', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Update 1', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Open dialog' }).click();
  const close = page.getByRole('button', { name: 'Close', exact: true });
  await expect(close).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(page.getByRole('button', { name: 'Second' })).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(close).toBeFocused();
  await close.click();
  await expect(page.getByRole('button', { name: 'Open dialog' })).toBeFocused();
  await expect(page.locator('output')).toHaveText('PDF rendered');
  await expect(page.getByTestId('pdf-canvas')).toBeVisible();
  await expect
    .poll(() =>
      page.getByTestId('pdf-canvas').evaluate((canvas) => {
        const pixel = (canvas as HTMLCanvasElement)
          .getContext('2d')
          ?.getImageData(50, 50, 1, 1).data;
        return pixel && Array.from(pixel);
      })
    )
    .toEqual([255, 0, 0, 255]);
  expect(errors).toEqual([]);
  expect(workers.some((url) => url.includes('/pdf.worker.min.js'))).toBe(true);
  expect(warnings.some((warning) => warning.includes('fake worker'))).toBe(false);
});
