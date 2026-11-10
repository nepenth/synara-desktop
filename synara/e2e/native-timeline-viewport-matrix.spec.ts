import { expect, test } from '@playwright/test';
import {
  VIEWPORT_HEIGHT,
  VIEWPORT_WIDTHS,
  expectInsideViewport,
  expectNoHorizontalOverflow,
  isMobileWidth,
} from './viewportMatrix';

// L1: a thread at every matrix width (`?fluid`: the room and the pane share
// the window, as in the app). Mobile has no room for a side pane, so the
// thread opens in place of the room with "Back to room", as the app does
// there; wider windows open it beside the room.
for (const width of VIEWPORT_WIDTHS) {
  const mobile = isMobileWidth(width);

  test.describe(`${width}px wide`, () => {
    test.use({ viewport: { width, height: VIEWPORT_HEIGHT } });

    test(`a thread opens ${mobile ? 'in place of the room' : 'beside the room'} and fits`, async ({
      page,
    }) => {
      await page.goto(
        `/e2e/native-timeline-harness/index.html?scenario=threads&fluid&threadDisplay=${
          mobile ? 'full' : 'side'
        }`
      );
      await expect(page.locator('[data-native-timeline-event-id]').first()).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await page
        .locator('#native-timeline')
        .getByRole('button', { name: /Thread · 7 replies/ })
        .click();

      if (mobile) {
        await expectInsideViewport(page.getByRole('button', { name: 'Back to room' }));
        await expect(page.locator('#native-timeline').getByText('Thread reply 7')).toBeVisible();
        await expectNoHorizontalOverflow(page);
        await page.getByRole('button', { name: 'Back to room' }).click();
        await expect(
          page.locator('#native-timeline').getByRole('button', { name: /Thread · 7 replies/ })
        ).toBeVisible();
        return;
      }
      const pane = page.locator('[data-thread-side-panel]');
      await expectInsideViewport(pane);
      await expectInsideViewport(pane.getByRole('button', { name: 'Close thread' }));
      await expect(pane.getByText('Thread reply 7')).toBeVisible();
      // The room keeps at least 40% of the width beside the pane.
      const room = (await page.locator('#native-timeline').boundingBox())!;
      expect(room.width).toBeGreaterThanOrEqual(width * 0.35);
      await expectNoHorizontalOverflow(page);
      await page.keyboard.press('Escape');
      await expect(pane).toHaveCount(0);
    });
  });
}
