import { expect, test, type Page } from '@playwright/test';

const openFixture = (page: Page) => page.goto('/e2e/room-list-harness/index.html');

// Inspect every room whose full 40px row should be visible. Text presence alone
// misses rows mounted in the wrong place; verify their actual viewport geometry.
const expectVisibleRooms = async (page: Page) => {
  await expect
    .poll(() =>
      page.evaluate(() => {
        const scroll = document.querySelector('[data-testid="room-scroll"]')!;
        const list = document.querySelector('[data-testid="room-list"]')!;
        const viewport = scroll.getBoundingClientRect();
        const origin = list.getBoundingClientRect().top;
        const missing: number[] = [];
        for (let index = 0; index < 100; index += 1) {
          const expectedTop = origin + index * 40;
          if (expectedTop < viewport.top + 2 || expectedTop + 40 > viewport.bottom - 2) continue;
          const row = document.querySelector(`[data-room-index="${index}"]`);
          if (!row || Math.abs(row.getBoundingClientRect().top - expectedTop) > 1)
            missing.push(index);
        }
        return missing;
      })
    )
    .toEqual([]);
};

const scrollTo = async (page: Page, top: number) => {
  await page.getByTestId('room-scroll').evaluate((element, value) => {
    element.scrollTop = value;
  }, top);
  await expectVisibleRooms(page);
};

test('all visible rooms stay painted beyond favorites when scrolling down and back', async ({
  page,
}) => {
  await openFixture(page);
  await expect(page.getByTestId('room-list')).toHaveCSS('height', '4000px');
  for (const top of [0, 480, 560, 640, 1040, 2400, 3960, 1040, 560, 0]) {
    await scrollTo(page, top);
  }
  await expect(page.locator('[data-room-index="0"]')).toBeVisible();
  expect(await page.locator('[data-room-index]').count()).toBeLessThan(40);
});

test('favorite changes, resize, sorting, and empty-to-populated transitions preserve row geometry', async ({
  page,
}) => {
  await openFixture(page);
  await scrollTo(page, 640);
  await page.getByRole('button', { name: 'Toggle favorites' }).click();
  await scrollTo(page, 1040);
  await page.getByTestId('room-scroll').evaluate((element) => {
    element.style.height = '420px';
  });
  await expectVisibleRooms(page);
  await page.getByRole('button', { name: 'Reverse rooms' }).click();
  await expectVisibleRooms(page);
  await scrollTo(page, 1008);
  await expect(page.locator('[data-room-index="0"]')).toHaveText('# Project — room-99');
  await page.getByRole('button', { name: 'Toggle list' }).click();
  await page.getByRole('button', { name: 'Toggle list' }).click();
  await scrollTo(page, 1040);
  await expectVisibleRooms(page);
});
