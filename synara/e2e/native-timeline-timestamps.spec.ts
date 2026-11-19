import { expect, test, type Page } from '@playwright/test';

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
  // The row's own hover behaviour is unchanged: the gutter time and the rail.
  const row = page.locator(grouped);
  await row.getByText('A grouped message').hover();
  await expect(row.locator('[data-native-timeline-gutter-time]')).toBeVisible();
  await expect(row.locator('[data-native-timeline-action-rail]')).toBeVisible();
});

for (const zoom of [75, 100, 200]) {
  for (const monospace of [false, true]) {
    test(`the gutter time fits at ${zoom}% with the ${monospace ? 'monospace' : 'default'} font`, async ({
      page,
    }) => {
      await open(page);
      await applyAppearance(page, zoom, monospace);
      const row = page.locator(grouped);
      const time = row.locator('[data-native-timeline-gutter-time]');
      const body = row.getByText('A grouped message');
      await body.scrollIntoViewIfNeeded();
      const restingLeft = (await body.boundingBox())?.x;
      await body.hover();
      await expect(time).toBeVisible();
      await expect(time.locator('[aria-hidden="true"]')).toHaveText('12:59');

      const geometry = await time.evaluate((node) => {
        const reveal = node.parentElement as HTMLElement;
        const gutter = reveal.parentElement as HTMLElement;
        const timeBox = node.getBoundingClientRect();
        const gutterBox = gutter.getBoundingClientRect();
        return {
          scrollWidth: reveal.scrollWidth,
          clientWidth: reveal.clientWidth,
          timeLeft: timeBox.left,
          timeRight: timeBox.right,
          gutterLeft: gutterBox.left,
          gutterRight: gutterBox.right,
        };
      });
      expect(geometry.scrollWidth).toBeLessThanOrEqual(geometry.clientWidth);
      expect(geometry.timeLeft).toBeGreaterThanOrEqual(geometry.gutterLeft - 0.5);
      expect(geometry.timeRight).toBeLessThanOrEqual(geometry.gutterRight + 0.5);

      // Header and grouped rows share the gutter, and hover never moves text.
      expect((await body.boundingBox())?.x).toBe(restingLeft);
      const headerBody = page.locator(header).getByText('Message', { exact: false }).first();
      expect((await headerBody.boundingBox())?.x).toBeCloseTo(restingLeft ?? 0, 1);
    });
  }
}

test('the gutter time fits in 24-hour mode too', async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem('settings', JSON.stringify({ hour24Clock: true }))
  );
  await open(page);
  await applyAppearance(page, 200, true);
  // The setting took: the header drops the meridiem.
  await expect(page.locator(header).locator('time')).toHaveText('12:59');
  const time = page.locator(grouped).locator('[data-native-timeline-gutter-time]');
  await expect(time.locator('[aria-hidden="true"]')).toHaveText('12:59');
  const fits = await time.evaluate((node) => {
    const reveal = node.parentElement as HTMLElement;
    return reveal.scrollWidth <= reveal.clientWidth;
  });
  expect(fits).toBe(true);
});

test('the time stays accessible and hovering it shows the full date', async ({ page }) => {
  await open(page);
  // In the app the room list sits left of the timeline, where the gutter's
  // tooltip grows; at the window's edge folds would move it back on screen.
  await page.evaluate(() => {
    document.body.style.paddingLeft = '320px';
  });
  const time = page.locator(grouped).locator('[data-native-timeline-gutter-time]');
  // Screen readers get the full date and time, not just the short clock.
  await expect(time).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T/);
  await expect(time).toContainText(/, .*2\d{3} 12:59 PM$/);
  expect(await time.evaluate((node) => node.closest('[aria-hidden="true"]'))).toBeNull();

  await page.locator(grouped).getByText('A grouped message').hover();
  await time.hover();
  const tooltip = page.getByRole('tooltip');
  await expect(tooltip).toHaveText(/day, .*2\d{3} 12:59 PM$/);
  // The tooltip sits above the time and grows left from its edge, so it covers
  // the gutter, never the message text that starts right of the gutter.
  const tooltipBox = await tooltip.boundingBox();
  const timeBox = await time.boundingBox();
  expect((tooltipBox?.y ?? 0) + (tooltipBox?.height ?? 0)).toBeLessThanOrEqual(timeBox?.y ?? 0);
  expect((tooltipBox?.x ?? 0) + (tooltipBox?.width ?? 0)).toBeLessThanOrEqual(
    (timeBox?.x ?? 0) + (timeBox?.width ?? 0) + 0.5
  );
  await page.mouse.move(0, 0);
  await expect(tooltip).toHaveCount(0);

  // The header row's time gives the full date the same way.
  const headerTime = page.locator(header).locator('time');
  await expect(headerTime).toHaveText('12:59 PM');
  await expect(headerTime).toHaveAttribute('datetime', /^\d{4}-\d{2}-\d{2}T/);
  await headerTime.hover();
  await expect(page.getByRole('tooltip')).toHaveText(/day, .*2\d{3} 12:59 PM$/);
  // Beside the header time, on the sender's line, not over the text below.
  const headerTooltipBox = await page.getByRole('tooltip').boundingBox();
  const headerTimeBox = await headerTime.boundingBox();
  expect(headerTooltipBox?.x ?? 0).toBeGreaterThanOrEqual(
    (headerTimeBox?.x ?? 0) + (headerTimeBox?.width ?? 0)
  );
  const firstLine = await page
    .locator(header)
    .getByText('Message', { exact: false })
    .first()
    .boundingBox();
  expect((headerTooltipBox?.y ?? 0) + (headerTooltipBox?.height ?? 0)).toBeLessThanOrEqual(
    (firstLine?.y ?? 0) + 0.5
  );
});
