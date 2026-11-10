import { expect, test } from '@playwright/test';
import {
  VIEWPORT_HEIGHT,
  VIEWPORT_WIDTHS,
  expectInsideViewport,
  expectNoHorizontalOverflow,
} from './viewportMatrix';

// L1: the Approvals page with two pending requests at every matrix width
// (`?narrow`: the page alone, as Mobile shows it beside no list). The cards'
// decision buttons wrap rather than run off the window.
const route = '/e2e/approvals-harness/index.html?narrow';

for (const width of VIEWPORT_WIDTHS) {
  test.describe(`${width}px wide`, () => {
    test.use({ viewport: { width, height: VIEWPORT_HEIGHT } });

    test('pending requests fit, with every decision button reachable', async ({ page }) => {
      await page.goto(route);
      const card = page.getByRole('region', { name: /Approval from Publishing/ });
      await expect(card).toBeVisible();
      await expectNoHorizontalOverflow(page);
      await expectInsideViewport(page.getByRole('heading', { name: 'Approvals', exact: true }));
      for (const name of ['Approve once', 'Approve always', 'Deny']) {
        await expectInsideViewport(card.getByRole('button', { name, exact: true }), {
          scroll: true,
        });
      }
      await expectInsideViewport(page.getByRole('button', { name: /^Recent/ }), { scroll: true });
    });
  });
}
