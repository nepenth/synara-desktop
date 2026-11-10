import { expect, test, type Page } from '@playwright/test';
import {
  SHORT_VIEWPORT,
  VIEWPORT_HEIGHT,
  VIEWPORT_WIDTHS,
  expectInsideViewport,
  expectNoHorizontalOverflow,
  hideDevtools,
  isMobileWidth,
} from './viewportMatrix';

// L1: the real app (router, list column, bottom bar, room header, settings,
// Approvals, Activity, Explore) at every matrix width. No horizontal page
// scroll, and the controls that get you around stay visible and inside the
// window. Mobile (<= 750px) shows one pane at a time: the list with its bar,
// or a page with a Back button.
const HARNESS = '/e2e/settings-harness/index.html';
const ROOM = '!review:example.test';
const roomPath = `#/home/${encodeURIComponent(ROOM)}/`;

const open = async (page: Page, hash: string, query = '?rooms=60') => {
  await page.goto(`${HARNESS}${query}${hash}`);
  await hideDevtools(page);
};
const bar = (page: Page) => page.getByTestId('nav-bottom-bar');
const barItems = ['home', 'activity', 'approvals', 'account'];
const roomRow = (page: Page) =>
  page.locator(`a[data-room-nav-link][data-room-nav-id="${ROOM}"]`).first();

for (const width of VIEWPORT_WIDTHS) {
  const mobile = isMobileWidth(width);

  test.describe(`${width}px wide`, () => {
    test.use({ viewport: { width, height: VIEWPORT_HEIGHT } });

    test('Home: the list, its header and the bottom bar fit', async ({ page }) => {
      await open(page, '#/home/');
      await expect(page.getByTestId('nav-search-field')).toBeVisible();
      await expectInsideViewport(page.getByTestId('space-switcher-trigger'));
      for (const id of barItems) await expectInsideViewport(page.getByTestId(`nav-bar-${id}`));
      // Labels under the icons on Mobile, where the column fills the window.
      if (mobile) await expect(bar(page)).toContainText('Activity');
      await expectNoHorizontalOverflow(page);
    });

    test('a room: header controls fit; on Mobile, Back returns to the list', async ({ page }) => {
      await open(page, '#/home/');
      await roomRow(page).click();
      const header = page.locator('header[data-tauri-drag-region]').filter({
        has: page.getByRole('heading', { name: 'Design review', level: 1 }),
      });
      await expect(header).toBeVisible();
      await expectInsideViewport(header.getByRole('heading', { level: 1 }));
      await expectInsideViewport(header.getByRole('button', { name: 'More Options' }));
      await expectInsideViewport(header.getByRole('button', { name: 'Call' }));
      // Narrow headers list the room's tools under More options.
      const search = header.getByRole('button', { name: 'Search', exact: true });
      if ((await search.count()) > 0) await expectInsideViewport(search);
      else {
        await header.getByRole('button', { name: 'More Options' }).click();
        await expectInsideViewport(page.getByRole('button', { name: 'Search', exact: true }));
        await page.keyboard.press('Escape');
      }
      await expectNoHorizontalOverflow(page);

      const back = header.getByRole('button', { name: 'Back' });
      if (mobile) {
        // One pane: the room alone, without the list or the bar.
        await expect(bar(page)).toHaveCount(0);
        await expectInsideViewport(back);
        await back.click();
        await expect(page.getByTestId('nav-search-field')).toBeVisible();
        await expectInsideViewport(page.getByTestId('nav-bar-home'));
      } else {
        await expect(back).toHaveCount(0);
        await expect(bar(page)).toBeVisible();
      }
    });

    test('Settings: the page list and a page fit, Close stays reachable', async ({ page }) => {
      await open(page, '#/settings/', '');
      const dialog = page.getByRole('dialog').first();
      await expect(dialog).toBeVisible();
      await expectInsideViewport(dialog);
      await expectInsideViewport(page.getByLabel('Search settings', { exact: true }));
      await page
        .getByRole('navigation', { name: 'Settings sections' })
        .getByRole('button', { name: 'Appearance' })
        .click();
      await expect(page.getByText('These settings apply to this device only.')).toBeVisible();
      if (mobile) await expectInsideViewport(page.getByRole('button', { name: 'Back' }));
      await expectInsideViewport(page.getByRole('button', { name: 'Close' }).first());
      await expectNoHorizontalOverflow(page);
    });

    test('Approvals, Activity and Explore fit', async ({ page }) => {
      await open(page, '#/approvals/');
      await expect(page.getByRole('heading', { name: 'Approvals', level: 1 })).toBeVisible();
      await expectInsideViewport(page.getByRole('searchbox', { name: 'Search approval requests' }));
      await expectInsideViewport(page.getByRole('button', { name: 'Refresh approval requests' }));
      if (mobile) await expectInsideViewport(page.getByRole('button', { name: 'Back' }));
      await expectNoHorizontalOverflow(page);

      await open(page, '#/inbox/notifications/');
      await expectInsideViewport(page.getByRole('button', { name: 'Highlighted' }));
      await expectInsideViewport(page.getByRole('button', { name: 'Mark visible read' }));
      await expectNoHorizontalOverflow(page);

      await open(page, '#/explore/');
      await expect(page.getByText('Featured').first()).toBeVisible();
      await expectNoHorizontalOverflow(page);
    });

    test('a space: the header mark stays a small avatar', async ({ page }) => {
      await open(page, `#/${encodeURIComponent('!studio:example.test')}/`, '?space&rooms=10');
      const trigger = page.getByTestId('space-switcher-trigger');
      await expectInsideViewport(trigger);
      const mark = await trigger.locator('span').first().boundingBox();
      expect(mark!.width).toBeLessThanOrEqual(32);
      expect(mark!.height).toBeLessThanOrEqual(32);
      for (const id of barItems) await expectInsideViewport(page.getByTestId(`nav-bar-${id}`));
      await expectNoHorizontalOverflow(page);
    });
  });
}

