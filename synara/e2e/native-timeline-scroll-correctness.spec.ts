import { expect, test, type Page } from '@playwright/test';

type Fixture = {
  appendLive(): void;
  prependHistory(count?: number): void;
  prependMediaWithoutInfo(count?: number): void;
  metadataPulse(): void;
  emitDeltaFlood(count?: number): void;
  outOfOrderLiveAppends(): void;
  emitRevisionGap(): void;
  metadataThenOps(): void;
  growEdit(eventId?: string): void;
  addReaction(eventId?: string): void;
  insertUngroupedHeader(): void;
  scrollEventIntoView(eventId: string): void;
  resizeTimeline(height: number): void;
};

const fixture = async (page: Page, action: keyof Fixture, arg?: number | string) =>
  page.evaluate(
    ([key, value]) => {
      const api = (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture;
      const method = api[key] as (value?: number | string) => void;
      method(value);
    },
    [action, arg] as const
  );

const open = async (page: Page, query: string) => {
  await page.goto(`/e2e/native-timeline-harness/index.html?${query}`);
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
      height: viewport.clientHeight,
    };
  });

const settledGeometry = async (page: Page) => {
  let previous = await geometry(page);
  for (let attempt = 0; attempt < 30; attempt += 1) {
    await page.waitForTimeout(100);
    const current = await geometry(page);
    if (
      current.eventId === previous.eventId &&
      current.top === previous.top &&
      Math.abs(current.offset - previous.offset) <= 0.5
    ) {
      return current;
    }
    previous = current;
  }
  throw new Error('Native timeline viewport did not settle');
};

const scrollToHistory = async (page: Page) => {
  await page.locator('#native-timeline').hover();
  await page.mouse.wheel(0, -1600);
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(200);
};

test('a sub-96px user scroll during live appends is not yanked back', async ({ page }) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'appendLive');
  await page.locator('#native-timeline').hover();
  await page.mouse.wheel(0, -80);
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(8);
  const departed = await geometry(page);
  await fixture(page, 'appendLive');
  await page.waitForTimeout(150);
  const after = await geometry(page);
  expect(after.distance).toBeGreaterThan(8);
  expect(Math.abs(after.offset - departed.offset)).toBeLessThanOrEqual(2);
  await expect(page.getByRole('button', { name: 'Jump to latest', exact: true })).toBeVisible();
});

test('scrollIntoView of a history row while following live releases ownership', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await page.evaluate(() => {
    const viewport = [...document.querySelectorAll<HTMLElement>('#native-timeline *')].find(
      (node) => ['auto', 'scroll'].includes(getComputedStyle(node).overflowY)
    );
    if (!viewport) throw new Error('Native Scroll viewport missing');
    const first = viewport.querySelector<HTMLElement>('[data-native-timeline-event-id]');
    if (!first) throw new Error('No visible event');
    first.scrollIntoView({ block: 'end', inline: 'nearest' });
  });
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(8);
  await fixture(page, 'appendLive');
  await page.waitForTimeout(150);
  expect((await geometry(page)).distance).toBeGreaterThan(8);
});

test('Home leaves follow-live; End returns to the tail; PageDown at bottom does not jump', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await page.evaluate(() => {
    const viewport = [...document.querySelectorAll<HTMLElement>('#native-timeline *')].find(
      (node) => ['auto', 'scroll'].includes(getComputedStyle(node).overflowY)
    );
    if (!viewport) throw new Error('Native Scroll viewport missing');
    viewport.tabIndex = 0;
    viewport.focus();
  });
  await page.keyboard.press('Home');
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(200);
  await fixture(page, 'appendLive');
  await page.waitForTimeout(120);
  expect((await geometry(page)).distance).toBeGreaterThan(8);
  await page.keyboard.press('End');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  const atEnd = await geometry(page);
  await page.keyboard.press('PageDown');
  await page.waitForTimeout(80);
  const afterPageDown = await geometry(page);
  expect(Math.abs(afterPageDown.top - atEnd.top)).toBeLessThanOrEqual(2);
  await fixture(page, 'appendLive');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
});

test('window resize at the live tail keeps follow-live; history resize does not snap', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'resizeTimeline', 320);
  await page.waitForTimeout(80);
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'resizeTimeline', 520);
  await page.waitForTimeout(80);
  await fixture(page, 'appendLive');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await scrollToHistory(page);
  const before = await geometry(page);
  await fixture(page, 'resizeTimeline', 360);
  await page.waitForTimeout(120);
  await fixture(page, 'appendLive');
  await page.waitForTimeout(120);
  const after = await geometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
});

test('two consecutive prepends keep the parked row within 2px', async ({ page }) => {
  await open(page, 'scenario=live&nativeEvents=1&jank=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await scrollToHistory(page);
  const before = await settledGeometry(page);
  await page.evaluate(async () => {
    const api = (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture;
    api.prependHistory(24);
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    api.prependHistory(24);
  });
  await page.waitForTimeout(300);
  const after = await settledGeometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
});

test('prepend without media info.w/h and a later prepend keep the parked row', async ({ page }) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await scrollToHistory(page);
  const before = await settledGeometry(page);
  await fixture(page, 'prependMediaWithoutInfo', 8);
  await page.waitForTimeout(80);
  await fixture(page, 'prependHistory', 16);
  await page.waitForTimeout(250);
  const after = await settledGeometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
});

test('unread open with wildly different measured heights still lands on the marker', async ({
  page,
}) => {
  await open(page, 'scenario=unread&nativeEvents=1&jank=1');
  await expect.poll(async () => (await geometry(page)).eventId).toBe('$2');
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(500);
  const placed = await settledGeometry(page);
  expect(placed.eventId).toBe('$2');
  expect(placed.offset).toBeLessThanOrEqual(8);
});

test('out-of-order same-frame revisions apply instead of desyncing', async ({ page }) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'outOfOrderLiveAppends');
  await expect(page.getByText('Native timeline stream lost synchronization.')).toBeHidden();
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
});

test('a true revision gap fails closed rather than dropping later rows silently', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'emitRevisionGap');
  await expect(page.getByText('Native timeline stream lost synchronization.')).toBeVisible();
});

test('a metadata flood past the coalesce cap then an ops delta stays recoverable', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'emitDeltaFlood', 130);
  await expect(page.getByText('Native timeline stream lost synchronization.')).toBeHidden();
  await fixture(page, 'metadataThenOps');
  await expect(page.getByText('Native timeline stream lost synchronization.')).toBeHidden();
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
});

test('growing an edited row above the parked row does not jump the parked anchor more than 2px', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await scrollToHistory(page);
  const before = await settledGeometry(page);
  await fixture(page, 'growEdit', '$1');
  await page.waitForTimeout(200);
  const after = await settledGeometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
});

test('a reaction on a later row does not jump the parked history anchor more than 2px', async ({
  page,
}) => {
  await open(page, 'scenario=live&nativeEvents=1');
  await scrollToHistory(page);
  const before = await settledGeometry(page);
  await fixture(page, 'addReaction', '$60');
  await page.waitForTimeout(250);
  const after = await settledGeometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
});
