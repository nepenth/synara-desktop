import { expect, type Locator, type Page } from '@playwright/test';

/**
 * The window widths every layout is checked at (L1): Mobile (380, 520, 700),
 * Tablet (900, 1100). Layout follows the window's width only, the same on
 * macOS, Windows and Linux, so one matrix covers all three. The main window
 * can be 360x480, so the smallest width is also checked 480px tall.
 */
export const VIEWPORT_WIDTHS = [380, 520, 700, 900, 1100] as const;
export const VIEWPORT_HEIGHT = 800;
export const SHORT_VIEWPORT = { width: 380, height: 480 } as const;

/** Above 750px the list sits beside the page; at or below it, one pane at a time. */
export const isMobileWidth = (width: number): boolean => width <= 750;

/**
 * The dev-only React Query devtools button floats over the bottom-right
 * corner (it is not in production builds); hide it so it cannot cover the
 * bottom bar's Account button.
 */
export const hideDevtools = (page: Page) =>
  page.addStyleTag({ content: '.tsqd-parent-container { display: none !important; }' });

/** Nothing is wider than the window: no horizontal page scroll. */
export const expectNoHorizontalOverflow = async (page: Page) => {
  const { scrollWidth, clientWidth } = await page.evaluate(() => ({
    scrollWidth: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
    clientWidth: document.documentElement.clientWidth,
  }));
  expect(scrollWidth, 'document is wider than the window').toBeLessThanOrEqual(clientWidth);
};

/** Visible and wholly inside the window (after scrolling to it, if `scroll`). */
export const expectInsideViewport = async (locator: Locator, { scroll = false } = {}) => {
  await expect(locator).toBeVisible();
  if (scroll) await locator.scrollIntoViewIfNeeded();
  // Measure where it settles, not mid-way through an opening animation.
  await locator.page().evaluate(() =>
    Promise.all(
      document
        .getAnimations()
        // Spinners run forever; only wait for animations that end.
        .filter((animation) => animation.effect?.getComputedTiming().endTime !== Infinity)
        .map((animation) => animation.finished.catch(() => undefined))
    )
  );
  const box = await locator.boundingBox();
  const size = locator.page().viewportSize();
  expect(box, 'element has a box').not.toBeNull();
  expect(size).not.toBeNull();
  const label = (await locator.getAttribute('aria-label')) ?? (await locator.textContent()) ?? '';
  expect(box!.x, `${label} starts left of the window`).toBeGreaterThanOrEqual(-1);
  expect(box!.y, `${label} starts above the window`).toBeGreaterThanOrEqual(-1);
  expect(box!.x + box!.width, `${label} ends right of the window`).toBeLessThanOrEqual(
    size!.width + 1
  );
  expect(box!.y + box!.height, `${label} ends below the window`).toBeLessThanOrEqual(
    size!.height + 1
  );
};
