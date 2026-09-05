import { expect, Page, test } from '@playwright/test';

const openHarness = async (page: Page) => {
  await page.goto('/');
  await page.waitForFunction(() => Boolean((window as any).timelineHarness));
};

test.beforeEach(async ({ page }) => {
  await openHarness(page);
});

test('renders deterministic 1-200-line rows while keeping the live window bounded', async ({
  page,
}) => {
  const metrics = await page.evaluate(() => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!variable:example.org', 120, { maxLines: 200 });
    harness.openRoom('!variable:example.org');
    return harness.getRowMetrics();
  });

  expect(metrics.count).toBe(120);
  expect(metrics.minLines).toBe(1);
  expect(metrics.maxLines).toBe(200);
  expect(metrics.maxHeight - metrics.minHeight).toBeGreaterThan(3_000);

  const boundedState = await page.evaluate(() => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!large:example.org', 5_000, { maxLines: 8 });
    return harness.openRoom('!large:example.org');
  });
  expect(boundedState.renderedRowCount).toBe(120);
  expect(boundedState.rangeStart).toBe(4_880);
  expect(boundedState.rangeEnd).toBe(5_000);
});

test('prepend pagination preserves the measured event and pixel anchor within two pixels', async ({
  page,
}) => {
  const result = await page.evaluate(async () => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!pagination:example.org', 400, { maxLines: 12 });
    harness.openRoom('!pagination:example.org', { start: 200, end: 280 });
    await harness.scrollEventToOffset('!pagination:example.org-event-240', 24);
    harness.clearScrollWrites();
    const anchor = harness.captureAnchor();
    await harness.prepend(40);
    return {
      anchor,
      drift: harness.getAnchorDrift(anchor),
      state: harness.getState(),
    };
  });

  expect(result.anchor.eventId).toMatch(/^!pagination:example\.org-event-\d+$/);
  expect(result.drift).toBeLessThanOrEqual(2);
  expect(result.state.renderedRowCount).toBeLessThanOrEqual(120);
  expect(result.state.rangeStart).toBe(160);
  expect(result.state.scrollWrites.every((write: any) => !write.activeUserScroll)).toBe(true);
});

test('late image, font, decryption, and reply layout changes preserve the viewport anchor', async ({
  page,
}) => {
  const drifts = await page.evaluate(async () => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!late-layout:example.org', 100, { maxLines: 5 });
    harness.openRoom('!late-layout:example.org');
    await harness.scrollEventToOffset('!late-layout:example.org-event-70', 18);
    harness.clearScrollWrites();
    const anchor = harness.captureAnchor();
    const measuredDrifts: number[] = [];

    await harness.expandImage('!late-layout:example.org-event-5', 320);
    measuredDrifts.push(harness.getAnchorDrift(anchor));
    await harness.loadFontMetrics(21);
    measuredDrifts.push(harness.getAnchorDrift(anchor));
    await harness.decryptEvent('!late-layout:example.org-event-10', 160);
    measuredDrifts.push(harness.getAnchorDrift(anchor));
    await harness.expandReply('!late-layout:example.org-event-20', 12);
    measuredDrifts.push(harness.getAnchorDrift(anchor));

    return { measuredDrifts, state: harness.getState() };
  });

  expect(drifts.measuredDrifts).toHaveLength(4);
  drifts.measuredDrifts.forEach((drift: number) => expect(drift).toBeLessThanOrEqual(2));
  expect(drifts.state.renderedRowCount).toBeLessThanOrEqual(120);
  expect(drifts.state.scrollWrites.every((write: any) => !write.activeUserScroll)).toBe(true);
});

test('wheel momentum queues structural changes and performs no active-scroll writes', async ({
  page,
}) => {
  await page.evaluate(async () => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!momentum:example.org', 100, { maxLines: 8 });
    harness.openRoom('!momentum:example.org');
    await harness.scrollEventToOffset('!momentum:example.org-event-55', 20);
    harness.clearScrollWrites();
  });

  const timeline = page.locator('#timeline');
  await timeline.hover();
  await page.mouse.wheel(0, 180);
  await page.mouse.wheel(0, 160);

  const duringScroll = await page.evaluate(async () => {
    const harness = (window as any).timelineHarness;
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
    );
    const anchor = harness.captureAnchor();
    const mutation = await harness.expandImage('!momentum:example.org-event-8', 280);
    return { anchor, mutation, state: harness.getState() };
  });

  expect(duringScroll.mutation.queued).toBe(true);
  expect(duringScroll.state.userScrolling).toBe(true);
  expect(duringScroll.state.queuedMutationCount).toBe(1);
  expect(duringScroll.state.scrollWrites).toHaveLength(0);

  const settled = await page.evaluate(async (anchor) => {
    const harness = (window as any).timelineHarness;
    await harness.waitForIdle();
    return { drift: harness.getAnchorDrift(anchor), state: harness.getState() };
  }, duringScroll.anchor);

  expect(settled.drift).toBeLessThanOrEqual(2);
  expect(settled.state.userScrolling).toBe(false);
  expect(settled.state.queuedMutationCount).toBe(0);
  expect(settled.state.scrollWrites.every((write: any) => !write.activeUserScroll)).toBe(true);
});

