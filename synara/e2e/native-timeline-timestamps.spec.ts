import { expect, test, type Locator, type Page } from '@playwright/test';

// `scenario=timestamps`: Ada's header row and two grouped rows, all at
// 12:59 PM today, the widest 12-hour clock time.
const fixture = '/e2e/native-timeline-harness/index.html?scenario=timestamps';
const header = '[data-native-timeline-event-id="$ada-1"]';
const grouped = '[data-native-timeline-event-id="$ada-2"]';
const edited = '[data-native-timeline-event-id="$ada-3"]';

const open = async (page: Page) => {
  await page.goto(fixture);
  await expect(page.locator(edited)).toContainText('An edited grouped message.');
};

/** Page zoom and the interface font, set the way the app sets them. */
const applyAppearance = (page: Page, zoom: number, monospace: boolean) =>
  page.evaluate(
    ({ zoom: percent, monospace: mono }) => {
      if (percent === 100) document.documentElement.style.removeProperty('font-size');
      else document.documentElement.style.setProperty('font-size', `calc(1em * ${percent / 100})`);
      document.body.style.fontFamily = mono ? 'ui-monospace, Menlo, monospace' : '';
    },
    { zoom, monospace }
  );

/** Every `<time>` in the row that takes up more than a hidden pixel. */
const drawnTimes = (row: Locator) =>
  row.locator('time').evaluateAll((nodes) =>
    nodes
      .filter((node) => {
        const box = node.getBoundingClientRect();
        return box.width > 1 || box.height > 1;
      })
      .map((node) => node.textContent)
  );

test('hovering a message body shows no timestamp over the text', async ({ page }) => {
  await open(page);
  for (const selector of [header, grouped, edited]) {
    const row = page.locator(selector);
    await row.locator('[data-native-timeline-row-surface]').scrollIntoViewIfNeeded();
    const body = row.getByText(/Message|grouped message/).first();
    await body.hover();
    // Longer than the tooltip delay, so a late tooltip would be open by now.
    await page.waitForTimeout(500);
    await expect(page.getByRole('tooltip')).toHaveCount(0);
    // No native `title` tooltip on the body or anything around it.
    const titles = await body.evaluate((node) => {
      const found: string[] = [];
      for (let el: Element | null = node; el; el = el.parentElement) {
        const title = el.getAttribute('title');
        if (title) found.push(title);
      }
      return found;
    });
    expect(titles).toEqual([]);
  }
  // The row's own hover behaviour is unchanged: the action rail still shows.
  await page.locator(grouped).getByText('A grouped message').hover();
  await expect(page.locator(grouped).locator('[data-native-timeline-action-rail]')).toBeVisible();
});

test('hovering or focusing a grouped row shows no time', async ({ page }) => {
  await open(page);
  for (const selector of [grouped, edited]) {
    const row = page.locator(selector);
    await row.getByText(/grouped message/).hover();
    await page.waitForTimeout(500);
    expect(await drawnTimes(row)).toEqual([]);
    await expect(page.getByRole('tooltip')).toHaveCount(0);

    await page.mouse.move(0, 0);
    await row.locator('[data-native-timeline-row-surface]').focus();
    await page.waitForTimeout(300);
    expect(await drawnTimes(row)).toEqual([]);
    await expect(page.getByRole('tooltip')).toHaveCount(0);
  }
  // The header row keeps its one visible time, beside the sender.
  expect(await drawnTimes(page.locator(header))).toEqual(['12:59 PM']);
});

for (const zoom of [75, 100, 200]) {
  for (const monospace of [false, true]) {
    test(`rows line up and hold still at ${zoom}% with the ${monospace ? 'monospace' : 'default'} font`, async ({
      page,
    }) => {
      await open(page);
      await applyAppearance(page, zoom, monospace);
      const body = page.locator(grouped).getByText('A grouped message');
      const headerBody = page.locator(header).getByText('Message', { exact: false }).first();
      await body.scrollIntoViewIfNeeded();
      const restingLeft = (await body.boundingBox())?.x ?? 0;
      const headerLeft = (await headerBody.boundingBox())?.x ?? 0;

      await body.hover();
      expect((await body.boundingBox())?.x).toBe(restingLeft);
      expect((await headerBody.boundingBox())?.x).toBe(headerLeft);
      // Header and grouped rows share the gutter, so their text lines up.
      expect(headerLeft).toBeCloseTo(restingLeft, 1);

      // The avatar fits its gutter at every zoom, clear of the sender's name.
      const gutter = await page
        .locator(header)
        .locator('[data-native-timeline-gutter]')
        .evaluate((node) => {
          const g = node.getBoundingClientRect();
          const a = (node.firstElementChild as HTMLElement).getBoundingClientRect();
          return { left: g.left, right: g.right, avatarLeft: a.left, avatarRight: a.right };
        });
      const nameBox = await page.locator(header).getByText('Ada', { exact: true }).boundingBox();
      expect(gutter.avatarLeft).toBeGreaterThanOrEqual(gutter.left - 0.5);
      expect(gutter.avatarRight).toBeLessThanOrEqual(gutter.right + 0.5);
      expect(gutter.avatarRight).toBeLessThan(nameBox?.x ?? 0);
    });
  }
}

test('grouped rows give screen readers their time', async ({ page }) => {
  await open(page);
  for (const selector of [grouped, edited]) {
    const time = page.locator(selector).locator('time');
    await expect(time).toHaveCount(1);
    await expect(time).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T/);
    await expect(time).toHaveText(/day, .*2\d{3} 12:59 PM$/);
    // In the accessibility tree, and not a tab stop of its own.
    expect(await time.evaluate((node) => node.closest('[aria-hidden="true"]'))).toBeNull();
    expect(await time.getAttribute('tabindex')).toBeNull();
  }
});

test('hovering the header time shows the full date beside it', async ({ page }) => {
  await open(page);
  const headerTime = page.locator(header).locator('time');
  await expect(headerTime).toHaveText('12:59 PM');
  await expect(headerTime).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T/);
  await headerTime.hover();
  const tooltip = page.getByRole('tooltip');
  await expect(tooltip).toHaveText(/day, .*2\d{3} 12:59 PM$/);
  // Beside the header time, on the sender's line, not over the text below.
  const tooltipBox = await tooltip.boundingBox();
  const timeBox = await headerTime.boundingBox();
  expect(tooltipBox?.x ?? 0).toBeGreaterThanOrEqual((timeBox?.x ?? 0) + (timeBox?.width ?? 0));
  const firstLine = await page
    .locator(header)
    .getByText('Message', { exact: false })
    .first()
    .boundingBox();
  expect((tooltipBox?.y ?? 0) + (tooltipBox?.height ?? 0)).toBeLessThanOrEqual(
    (firstLine?.y ?? 0) + 0.5
  );
  await page.mouse.move(0, 0);
  await expect(tooltip).toHaveCount(0);
});
