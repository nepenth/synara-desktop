import { expect, test } from '@playwright/test';

test('idle Hermes ordered-list and code-block edits immediately reserve their committed height', async ({
  page,
}) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=short&nativeEvents=1&theme=1');
  await expect(page.locator('[data-native-timeline-event-id="$1"]')).toBeAttached();
  await page.evaluate(() => {
    (window as unknown as { hermesLayoutOverlaps: number[] }).hermesLayoutOverlaps = [];
    const observer = new MutationObserver(() => {
      const first = document.querySelector<HTMLElement>('[data-native-timeline-event-id="$1"]');
      const next = document.querySelector<HTMLElement>('[data-native-timeline-event-id="$2"]');
      if (!first?.querySelector('ol') || !next) return;
      const gap = next.getBoundingClientRect().top - first.getBoundingClientRect().bottom;
      if (gap < -1)
        (window as unknown as { hermesLayoutOverlaps: number[] }).hermesLayoutOverlaps.push(gap);
    });
    observer.observe(document.querySelector('#native-timeline')!, {
      childList: true,
      characterData: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['style'],
    });
  });
  for (const updates of [2, 54, 7, 62]) {
    // TanStack deliberately defers sync reads during user scrolling to its
    // ResizeObserver. Test the committed-layout path only while it owns that
    // immediate read; scrolling growth has separate frame-boundary coverage.
    await expect(page.locator('#native-timeline-history')).toHaveAttribute(
      'data-native-timeline-scrolling',
      'false'
    );
    await page.evaluate((count) => {
      (
        window as unknown as { nativeTimelineFixture: { hermesActivityEdit(count: number): void } }
      ).nativeTimelineFixture.hermesActivityEdit(count);
    }, updates);
    await expect(page.locator('[data-native-timeline-event-id="$1"] ol > li')).toHaveCount(updates);
    await expect
      .poll(() =>
        page.evaluate(() => {
          const first = document.querySelector<HTMLElement>(
            '[data-native-timeline-event-id="$1"]'
          )!;
          const next = document.querySelector<HTMLElement>('[data-native-timeline-event-id="$2"]')!;
          const descendants = [
            ...first.querySelectorAll<HTMLElement>('li, p, pre, [data-native-code-block]'),
          ];
          const paintedBottom = Math.max(
            first.getBoundingClientRect().bottom,
            ...descendants.map((node) => node.getBoundingClientRect().bottom)
          );
          return next.getBoundingClientRect().top - paintedBottom;
        })
      )
      .toBeGreaterThanOrEqual(-1);
  }
  await page.locator('#native-timeline').hover();
  await page.mouse.wheel(0, -900);
  await page.mouse.wheel(0, 900);
  await expect(page.locator('[data-native-timeline-event-id="$1"] ol > li')).toHaveCount(62);
  expect(
    await page.evaluate(
      () => (window as unknown as { hermesLayoutOverlaps: number[] }).hermesLayoutOverlaps
    )
  ).toEqual([]);
});