test.describe('the smallest window, 380x480', () => {
  test.use({ viewport: SHORT_VIEWPORT });

  test('the space switcher and Manage spaces fit', async ({ page }) => {
    await open(page, '#/home/', '?rooms=60&space');
    await page.getByTestId('space-switcher-trigger').click();
    const switcher = page.getByTestId('space-switcher');
    await expectInsideViewport(switcher);
    await switcher.getByRole('button', { name: 'Manage spaces' }).click();
    const manage = page.getByRole('dialog', { name: 'Manage spaces' });
    await expectInsideViewport(manage);
    await expectInsideViewport(manage.getByRole('button', { name: 'Close' }));
    await expectNoHorizontalOverflow(page);
  });

  test('the shortcut help sheet and search fit and close', async ({ page }) => {
    await open(page, '#/home/');
    await expect(page.getByTestId('nav-search-field')).toBeVisible();
    await page.keyboard.press('Control+/');
    const help = page.locator('[data-shortcut-help]');
    await expectInsideViewport(help);
    await expectInsideViewport(help.getByRole('button', { name: 'Close' }));
    await page.keyboard.press('Escape');
    await expect(help).toHaveCount(0);

    await page.keyboard.press('Control+k');
    const search = page.getByRole('combobox').or(page.getByRole('textbox', { name: /Search/ }));
    await expectInsideViewport(search.first());
    await expectNoHorizontalOverflow(page);
  });

  test('create room: the form fits and its button is reachable', async ({ page }) => {
    await open(page, '#/home/');
    await page.getByRole('button', { name: 'Add room' }).click();
    await page.getByRole('button', { name: 'Create room' }).first().click();
    await expectInsideViewport(page.getByRole('button', { name: 'Back' }));
    await expectInsideViewport(page.getByRole('button', { name: 'Create', exact: true }), {
      scroll: true,
    });
    await expectNoHorizontalOverflow(page);
  });

  test('a room: More options scrolls inside the window; the bar and account menu fit', async ({
    page,
  }) => {
    await open(page, roomPath);
    await page.getByRole('button', { name: 'More Options' }).click();
    const leave = page.getByRole('button', { name: 'Leave Room' });
    await expectInsideViewport(leave, { scroll: true });
    await page.keyboard.press('Escape');

    await page.getByRole('button', { name: 'Back' }).click();
    await page.getByTestId('nav-bar-account').click();
    await expectInsideViewport(page.getByRole('button', { name: /^Settings/ }));
    await expectInsideViewport(page.getByRole('button', { name: 'Log out' }));
  });
});

test.describe('Tablet width', () => {
  test.use({ viewport: { width: 900, height: VIEWPORT_HEIGHT } });

  test('members open over the room, inside it, and Esc closes them', async ({ page }) => {
    await open(page, roomPath);
    const room = page.locator('main').filter({ has: page.getByRole('heading', { level: 1 }) });
    const roomBox = (await room.first().boundingBox())!;
    const header = page.locator('header[data-tauri-drag-region]').last();
    let show = header.getByRole('button', { name: 'Show Members' });
    if ((await show.count()) === 0) {
      await header.getByRole('button', { name: 'More Options' }).click();
      show = page.getByRole('button', { name: 'Show Members' });
    }
    await show.click();
    const overlay = page.locator('[data-room-panel-overlay="members"]');
    await expect(overlay).toBeVisible();
    const box = (await overlay.boundingBox())!;
    // Over the room's right edge, not beside it: the room keeps its width.
    expect(Math.round(box.x + box.width)).toBe(Math.round(roomBox.x + roomBox.width));
    expect((await room.first().boundingBox())!.width).toBe(roomBox.width);
    await expectNoHorizontalOverflow(page);
    await overlay.getByRole('button', { name: 'Close' }).focus();
    await page.keyboard.press('Escape');
    await expect(overlay).toHaveCount(0);
  });
});
