import { expect, test, type Worker } from '@playwright/test';

for (const readiness of ['idle', 'offline', 'failed', 'terminated']) {
  test(`the production Retry button recovers ${readiness} native sync and awaits one owner request`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.route('**/mock-homeserver/_matrix/client/versions', (route) =>
      route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } })
    );
    await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
    await page.clock.install();
    await page.goto(`/e2e/runtime-maturity-harness/index.html?sync-recovery=${readiness}`);
    await expect(page.getByText('Heating up', { exact: true })).toBeVisible();
    await expect(
      page.getByText(
        // A terminated SyncService maps to ERROR like failed (shared
        // desktop/iOS readiness table), so both read as retrying.
        readiness === 'failed' || readiness === 'terminated'
          ? 'Sync is retrying'
          : readiness === 'offline'
            ? 'Reconnecting'
            : 'Sync is stopped',
        { exact: true }
      )
    ).toBeVisible();
    await page.clock.fastForward(30_001);
    await expect(page.getByText('Sync is taking longer than expected.')).toBeVisible();
    const retry = page.getByRole('button', { name: 'Retry', exact: true });
    await retry.click();
    await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
    await expect(retry).toBeDisabled();
    await retry.dispatchEvent('click');
    await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
    await expect(page.getByTestId('native-readiness')).toHaveText(readiness);
    await expect(page.getByText('Native client ready')).toHaveCount(0);
    await page.getByRole('button', { name: 'Complete native recovery' }).click();
    await expect(page.getByTestId('native-readiness')).toHaveText('running');
    await expect(page.getByText('Native client ready')).toBeVisible();
    await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
    await expect(page.getByTestId('native-logout-count')).toHaveText('0');
    expect(errors).toEqual([]);
  });
}

test('a rejected native recovery stays visible and allows a later explicit Retry without signing out', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/mock-homeserver/_matrix/client/versions', (route) =>
    route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } })
  );
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  await page.clock.install();
  await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=failed');
  await expect(page.getByText('Sync is retrying', { exact: true })).toBeVisible();
  await page.clock.fastForward(30_001);
  const retry = page.getByRole('button', { name: 'Retry', exact: true });
  await retry.click();
  await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
  await page.getByRole('button', { name: 'Fail native recovery' }).click();
  await expect(page.getByRole('alert')).toHaveText(
    'Could not restart sync. You can retry or reload the application.'
  );
  await expect(retry).toBeEnabled();
  await expect(page.getByTestId('native-readiness')).toHaveText('failed');
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  await retry.click();
  await expect(page.getByTestId('native-recovery-count')).toHaveText('2');
  await page.getByRole('button', { name: 'Complete native recovery' }).click();
  await expect(page.getByText('Native client ready')).toBeVisible();
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  expect(errors).toEqual([]);
});

test('an acknowledged restart keeps recovery controls until native sync is actually ready', async ({
  page,
}) => {
  await page.route('**/mock-homeserver/_matrix/client/versions', (route) =>
    route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } })
  );
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  await page.clock.install();
  await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=offline');
  await expect(page.getByText('Reconnecting', { exact: true })).toBeVisible();
  await page.clock.fastForward(30_001);
  const retry = page.getByRole('button', { name: 'Retry', exact: true });
  await retry.click();
  await page.getByRole('button', { name: 'Acknowledge recovery without readiness' }).click();
  await expect(page.getByTestId('native-readiness')).toHaveText('offline');
  await expect(page.getByText('Sync is taking longer than expected.')).toBeVisible();
  await expect(retry).toBeEnabled();
  await expect(page.getByText('Native client ready')).toHaveCount(0);
  await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  await retry.click();
  await page.getByRole('button', { name: 'Complete native recovery' }).click();
  await expect(page.getByText('Native client ready')).toBeVisible();
});

test('a failed native restore stays retryable across repeated failures without signing out', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/mock-homeserver/_matrix/client/versions', (route) =>
    route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } })
  );
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  await page.clock.install();
  await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=offline&restore-error');
  await expect(page.getByText('Failed to load. Controlled native restore failure')).toBeVisible();
  await expect(page.getByTestId('native-restore-count')).toHaveText('1');
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByTestId('native-restore-count')).toHaveText('2');
  await expect(page.getByText('Failed to load. Controlled native restore failure')).toBeVisible();
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByTestId('native-restore-count')).toHaveText('3');
  await expect(page.getByText('Reconnecting', { exact: true })).toBeVisible();
  await page.clock.fastForward(30_001);
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
  await page.getByRole('button', { name: 'Complete native recovery' }).click();
  await expect(page.getByText('Native client ready')).toBeVisible();
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  expect(errors).toEqual([]);
});

