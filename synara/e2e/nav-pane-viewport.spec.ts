import { expect, test, type Page } from '@playwright/test';
import {
  SHORT_VIEWPORT,
  VIEWPORT_HEIGHT,
  VIEWPORT_WIDTHS,
  expectInsideViewport,
  expectNoHorizontalOverflow,
  isMobileWidth,
} from './viewportMatrix';

// The list column (header, space switcher with twelve spaces and a folder,
// bottom bar) with the layout following the window's width (`?viewport`).
// L1: everything fits at every matrix width. L2: at Tablet width a thread
// hides the list while it is open, and the user's own show or hide wins.
const open = (page: Page) => page.goto('/e2e/room-list-harness/index.html?navPane&viewport');
const pane = (page: Page) => page.locator('#synara-room-list-pane');
const trigger = (page: Page) => page.getByTestId('space-switcher-trigger');
const switcher = (page: Page) => page.getByTestId('space-switcher');
const hidden = (page: Page) => pane(page).getAttribute('data-collapsed');

for (const viewport of [
  ...VIEWPORT_WIDTHS.map((width) => ({ width, height: VIEWPORT_HEIGHT })),
  SHORT_VIEWPORT,
]) {
  test.describe(`${viewport.width}x${viewport.height}`, () => {
    test.use({ viewport });

    test('the bar and the space switcher fit the window', async ({ page }) => {
      await open(page);
      for (const id of ['home', 'activity', 'approvals', 'account']) {
        await expectInsideViewport(page.getByTestId(`nav-bar-${id}`));
      }
      // Mobile: the list fills the window, alone, with labels in the bar.
      if (isMobileWidth(viewport.width)) {
        await expect(page.getByTestId('current-route')).toHaveCount(0);
        await expect(page.getByTestId('nav-bottom-bar')).toContainText('Approvals');
      }
      await trigger(page).click();
      await expectInsideViewport(switcher(page));
      await expectInsideViewport(switcher(page).getByRole('searchbox', { name: 'Find a space' }));
      // The last row and Manage spaces are reachable (the list scrolls).
      await expectInsideViewport(
        switcher(page).getByRole('button', { name: 'Zeta', exact: true }),
        {
          scroll: true,
        }
      );
      await expectInsideViewport(switcher(page).getByRole('button', { name: 'Manage spaces' }), {
        scroll: true,
      });
      await expectNoHorizontalOverflow(page);
    });
  });
}

test.describe('Tablet: a thread hides the list while it is open', () => {
  test.use({ viewport: { width: 900, height: VIEWPORT_HEIGHT } });

  test('hidden while open, back when it closes', async ({ page }) => {
    await open(page);
    expect(await hidden(page)).toBeNull();
    await page.getByRole('button', { name: 'Open thread' }).click();
    await expect(page.getByTestId('thread-pane-open')).toBeVisible();
    await expect(pane(page)).toHaveAttribute('data-collapsed', 'true');
    // The page header offers the list back.
    await expect(page.locator('#synara-room-list-toggle')).toBeVisible();
    await page.getByRole('button', { name: 'Close thread' }).click();
    await expect(pane(page)).not.toHaveAttribute('data-collapsed', 'true');
    // Never saved: a reload shows the list.
    await open(page);
    expect(await hidden(page)).toBeNull();
  });

  test('showing the list while a thread is open is the user’s choice and stays', async ({
    page,
  }) => {
    await open(page);
    await page.getByRole('button', { name: 'Open thread' }).click();
    await expect(pane(page)).toHaveAttribute('data-collapsed', 'true');
    await page.locator('#synara-room-list-toggle').click();
    await expect(pane(page)).not.toHaveAttribute('data-collapsed', 'true');
    await page.getByRole('button', { name: 'Close thread' }).click();
    await expect(pane(page)).not.toHaveAttribute('data-collapsed', 'true');
  });

  test('a list the user hid stays hidden after the thread closes', async ({ page }) => {
    await open(page);
    await pane(page).getByTestId('room-list-hide').click();
    await expect(pane(page)).toHaveAttribute('data-collapsed', 'true');
    await page.getByRole('button', { name: 'Open thread' }).click();
    await page.getByRole('button', { name: 'Close thread' }).click();
    await expect(pane(page)).toHaveAttribute('data-collapsed', 'true');
  });
});

test.describe('Desktop', () => {
  test.use({ viewport: { width: 1280, height: VIEWPORT_HEIGHT } });

  test('a thread leaves the list alone', async ({ page }) => {
    await open(page);
    await page.getByRole('button', { name: 'Open thread' }).click();
    await expect(page.getByTestId('thread-pane-open')).toBeVisible();
    expect(await hidden(page)).toBeNull();
  });
});
