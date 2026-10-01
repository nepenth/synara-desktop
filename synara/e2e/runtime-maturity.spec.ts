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
  await expect(page.getByTestId('pdf-status')).toHaveText('PDF rendered');
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

test('the production custom parser renders empty pre and code blocks without crashing', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html');
  const blocks = page.getByRole('region', { name: 'Code block regression' }).locator('pre');
  await expect(blocks).toHaveCount(3);
  for (let index = 0; index < 3; index += 1) {
    await expect(blocks.nth(index).getByText('Code', { exact: true })).toBeVisible();
    await expect(
      blocks.nth(index).getByRole('button', { name: 'Copy', exact: true })
    ).toBeVisible();
  }
  await expect(blocks.nth(0).locator('code')).toHaveCount(0);
  await expect(blocks.nth(1).locator('code')).toBeEmpty();
  await expect(blocks.nth(2).locator('code')).toHaveText('hello');
  expect(errors).toEqual([]);
});

test('nested CDATA retains literal text through the production custom parser', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html');
  const region = page.getByRole('region', { name: 'Nested XML regression' });
  await expect(region.locator('strong')).toHaveText('nested literal <kept>');
  await expect(region.locator('kept')).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('the production Slate composer edits, formats, splits paragraphs and restores history', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html');
  const editor = page
    .getByRole('region', { name: 'Message composer' })
    .locator('[data-slate-editor]');
  await editor.click();
  await page.keyboard.insertText('first');
  await page.keyboard.press('ControlOrMeta+b');
  await page.keyboard.insertText(' bold');
  await expect(editor.locator('strong')).toHaveText(' bold');
  await page.keyboard.press('ControlOrMeta+b');
  await page.keyboard.press('Enter');
  await page.keyboard.insertText('second');
  await expect(editor).toContainText('second');
  await expect.poll(() => page.getByTestId('composer-state').textContent()).toContain('second');
  await page.keyboard.press('ControlOrMeta+z');
  await expect(editor).not.toContainText('second');
  await page.keyboard.press('ControlOrMeta+Shift+z');
  await expect(editor).toContainText('second');
  expect(errors).toEqual([]);
});