test('Retry clears a renderer startup read failure through native recovery and fresh hydration', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/mock-homeserver/_matrix/client/versions', (route) =>
    route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } })
  );
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=offline&startup-error');
  await expect(page.getByText('Failed to start. Controlled startup read failure')).toBeVisible();
  const retry = page.getByRole('button', { name: 'Retry', exact: true });
  await retry.click();
  await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
  await expect(retry).toBeDisabled();
  await page.getByRole('button', { name: 'Complete native recovery' }).click();
  await expect(page.getByText('Native client ready')).toBeVisible();
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  expect(errors).toEqual([]);
});

test('an established client requests native recovery on network return and contains a rejected wake', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/mock-homeserver/_matrix/client/versions', (route) =>
    route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } })
  );
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  await page.clock.install();
  await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=running');
  await expect(page.getByText('Native client ready')).toBeVisible();
  await page.getByRole('button', { name: 'Lose native connection' }).click();
  await page.clock.runFor(6_000);
  await expect(page.getByText('Connection Lost! Reconnecting...', { exact: true })).toBeVisible();
  await page.evaluate(() => window.dispatchEvent(new Event('online')));
  await page.clock.runFor(1);
  await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
  await page.getByRole('button', { name: 'Fail native recovery' }).click();
  await expect(page.getByTestId('native-readiness')).toHaveText('offline');
  await page.clock.fastForward(8_001);
  await page.evaluate(() => window.dispatchEvent(new Event('online')));
  await page.clock.runFor(1);
  await expect(page.getByTestId('native-recovery-count')).toHaveText('2');
  await page.getByRole('button', { name: 'Complete native recovery' }).click();
  await expect(page.getByText('Connected', { exact: true })).toBeVisible();
  await expect(page.getByText('Native client ready')).toBeVisible();
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  expect(errors).toEqual([]);
});

for (const metadataFailure of ['failed', 'hanging']) {
  test(`a ${metadataFailure} renderer versions request cannot block a restored native client`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    let releaseRequest: (() => void) | undefined;
    const hold = new Promise<void>((resolve) => {
      releaseRequest = resolve;
    });
    await page.route('**/mock-homeserver/_matrix/client/versions', async (route) => {
      if (metadataFailure === 'hanging') await hold;
      await route.abort('failed');
    });
    await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
    try {
      await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=running');
      await expect(page.getByText('Native client ready')).toBeVisible();
      await expect(page.getByTestId('native-readiness')).toHaveText('running');
      await expect(
        page.getByText('Unable to connect to the homeserver.', { exact: false })
      ).toHaveCount(0);
      await expect(page.getByTestId('native-recovery-count')).toHaveText('0');
      await expect(page.getByTestId('native-logout-count')).toHaveText('0');
      expect(errors).toEqual([]);
    } finally {
      releaseRequest?.();
    }
  });
}

test('failed renderer metadata leaves the real native sync recovery controls reachable', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.route('**/mock-homeserver/_matrix/client/versions', (route) => route.abort('failed'));
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  await page.clock.install();
  await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=offline');
  await expect(page.getByText('Reconnecting', { exact: true })).toBeVisible();
  await page.clock.fastForward(30_001);
  await expect(page.getByText('Sync is taking longer than expected.')).toBeVisible();
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByTestId('native-recovery-count')).toHaveText('1');
  await page.getByRole('button', { name: 'Complete native recovery' }).click();
  await expect(page.getByText('Native client ready')).toBeVisible();
  await expect(page.getByTestId('native-logout-count')).toHaveText('0');
  expect(errors).toEqual([]);
});

