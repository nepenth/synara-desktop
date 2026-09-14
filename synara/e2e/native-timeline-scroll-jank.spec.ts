import { expect, test, type Page } from '@playwright/test';

type JankFixture = {
  appendLive(): void;
  prependHistory(count?: number): void;
  metadataPulse(): void;
};

type JankMetrics = {
  samples: number;
  longFrames: number;
  droppedFrames: number;
  longFrameRatio: number;
  maxFrameMs: number;
  p95FrameMs: number;
  medianFrameMs: number;
};

const fixture = (page: Page, action: keyof JankFixture, count?: number) =>
  page.evaluate(
    ([key, value]) => {
      const api = (window as unknown as { nativeTimelineFixture: JankFixture })
        .nativeTimelineFixture;
      if (key === 'prependHistory') api.prependHistory(value);
      else api[key]();
    },
    [action, count] as const
  );

const openJank = async (page: Page) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=live&nativeEvents=1&jank=1');
  await expect(page.locator('[data-native-timeline-event-id]').first()).toBeVisible();
};

const geometry = (page: Page) =>
  page.evaluate(() => {
    const viewport = [...document.querySelectorAll<HTMLElement>('#native-timeline *')].find(
      (node) => ['auto', 'scroll'].includes(getComputedStyle(node).overflowY)
    );
    if (!viewport) throw new Error('Native Scroll viewport missing');
    const top = viewport.getBoundingClientRect().top;
    const visible = [
      ...viewport.querySelectorAll<HTMLElement>('[data-native-timeline-event-id]'),
    ].find((node) => node.getBoundingClientRect().bottom > top + 1);
    return {
      top: viewport.scrollTop,
      distance: viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight,
      eventId: visible?.dataset.nativeTimelineEventId,
      offset: (visible?.getBoundingClientRect().top ?? top) - top,
    };
  });

const startProbe = (page: Page) =>
  page.evaluate(() => {
    const frames: number[] = [];
    let last = performance.now();
    let handle = 0;
    const tick = (now: number) => {
      frames.push(now - last);
      last = now;
      handle = requestAnimationFrame(tick);
    };
    handle = requestAnimationFrame(tick);
    Object.assign(window, {
      __nativeTimelineJankProbe: {
        stop(): JankMetrics {
          cancelAnimationFrame(handle);
          const sample = frames.slice(3);
          const vsync = 1000 / 60;
          const long = sample.filter((duration) => duration > vsync);
          const dropped = sample.filter((duration) => duration > vsync * 2);
          const sorted = [...sample].sort((left, right) => left - right);
          return {
            samples: sample.length,
            longFrames: long.length,
            droppedFrames: dropped.length,
            longFrameRatio: sample.length === 0 ? 0 : long.length / sample.length,
            maxFrameMs: sample.length === 0 ? 0 : Math.max(...sample),
            p95FrameMs: sample.length === 0 ? 0 : sorted[Math.floor(sample.length * 0.95)] ?? 0,
            medianFrameMs: sample.length === 0 ? 0 : sorted[Math.floor(sample.length * 0.5)] ?? 0,
          };
        },
      },
    });
  });

const stopProbe = (page: Page) =>
  page.evaluate(() =>
    (
      window as unknown as {
        __nativeTimelineJankProbe: { stop(): JankMetrics };
      }
    ).__nativeTimelineJankProbe.stop()
  );

test('records scroll-jank while live appends, metadata pulses, and backward pagination run', async ({
  page,
}) => {
  await openJank(page);
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);

  await startProbe(page);
  for (let step = 0; step < 6; step += 1) {
    await fixture(page, 'appendLive');
    await page.waitForTimeout(300);
  }
  const followLiveMetrics = await stopProbe(page);
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);

  await page.locator('#native-timeline').hover();
  await page.mouse.wheel(0, -2400);
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(200);

  await startProbe(page);
  for (let step = 0; step < 8; step += 1) {
    await fixture(page, 'appendLive');
    if (step % 2 === 0) await fixture(page, 'metadataPulse');
    await page.mouse.wheel(0, step % 2 === 0 ? -180 : 160);
    await page.waitForTimeout(300);
  }
  const scrolledMetrics = await stopProbe(page);

  await page.waitForTimeout(120);
  const before = await geometry(page);
  await startProbe(page);
  await fixture(page, 'prependHistory', 40);
  await page.waitForTimeout(250);
  const prependMetrics = await stopProbe(page);
  const after = await geometry(page);

  const metrics = { followLiveMetrics, scrolledMetrics, prependMetrics };
  console.log(`native-timeline-scroll-jank ${JSON.stringify(metrics)}`);

  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
  expect(scrolledMetrics.samples).toBeGreaterThan(40);
  // Follow-live and wheel stay on the cheap path (p95 near vsync). A single
  // long frame on a loaded CI runner is not a product regression; a 40-row
  // prepend is allowed one expensive layout. Fail a multi-frame freeze.
  expect(followLiveMetrics.p95FrameMs).toBeLessThan(25);
  expect(scrolledMetrics.p95FrameMs).toBeLessThan(25);
  expect(followLiveMetrics.droppedFrames).toBeLessThan(8);
  expect(scrolledMetrics.droppedFrames).toBeLessThan(10);
  expect(prependMetrics.droppedFrames).toBeLessThan(8);
  expect(prependMetrics.maxFrameMs).toBeLessThan(150);
});
