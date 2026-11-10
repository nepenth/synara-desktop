import { expect, test } from '@playwright/test';
import {
  SHORT_VIEWPORT,
  VIEWPORT_HEIGHT,
  VIEWPORT_WIDTHS,
  expectInsideViewport,
  expectNoHorizontalOverflow,
  hideDevtools,
} from './viewportMatrix';

// L1: sign-in at every matrix width, and in the smallest window (380x480),
// where the form scrolls but every field and Sign in stay reachable.
const HARNESS = '/e2e/login-harness/index.html';

const sizes = [
  ...VIEWPORT_WIDTHS.map((width) => ({ width, height: VIEWPORT_HEIGHT })),
  SHORT_VIEWPORT,
];

for (const viewport of sizes) {
  test.describe(`${viewport.width}x${viewport.height}`, () => {
    test.use({ viewport });

    test('sign-in fits: no sideways scroll, fields and Sign in reachable', async ({ page }) => {
      await page.goto(HARNESS);
      await hideDevtools(page);
      await expect(page.getByText('Connected', { exact: false })).toBeVisible();
      await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
      await expectNoHorizontalOverflow(page);
      for (const field of ['usernameInput', 'passwordInput']) {
        await expectInsideViewport(page.locator(`input[name="${field}"]`), { scroll: true });
      }
      await expectInsideViewport(page.getByRole('button', { name: 'Sign in' }), { scroll: true });
      await expectNoHorizontalOverflow(page);
    });
  });
}