test('Jump Latest stays visible through loading/failure and hides only after bottom confirmation', async ({
  page,
}) => {
  await page.evaluate(() => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!jump:example.org', 300, { maxLines: 10 });
    harness.openRoom('!jump:example.org', { start: 0, end: 120 });
    harness.clearScrollWrites();
  });

  const jumpButton = page.locator('#jump-latest');
  await expect(jumpButton).toBeVisible();
  const failedJump = page.evaluate(() =>
    (window as any).timelineHarness.jumpLatest({ delayMs: 50, fail: true })
  );
  await expect(jumpButton).toHaveText('Loading latest…');
  await expect(jumpButton).toBeVisible();
  expect(await failedJump).toEqual({ ok: false, stale: false });
  await expect(jumpButton).toBeVisible();

  const successfulJump = page.evaluate(() =>
    (window as any).timelineHarness.jumpLatest({ delayMs: 50 })
  );
  await expect(jumpButton).toHaveText('Loading latest…');
  await expect(jumpButton).toBeVisible();
  const success = await successfulJump;
  expect(success.ok).toBe(true);
  expect(success.bottomGap).toBeLessThanOrEqual(2);
  await expect(jumpButton).toBeHidden();

  const state = await page.evaluate(() => (window as any).timelineHarness.getState());
  expect(state.phase).toBe('bottomConfirmed');
  expect(state.rangeEnd).toBe(300);
  expect(state.renderedRowCount).toBeLessThanOrEqual(120);
  expect(state.scrollWrites.every((write: any) => !write.activeUserScroll)).toBe(true);
});

test('rapid room changes invalidate late Jump Latest completions from the previous generation', async ({
  page,
}) => {
  const result = await page.evaluate(async () => {
    const harness = (window as any).timelineHarness;
    harness.seedRoom('!room-a:example.org', 300, { maxLines: 6 });
    harness.seedRoom('!room-b:example.org', 80, { maxLines: 6 });
    harness.openRoom('!room-a:example.org', { start: 0, end: 120 });
    harness.clearScrollWrites();
    const pendingJump = harness.jumpLatest({ delayMs: 80 });
    harness.openRoom('!room-b:example.org', { atBottom: true });
    harness.clearScrollWrites();
    const jumpResult = await pendingJump;
    return { jumpResult, state: harness.getState() };
  });

  expect(result.jumpResult).toEqual({ ok: false, stale: true });
  expect(result.state.roomId).toBe('!room-b:example.org');
  expect(result.state.rangeEnd).toBe(80);
  expect(result.state.staleOperations).toBe(1);
  expect(
    result.state.scrollWrites.some((write: any) => write.reason === 'jump-latest-live-tail')
  ).toBe(false);
});

test('measures a short unread room without a scroll event, then tracks content and window resizing', async ({
  page,
}) => {
  await page.goto('/');
  await page.evaluate(async () => {
    const { observeNativeTimelineBottom } = await import('/nativeTimelineVisibility.js');
    document.body.innerHTML =
      '<div id="room" style="height:400px;overflow:auto"><div style="height:80px">Unread message</div></div>';
    const room = document.getElementById('room')!;
    const state = { atBottom: false, scrollEvents: 0, reports: 0 };
    room.addEventListener('scroll', () => {
      state.scrollEvents += 1;
    });
    const stop = observeNativeTimelineBottom(room, (atBottom: boolean) => {
      state.atBottom = atBottom;
      state.reports += 1;
    });
    Object.assign(window, { visibilityTest: { state, stop } });
  });
  const atBottom = () => page.evaluate(() => (window as any).visibilityTest.state.atBottom);
  await expect.poll(atBottom).toBe(true);
  expect(await page.evaluate(() => (window as any).visibilityTest.state.scrollEvents)).toBe(0);
  await page.locator('#room > div').evaluate((element) => {
    (element as HTMLElement).style.height = '800px';
  });
  await expect.poll(atBottom).toBe(false);
  await page.locator('#room').evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });
  await expect.poll(atBottom).toBe(true);
  await page.locator('#room').evaluate((element) => {
    (element as HTMLElement).style.height = '200px';
  });
  await expect.poll(atBottom).toBe(false);
  await page.locator('#room').evaluate((element) => {
    element.innerHTML = '<div style="height:20px">Replacement</div>';
  });
  await expect.poll(atBottom).toBe(true);
  const reports = await page.evaluate(() => {
    const t = (window as any).visibilityTest;
    t.stop();
    return t.state.reports;
  });
  await page.locator('#room').evaluate((element) => {
    element.innerHTML = '<div style="height:900px">Unseen</div>';
  });
  await page.evaluate(
    () =>
      new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
      )
  );
  expect(await page.evaluate(() => (window as any).visibilityTest.state.reports)).toBe(reports);
});
