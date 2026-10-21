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

type SendFixture = {
  nativeTimelineFixture: { send(roomId?: string, threadRootId?: string): void; threadRoot: string };
};
/** A composer send acknowledged by Core: into the room, or into the thread. */
const sendToRoom = (page: Page) =>
  page.evaluate(() =>
    (window as unknown as SendFixture).nativeTimelineFixture.send(undefined, undefined)
  );
const sendToThread = (page: Page) =>
  page.evaluate(() => {
    const { nativeTimelineFixture } = window as unknown as SendFixture;
    nativeTimelineFixture.send(undefined, nativeTimelineFixture.threadRoot);
  });

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

test('each view streams over its own delta channel, which closes with the view', async ({
  page,
}) => {
  type Channels = { subscriptions: number[]; threadOwners: unknown[]; roomOwners: unknown[] };
  const deltaChannels = () =>
    page.evaluate(() =>
      (
        window as unknown as { nativeTimelineFixture: { deltaChannels(): Channels } }
      ).nativeTimelineFixture.deltaChannels()
    );
  const commandArgs = (command: string) =>
    page.evaluate(
      (name) =>
        (
          window as unknown as {
            nativeTimelineFixture: {
              commands: { command: string; args?: Record<string, unknown> }[];
            };
          }
        ).nativeTimelineFixture.commands
          .filter((entry) => entry.command === name)
          .map((entry) => ({
            roomId: entry.args?.roomId,
            subscriptionId: entry.args?.subscriptionId,
          })),
      command
    );
  await page.goto(
    '/e2e/native-timeline-harness/index.html?scenario=threads&threadDisplay=inline&nativeEvents=1'
  );
  await expect(page.locator('[data-native-timeline-event-id]').first()).toBeVisible();
  // The room view subscribed before it opened, and its stream is its own.
  await expect
    .poll(deltaChannels)
    .toEqual({ subscriptions: [1], threadOwners: [], roomOwners: [1] });
  expect(await commandArgs('desktop_timeline_subscribe')).toEqual([
    { roomId: '!navigation:example.test', subscriptionId: undefined },
  ]);
  expect((await commandArgs('matrix_timeline_open'))[0]?.subscriptionId).toBe(1);

  await threadButton(page).click();
  await expect(page.locator('[data-inline-thread]').getByText('Thread reply 7')).toBeVisible();
  await expect
    .poll(deltaChannels)
    .toEqual({ subscriptions: [1, 2], threadOwners: [2], roomOwners: [1] });

  await page.getByRole('button', { name: /Hide 7 replies/ }).click();
  await expect
    .poll(deltaChannels)
    .toEqual({ subscriptions: [1], threadOwners: [], roomOwners: [1] });
  expect(await commandArgs('desktop_timeline_unsubscribe')).toEqual([
    { roomId: undefined, subscriptionId: 2 },
  ]);
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

test('a routed thread reply opens its thread beside the live room', async ({ page }) => {
  await openThreads(page, 'side');
  await page.getByRole('button', { name: 'Focus thread reply' }).click();

  const pane = page.locator('[data-thread-side-panel]');
  await expect(pane).toBeVisible();
  await expect(pane.getByText('Thread reply 3')).toBeVisible();
  // The room stays the room: it never shows thread replies as room messages.
  await expect(page.locator('#native-timeline').getByText('Thread reply 3')).toHaveCount(0);
  await expect(threadButton(page)).toBeVisible();
});

test('a routed thread reply in full view swaps the timeline into its thread', async ({ page }) => {
  await openThreads(page, 'full');
  await page.getByRole('button', { name: 'Focus thread reply' }).click();
  await expect(page.locator('#native-timeline').getByText('Thread reply 3')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Back to room' })).toBeVisible();
});

test('a routed message that no longer exists says so and keeps the room usable', async ({
  page,
}) => {
  await openThreads(page, 'side');
  await page.getByRole('button', { name: 'Focus deleted' }).click();
  await expect(page.getByText(/That message isn't available/)).toBeVisible();
  await expect(threadButton(page)).toBeVisible();
});

test('a room send leaves an open thread pane open', async ({ page }) => {
  await openThreads(page, 'side');
  await threadButton(page).click();
  const pane = page.locator('[data-thread-side-panel]');
  await expect(pane.getByText('Thread reply 7')).toBeVisible();

  await sendToRoom(page);
  await page.waitForTimeout(300);
  await expect(pane).toBeVisible();
  await expect(pane.getByText('Thread reply 7')).toBeVisible();
  expect(await openThreadStreams(page)).toBe(1);

  // A reply in the pane keeps both views where they are.
  await sendToThread(page);
  await page.waitForTimeout(300);
  await expect(pane.getByText('Thread reply 7')).toBeVisible();
  await expect(threadButton(page)).toBeVisible();
});

test('a reply sent from the full thread view stays in the thread', async ({ page }) => {
  await openThreads(page, 'full');
  await threadButton(page).click();
  const timeline = page.locator('#native-timeline');
  await expect(timeline.getByText('Thread reply 7')).toBeVisible();

  await sendToThread(page);
  await page.waitForTimeout(300);
  await expect(timeline.getByText('Thread reply 7')).toBeVisible();
  expect(await openThreadStreams(page)).toBe(1);
});

type ThreadFixture = {
  nativeTimelineFixture: {
    scrollEventIntoView(eventId: string): void;
    releaseThreadOpen(): void;
    threadOpenCount(): number;
    threadRoot: string;
  };
};

const openInline = async (page: Page, query: string) => {
  await page.goto(
    `/e2e/native-timeline-harness/index.html?scenario=threads&threadDisplay=inline&${query}`
  );
  await expect(page.locator('[data-native-timeline-event-id]').first()).toBeVisible();
};

test('inline replies render their Matrix HTML through the timeline renderer', async ({ page }) => {
  await openThreads(page, 'inline');
  await threadButton(page).click();
  const inline = page.locator('[data-inline-thread]');
  const formatted = inline.locator('p', { hasText: 'Thread reply 6 is' });
  await expect(formatted.locator('strong')).toHaveText('formatted');
  await expect(formatted.locator('code')).toHaveText('code');
  await expect(inline.locator('script')).toHaveCount(0);
});

test('an inline thread that fails to open says so and retries', async ({ page }) => {
  await openInline(page, 'failThreadOpen');
  await threadButton(page).click();
  const inline = page.locator('[data-inline-thread]');
  const alert = inline.getByRole('alert');
  await expect(alert).toContainText('Synara could not open this thread. Try again.');
  await expect(inline.getByText('Thread reply 7')).toHaveCount(0);

  await alert.getByRole('button', { name: 'Retry' }).click();
  await expect(inline.getByText('Thread reply 7')).toBeVisible();
  await expect(inline.getByRole('alert')).toHaveCount(0);
  expect(await openThreadStreams(page)).toBe(1);
});

test('a failed "Show earlier replies" shows an inline error with Retry', async ({ page }) => {
  await openInline(page, 'threadMore&failThreadPaginate');
  await threadButton(page).click();
  const inline = page.locator('[data-inline-thread]');
  const earlier = inline.getByRole('button', { name: 'Show earlier replies' });
  // The first press reveals the two replies already loaded.
  await earlier.click();
  await expect(inline.getByText('Thread reply 1')).toBeVisible();
  // Older replies remain on the server; this pagination is rejected.
  await earlier.click();
  const alert = inline.getByRole('alert');
  await expect(alert).toContainText('Synara could not load earlier replies. Try again.');
  await expect(inline.getByText('Thread reply 7')).toBeVisible();

  await alert.getByRole('button', { name: 'Retry' }).click();
  await expect(inline.getByRole('alert')).toHaveCount(0);
  await expect(earlier).toBeEnabled();
});

test('an expanded inline thread shows its replies at once after scrolling back', async ({
  page,
}) => {
  await openInline(page, 'holdThreadReopen');
  await threadButton(page).click();
  const inline = page.locator('[data-inline-thread]');
  await expect(inline.getByText('Thread reply 7')).toBeVisible();

  // Scroll the root out of the virtualized window: its stream closes.
  await page.locator('#native-timeline').hover();
  await expect
    .poll(async () => {
      await page.mouse.wheel(0, -2000);
      return page.locator('[data-inline-thread]').count();
    })
    .toBe(0);
  await expect.poll(() => openThreadStreams(page)).toBe(0);

  // Back again: the reopen is held, yet the cached replies show immediately.
  await expect
    .poll(async () => {
      await page.mouse.wheel(0, 2000);
      return page.locator('[data-inline-thread]').count();
    })
    .toBe(1);
  await page.evaluate(() => {
    const fixture = (window as unknown as ThreadFixture).nativeTimelineFixture;
    fixture.scrollEventIntoView(fixture.threadRoot);
  });
  await expect(inline.getByText('Thread reply 7')).toBeVisible();
  await expect(inline.getByText('Loading replies…')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /Hide 7 replies/ })).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as unknown as ThreadFixture).nativeTimelineFixture.threadOpenCount()
    )
  ).toBe(2);

  await page.evaluate(() =>
    (window as unknown as ThreadFixture).nativeTimelineFixture.releaseThreadOpen()
  );
  await expect.poll(() => openThreadStreams(page)).toBe(1);
  await expect(inline.getByText('Thread reply 7')).toBeVisible();
});
