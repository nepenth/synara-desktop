import { expect, test, type Worker } from '@playwright/test';

test('the typed update hook, dialog focus trap and PDF worker run without Node polyfills', async ({
  page,
  browserName,
}) => {
  const errors: string[] = [];
  const workers: string[] = [];
  const warnings: string[] = [];
  page.on('worker', (worker) => workers.push(worker.url()));
  page.on('console', (message) => {
    if (message.type() === 'warning') warnings.push(message.text());
  });
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Update 0', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Update 1', exact: true })).toBeVisible();
  const opener = page.getByRole('button', { name: 'Open dialog' });
  // Keyboard activation gives the trap a real return-focus target in Safari,
  // where clicking a button does not focus it by default.
  await opener.focus();
  await opener.press('Enter');
  const close = page.getByRole('button', { name: 'Close', exact: true });
  await expect(close).toBeFocused();
  // Safari's native default uses Option-Tab to traverse button controls.
  const nextControl = browserName === 'webkit' ? 'Alt+Tab' : 'Tab';
  await page.keyboard.press(nextControl);
  await expect(page.getByRole('button', { name: 'Second' })).toBeFocused();
  await page.keyboard.press(nextControl);
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
  expect(workers.some((url) => url.includes('pdfjs-worker'))).toBe(true);
  expect(warnings.some((warning) => warning.includes('fake worker'))).toBe(false);
});