test('Hermes growth during scrolling reserves its height by the next rendering frame', async ({
  page,
}) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=short&nativeEvents=1&theme=1');
  await expect(page.locator('[data-native-timeline-event-id="$1"]')).toBeAttached();
  await page.evaluate(() => {
    (
      window as unknown as { nativeTimelineFixture: { hermesActivityEdit(count: number): void } }
    ).nativeTimelineFixture.hermesActivityEdit(7);
  });
  await expect(page.locator('[data-native-timeline-event-id="$1"] ol > li')).toHaveCount(7);
  await expect(page.locator('#native-timeline-history')).toHaveAttribute(
    'data-native-timeline-scrolling',
    'false'
  );
  await page.evaluate(() => {
    const viewport = document.querySelector<HTMLElement>('#native-timeline-history')!;
    const first = viewport.querySelector<HTMLElement>('[data-native-timeline-event-id="$1"]')!;
    const state = window as unknown as {
      hermesRenderedFrameGaps: number[];
      nativeTimelineFixture: { hermesActivityEdit(count: number): void };
    };
    state.hermesRenderedFrameGaps = [];
    // MutationObserver and an arbitrary rAF can run before the virtualizer's
    // ResizeObserver in the same frame. Schedule this rAF from ResizeObserver
    // so one complete rendering cycle has finished before sampling geometry.
    const observer = new ResizeObserver(() => {
      if (first.querySelectorAll('ol > li').length !== 62) return;
      requestAnimationFrame(() => {
        const next = viewport.querySelector<HTMLElement>('[data-native-timeline-event-id="$2"]');
        if (!first.isConnected || !next || first.querySelectorAll('ol > li').length !== 62) return;
        const bottom = Math.max(
          first.getBoundingClientRect().bottom,
          ...[...first.querySelectorAll<HTMLElement>('li, p, pre, [data-native-code-block]')].map(
            (node) => node.getBoundingClientRect().bottom
          )
        );
        state.hermesRenderedFrameGaps.push(next.getBoundingClientRect().top - bottom);
      });
    });
    observer.observe(first);
    // Activate the actual scrolling owner synchronously, rather than depending
    // on platform-specific wheel event scheduling in the test driver.
    viewport.scrollTop += viewport.scrollTop > 0 ? -10 : 10;
    viewport.dispatchEvent(new Event('scroll'));
    if (viewport.dataset.nativeTimelineScrolling !== 'true')
      throw new Error('Scrolling fixture did not enter its required state');
    state.nativeTimelineFixture.hermesActivityEdit(62);
  });
  await expect(page.locator('[data-native-timeline-event-id="$1"] ol > li')).toHaveCount(62);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as unknown as { hermesRenderedFrameGaps: number[] }).hermesRenderedFrameGaps
            .length
      )
    )
    .toBeGreaterThan(0);
  const frameGaps = await page.evaluate(
    () => (window as unknown as { hermesRenderedFrameGaps: number[] }).hermesRenderedFrameGaps
  );
  expect(
    frameGaps.every((gap) => gap >= -1),
    JSON.stringify(frameGaps)
  ).toBe(true);
});

test('formatted edits above parked history preserve the anchor without React commit warnings', async ({
  page,
}) => {
  const commitWarnings: string[] = [];
  page.on('console', (message) => {
    if (
      message.text().includes('flushSync') ||
      message.text().includes('Cannot update a component')
    )
      commitWarnings.push(message.text());
  });
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=live&nativeEvents=1&theme=1');
  await expect(page.locator('[data-native-timeline-event-id]').first()).toBeAttached();
  await page.locator('#native-timeline').hover();
  await page.mouse.wheel(0, -1400);
  await page.waitForTimeout(300);
  const before = await page.evaluate(() => {
    const viewport = document.querySelector<HTMLElement>('#native-timeline-history')!;
    const top = viewport.getBoundingClientRect().top;
    const rows = [...viewport.querySelectorAll<HTMLElement>('[data-native-timeline-event-id]')];
    const visible = rows.findIndex((node) => node.getBoundingClientRect().bottom > top + 1);
    return {
      eventId: rows[visible].dataset.nativeTimelineEventId!,
      editId: rows[visible - 1].dataset.nativeTimelineEventId!,
      offset: rows[visible].getBoundingClientRect().top - top,
    };
  });
  await page.evaluate((eventId) => {
    (
      window as unknown as {
        nativeTimelineFixture: { hermesActivityEdit(count: number, eventId: string): void };
      }
    ).nativeTimelineFixture.hermesActivityEdit(54, eventId);
  }, before.editId);
  await expect
    .poll(() =>
      page.evaluate((anchor) => {
        const viewport = document.querySelector<HTMLElement>('#native-timeline-history')!;
        const node = viewport.querySelector<HTMLElement>(
          `[data-native-timeline-event-id="${anchor.eventId}"]`
        );
        return node
          ? Math.abs(
              node.getBoundingClientRect().top -
                viewport.getBoundingClientRect().top -
                anchor.offset
            )
          : Infinity;
      }, before)
    )
    .toBeLessThanOrEqual(2);
  expect(commitWarnings).toEqual([]);
});