test('optional versions metadata updates capabilities without blocking or remounting native readiness', async ({
  page,
}) => {
  let releaseRequest: (() => void) | undefined;
  const hold = new Promise<void>((resolve) => {
    releaseRequest = resolve;
  });
  await page.route('**/mock-homeserver/_matrix/client/versions', async (route) => {
    await hold;
    await route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } });
  });
  await page.route('**/.well-known/matrix/client', (route) => route.fulfill({ json: {} }));
  try {
    await page.goto('/e2e/runtime-maturity-harness/index.html?sync-recovery=running');
    await expect(page.getByText('Native client ready')).toBeVisible();
    await expect(page.getByTestId('server-versions')).toHaveText('');
    releaseRequest?.();
    await expect(page.getByTestId('server-versions')).toHaveText('v1.11');
    await expect(page.getByText('Native client ready')).toBeVisible();
    await expect(page.getByTestId('native-restore-count')).toHaveText('1');
    await expect(page.getByTestId('native-recovery-count')).toHaveText('0');
  } finally {
    releaseRequest?.();
  }
});

test('the default pre-login metadata loader still blocks and retries failed server validation', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  let attempts = 0;
  let releaseRequest: (() => void) | undefined;
  const hold = new Promise<void>((resolve) => {
    releaseRequest = resolve;
  });
  await page.route('**/mock-homeserver/_matrix/client/versions', async (route) => {
    attempts += 1;
    if (attempts === 1) {
      await hold;
      await route.abort('failed');
    } else {
      await route.fulfill({ json: { versions: ['v1.11'], unstable_features: {} } });
    }
  });
  try {
    await page.goto(
      '/e2e/runtime-maturity-harness/index.html?sync-recovery=idle&pre-login-versions'
    );
    await expect(page.getByText('Checking login server')).toBeVisible();
    await expect(page.getByText('Login server validated:', { exact: false })).toHaveCount(0);
    releaseRequest?.();
    await page.getByRole('button', { name: 'Retry login server check' }).click();
    await expect(page.getByText('Login server validated: v1.11')).toBeVisible();
    expect(attempts).toBe(2);
    expect(errors).toEqual([]);
  } finally {
    releaseRequest?.();
  }
});

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
  // Slate picks the redo chord from the user agent (Cmd+Shift+Z on Apple,
  // Ctrl+Shift+Z elsewhere). Playwright's WebKit reports a Mac user agent on a
  // Linux host, so a host-derived ControlOrMeta chord would never redo there.
  const appleUserAgent = await page.evaluate(() => /Mac OS X/.test(navigator.userAgent));
  await page.keyboard.press(appleUserAgent ? 'Meta+Shift+z' : 'Control+Shift+z');
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
  await expect(page.getByTestId('pdf-status')).toHaveText('PDF rendered');
  await expect(page.getByTestId('pdf-canvas')).toBeVisible();
  // Cause a real RenderTask to reject inside CanvasGraphics on the next page.
  // The actual viewer, PDF document, module worker and render owner remain unchanged.
  await page.evaluate(() => {
    const state = globalThis as typeof globalThis & { controlledPdfRenderFailures: number };
    state.controlledPdfRenderFailures = 0;
    const standaloneCanvas = document.querySelector<HTMLCanvasElement>(
      '[data-testid="pdf-canvas"]'
    );
    if (!standaloneCanvas) throw new Error('Standalone PDF must complete before fault injection');
    const originalFill = CanvasRenderingContext2D.prototype.fill;
    CanvasRenderingContext2D.prototype.fill = function (
      this: CanvasRenderingContext2D,
      ...args: unknown[]
    ) {
      // The independent harness render can never satisfy the viewer retry oracle.
      if (this.canvas === standaloneCanvas) {
        Reflect.apply(originalFill, this, args);
        return;
      }
      state.controlledPdfRenderFailures += 1;
      throw new Error('Controlled next-page paint failure');
    };
  });
  await viewer.getByText('Next', { exact: true }).click();
  await expect(viewer.getByText('2/2', { exact: true })).toBeVisible();
  await expect(viewer.getByText('Failed to load PDF', { exact: true })).toBeVisible();
  await expect(viewer.locator('canvas')).toHaveCount(0);
  const failuresBeforeRetry = await page.evaluate(
    () =>
      (globalThis as typeof globalThis & { controlledPdfRenderFailures: number })
        .controlledPdfRenderFailures
  );
  expect(failuresBeforeRetry).toBeGreaterThan(0);
  await viewer.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (globalThis as typeof globalThis & { controlledPdfRenderFailures: number })
            .controlledPdfRenderFailures
      )
    )
    .toBeGreaterThan(failuresBeforeRetry);
  await expect(viewer.getByText('Failed to load PDF', { exact: true })).toBeVisible();
  await expect(viewer.locator('canvas')).toHaveCount(0);
  expect(errors).toEqual([]);
});