test('the production custom parser renders empty pre and code blocks without crashing', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
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

// Model builtins missing from older webviews in both independent realms.
// The application uses upstream PDF.js compatibility bundles, not test polyfills.
const removeNewPdfBuiltins = () => {
  const slots: [object, string][] = [
    [Promise, 'withResolvers'],
    [Promise, 'try'],
    [ArrayBuffer.prototype, 'transferToFixedLength'],
    [Uint8Array.prototype, 'toBase64'],
    [Uint8Array, 'fromBase64'],
  ];
  for (const [owner, name] of slots) Reflect.deleteProperty(owner, name);
  (
    globalThis as typeof globalThis & { pdfBuiltinsMissingBeforeLoad: boolean }
  ).pdfBuiltinsMissingBeforeLoad = slots.every(([owner, name]) => !(name in owner));
};

test('the compatibility PDF API and real worker render when newer builtins are absent', async ({
  page,
}) => {
  const errors: string[] = [];
  const warnings: string[] = [];
  const workers: Worker[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('worker', (worker) => workers.push(worker));
  page.on('console', (message) => {
    if (message.type() === 'warning') warnings.push(message.text());
  });
  await page.addInitScript(removeNewPdfBuiltins);
  // A test prelude removes builtins before importing the unchanged production
  // worker entry. Static imports in that entry then initialize the upstream shim.
  await page.route('**/pdfjs-worker.ts?*', async (route) => {
    const originalUrl = new URL(route.request().url());
    if (
      !originalUrl.searchParams.has('worker_file') ||
      originalUrl.searchParams.has('pdfCompatibilityOriginal')
    ) {
      await route.continue();
      return;
    }
    originalUrl.searchParams.set('pdfCompatibilityOriginal', '1');
    await route.fulfill({
      contentType: 'text/javascript',
      body: `(${removeNewPdfBuiltins.toString()})();\nawait import(${JSON.stringify(
        originalUrl.href
      )});`,
    });
  });
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
  await expect(page.getByTestId('pdf-status')).toHaveText('PDF rendered');
  await expect
    .poll(() =>
      page
        .getByTestId('pdf-canvas')
        .evaluate((canvas) =>
          Array.from(
            (canvas as HTMLCanvasElement).getContext('2d')!.getImageData(50, 50, 1, 1).data
          )
        )
    )
    .toEqual([255, 0, 0, 255]);
  const probe = () => ({
    missingBeforeLoad: (
      globalThis as typeof globalThis & {
        pdfBuiltinsMissingBeforeLoad: boolean;
      }
    ).pdfBuiltinsMissingBeforeLoad,
    withResolversAfterLoad: typeof (Promise as unknown as Record<string, unknown>).withResolvers,
    tryAfterLoad: typeof (Promise as unknown as Record<string, unknown>).try,
  });
  const expected = {
    missingBeforeLoad: true,
    withResolversAfterLoad: 'function',
    tryAfterLoad: 'function',
  };
  expect(await page.evaluate(probe)).toEqual(expected);
  const pdfWorker = workers.find((worker) => worker.url().includes('pdfjs-worker'));
  expect(pdfWorker, 'A real PDF worker must render the document').toBeDefined();
  expect(await pdfWorker!.evaluate(probe)).toEqual(expected);
  expect(
    await pdfWorker!.evaluate(() => {
      const transfer = (ArrayBuffer.prototype as unknown as Record<string, unknown>)
        .transferToFixedLength;
      return typeof transfer;
    })
  ).toBe('function');
  expect(errors).toEqual([]);
  expect(warnings.some((warning) => warning.includes('fake worker'))).toBe(false);
});

test('nested CDATA retains literal text through the production custom parser', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
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
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
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

test('PDF render status waits for completion and surfaces RenderTask failure through the production owner', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
  const lifecycle = page.getByRole('region', { name: 'PDF render lifecycle' });
  await lifecycle.getByRole('button', { name: 'Start PDF render', exact: true }).click();
  await expect(lifecycle.getByTestId('pdf-render-state')).toHaveText('loading');
  await expect(lifecycle.locator('canvas')).toHaveCount(0);
  await lifecycle.getByRole('button', { name: 'Finish PDF render', exact: true }).click();
  await expect(lifecycle.getByTestId('pdf-render-state')).toHaveText('success');
  await expect(lifecycle.locator('canvas')).toHaveCount(1);
  await lifecycle.getByRole('button', { name: 'Start PDF render', exact: true }).click();
  await expect(lifecycle.getByTestId('pdf-render-state')).toHaveText('loading');
  await expect(lifecycle.locator('canvas')).toHaveCount(0);
  await lifecycle.getByRole('button', { name: 'Fail PDF render', exact: true }).click();
  await expect(lifecycle.getByTestId('pdf-render-state')).toHaveText('error');
  await expect(lifecycle.getByRole('alert')).toContainText('Controlled render failure');
  await expect(lifecycle.locator('canvas')).toHaveCount(0);
  await lifecycle.getByRole('button', { name: 'Retry PDF render', exact: true }).click();
  await expect(lifecycle.getByTestId('pdf-render-state')).toHaveText('loading');
  await lifecycle.getByRole('button', { name: 'Fail PDF render', exact: true }).click();
  await expect(lifecycle.getByTestId('pdf-render-state')).toHaveText('error');
  await expect(lifecycle.getByRole('alert')).toContainText('Controlled render failure');
  await expect(lifecycle.locator('canvas')).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('the actual PDF viewer removes the old page and exposes render and retry failures', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/e2e/runtime-maturity-harness/index.html', { waitUntil: 'domcontentloaded' });
  const viewer = page.getByRole('region', { name: 'PDF viewer regression' });
  await expect(viewer.locator('canvas')).toHaveCount(1);
  await expect(viewer.getByText('1/2', { exact: true })).toBeVisible();
  // Cause a real RenderTask to reject inside CanvasGraphics on the next page.
  // The actual viewer, PDF document, module worker and render owner remain unchanged.
  await page.evaluate(() => {
    const state = globalThis as typeof globalThis & { controlledPdfRenderFailures: number };
    state.controlledPdfRenderFailures = 0;
    CanvasRenderingContext2D.prototype.fill = () => {
      state.controlledPdfRenderFailures += 1;
      throw new Error('Controlled next-page paint failure');
    };
  });
  await viewer.getByText('Next', { exact: true }).click();
  await expect(viewer.getByText('2/2', { exact: true })).toBeVisible();
  await expect(viewer.getByText('Failed to load PDF', { exact: true })).toBeVisible();
  await expect(viewer.locator('canvas')).toHaveCount(0);
  await viewer.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (globalThis as typeof globalThis & { controlledPdfRenderFailures: number })
            .controlledPdfRenderFailures
      )
    )
    .toBeGreaterThanOrEqual(2);
  await expect(viewer.getByText('Failed to load PDF', { exact: true })).toBeVisible();
  await expect(viewer.locator('canvas')).toHaveCount(0);
  expect(errors).toEqual([]);
});
