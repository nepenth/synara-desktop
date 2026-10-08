import { expect, test, type Page } from '@playwright/test';

const openThreads = async (page: Page, threadDisplay: string) => {
  await page.goto(
    `/e2e/native-timeline-harness/index.html?scenario=threads&threadDisplay=${threadDisplay}`
  );
  await expect(page.locator('[data-native-timeline-event-id]').first()).toBeVisible();
};

const openThreadStreams = (page: Page) =>
  page.evaluate(() =>
    (
      window as unknown as { nativeTimelineFixture: { openThreadStreams(): number } }
    ).nativeTimelineFixture.openThreadStreams()
  );

const threadButton = (page: Page) =>
  page.locator('#native-timeline').getByRole('button', { name: /Thread · 7 replies/ });

test('side panel opens the thread beside a live room timeline and closes on Escape', async ({
  page,
}) => {
  await openThreads(page, 'side');
  await threadButton(page).click();

  const pane = page.locator('[data-thread-side-panel]');
  await expect(pane).toBeVisible();
  await expect(pane.getByText('Thread reply 7')).toBeVisible();
  // The room timeline stays put: still the room, not swapped into the thread.
  await expect(page.locator('#native-timeline').getByText('Thread reply 7')).toHaveCount(0);
  await expect(threadButton(page)).toBeVisible();
  expect(await openThreadStreams(page)).toBe(1);

  await page.keyboard.press('Escape');
  await expect(pane).toHaveCount(0);
  await expect.poll(() => openThreadStreams(page)).toBe(0);
});

test('side panel resizes by keyboard within its bounds', async ({ page }) => {
  await openThreads(page, 'side');
  await threadButton(page).click();
  const pane = page.locator('[data-thread-side-panel]');
  const before = (await pane.boundingBox())?.width ?? 0;
  await page.getByRole('button', { name: /Resize thread panel/ }).focus();
  await page.keyboard.press('Shift+ArrowLeft');
  await expect.poll(async () => (await pane.boundingBox())?.width ?? 0).toBeGreaterThan(before);
  for (let step = 0; step < 20; step += 1) await page.keyboard.press('Shift+ArrowRight');
  await expect.poll(async () => Math.round((await pane.boundingBox())?.width ?? 0)).toBe(320);
});

test('inline threads expand under the root, reveal earlier replies, and close their stream', async ({
  page,
}) => {
  await openThreads(page, 'inline');
  const sequence = await page.locator('[data-native-timeline-event-id]').count();
  expect(sequence).toBeGreaterThan(1);
  const toggle = threadButton(page);
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  expect(await openThreadStreams(page)).toBe(0);

  // The label changes on expand, so measure the toggle by its stable attribute.
  const stableToggle = page.locator('[data-inline-thread] button[aria-expanded]').first();
  const toggleTop = async () => (await stableToggle.boundingBox())?.y ?? Number.NaN;
  // Measure only after the opening placement settles at the live bottom.
  let before = await toggleTop();
  await expect
    .poll(async () => {
      const previous = before;
      before = await toggleTop();
      return Math.abs(before - previous);
    })
    .toBeLessThan(0.5);
  await toggle.click();
  const inline = page.locator('[data-inline-thread]');
  await expect(inline.getByText('Thread reply 7')).toBeVisible();
  // Expanding near the live bottom must not push the thread off screen: the
  // root keeps its place while the replies grow beneath it.
  await expect.poll(async () => Math.abs((await toggleTop()) - before)).toBeLessThan(2);
  await expect(inline.getByText('Thread reply 3')).toBeVisible();
  // Only the latest five show until asked.
  await expect(inline.getByText('Thread reply 2')).toHaveCount(0);
  expect(await openThreadStreams(page)).toBe(1);
  // The control that was clicked stays on screen while the replies grow.
  await expect(page.getByRole('button', { name: /Hide 7 replies/ })).toBeInViewport();

  await inline.getByRole('button', { name: 'Show earlier replies' }).click();
  await expect(inline.getByText('Thread reply 1')).toBeVisible();
  await expect(inline.getByRole('button', { name: 'Show earlier replies' })).toHaveCount(0);

  await page.getByRole('button', { name: /Hide 7 replies/ }).click();
  await expect(inline.getByText('Thread reply 7')).toHaveCount(0);
  await expect.poll(() => openThreadStreams(page)).toBe(0);
});

test('inline Reply in thread opens the side panel', async ({ page }) => {
  await openThreads(page, 'inline');
  await threadButton(page).click();
  await page.getByRole('button', { name: 'Reply in thread' }).click();
  await expect(page.locator('[data-thread-side-panel]')).toBeVisible();
});

test('full view keeps swapping the timeline into the thread', async ({ page }) => {
  await openThreads(page, 'full');
  await threadButton(page).click();
  await expect(page.locator('#native-timeline').getByText('Thread reply 7')).toBeVisible();
  await expect(page.locator('[data-thread-side-panel]')).toHaveCount(0);
});
