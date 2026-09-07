import { expect, test, type Page } from '@playwright/test';

type Fixture = {
  append(): void;
  edit(): void;
  unread(): void;
  prependMissing(): void;
  releaseJump(): void;
  send(roomId?: string): void;
  commands: { command: string }[];
};
const fixture = (page: Page, action: Exclude<keyof Fixture, 'commands'>) =>
  page.evaluate((key) => {
    (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture[key]();
  }, action);
const open = async (page: Page, scenario: string) => {
  await page.goto(`/e2e/native-timeline-harness/index.html?scenario=${scenario}`);
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
const scrollToHistory = async (page: Page) => {
  await page.locator('#native-timeline').hover();
  await page.mouse.wheel(0, -1600);
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(200);
  await expect(page.getByRole('button', { name: 'Jump to latest', exact: true })).toBeVisible();
};

test('live append follows bottom; history, edits and another room send do not move it', async ({
  page,
}) => {
  await open(page, 'live');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await fixture(page, 'append');
  await expect(page.locator('[data-native-timeline-event-id="$61"]')).toBeVisible();
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await expect(page.getByRole('button', { name: 'Jump to latest', exact: true })).toBeHidden();
  await scrollToHistory(page);
  const before = await geometry(page);
  await fixture(page, 'append');
  await fixture(page, 'edit');
  await page.evaluate(() =>
    (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.send(
      '!other:example.test'
    )
  );
  await expect.poll(async () => (await geometry(page)).eventId).toBe(before.eventId);
  await page.waitForTimeout(1000); // allow the controller's native snapshot poll to render both changes
  const after = await geometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
});

test('short unread room becomes live and follows later messages without a scroll gesture', async ({
  page,
}) => {
  await open(page, 'short');
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as unknown as { nativeTimelineFixture: Fixture }
          ).nativeTimelineFixture.commands.filter(
            (c) => c.command === 'matrix_timeline_follow_live'
          ).length
      )
    )
    .toBe(1);
  for (let index = 0; index < 12; index += 1) await fixture(page, 'append');
  await expect(page.locator('[data-native-timeline-event-id="$14"]')).toBeVisible();
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await expect(page.getByRole('button', { name: 'Jump to latest', exact: true })).toBeHidden();
});

test('stored bottom never overrides a new unread anchor', async ({ page }) => {
  await open(page, 'live');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await page.getByRole('button', { name: 'Toggle room' }).click();
  await fixture(page, 'unread');
  await page.getByRole('button', { name: 'Toggle room' }).click();
  await expect.poll(async () => (await geometry(page)).eventId).toBe('$2');
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(500);
});

test('missing last read retains the mounted location when later data arrives', async ({ page }) => {
  await open(page, 'missing');
  await expect(page.getByRole('button', { name: 'Jump to Last Read' })).toBeVisible();
  const before = await geometry(page);
  await fixture(page, 'prependMissing');
  await page.waitForTimeout(1000);
  const after = await geometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
  await expect(page.getByRole('button', { name: 'Jump to Last Read' })).toBeVisible();
});

test('send intent waits for new live provider and layout before hiding latest control', async ({
  page,
}) => {
  await open(page, 'live&delayJump=1');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await scrollToHistory(page);
  await fixture(page, 'send');
  await expect(page.getByRole('button', { name: 'Jump to latest', exact: true })).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as unknown as { nativeTimelineFixture: Fixture }
          ).nativeTimelineFixture.commands.filter(
            (c) => c.command === 'matrix_timeline_jump_latest'
          ).length
      )
    )
    .toBe(1);
  await fixture(page, 'releaseJump');
  await expect.poll(async () => (await geometry(page)).distance).toBeLessThanOrEqual(8);
  await expect(page.getByRole('button', { name: 'Jump to latest', exact: true })).toBeHidden();
});

test('delayed latest result cannot scroll a newer focused event in the same room', async ({
  page,
}) => {
  await open(page, 'live&delayJump=1');
  await scrollToHistory(page);
  await fixture(page, 'send');
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as unknown as { nativeTimelineFixture: Fixture }
          ).nativeTimelineFixture.commands.filter(
            (command) => command.command === 'matrix_timeline_jump_latest'
          ).length
      )
    )
    .toBe(1);
  await page.getByRole('button', { name: 'Focus middle' }).click();
  await expect(page.locator('[data-native-timeline-event-id="$30"]')).toBeInViewport();
  await expect.poll(async () => (await geometry(page)).distance).toBeGreaterThan(500);
  const before = await geometry(page);
  await fixture(page, 'releaseJump');
  await page.waitForTimeout(1000); // include provider return, layout and the next native snapshot poll
  const after = await geometry(page);
  expect(after.eventId).toBe(before.eventId);
  expect(Math.abs(after.offset - before.offset)).toBeLessThanOrEqual(2);
  await expect(page.locator('[data-native-timeline-event-id="$30"]')).toBeInViewport();
});

test('sparse history and missing last-read recovery controls are separately clickable', async ({
  page,
}) => {
  await open(page, 'sparse-missing');
  const older = page.getByRole('button', { name: 'Load older messages', exact: true });
  const lastRead = page.getByRole('button', { name: 'Jump to Last Read', exact: true });
  await expect(older).toBeVisible();
  await expect(lastRead).toBeVisible();
  const olderBox = await older.boundingBox();
  const lastReadBox = await lastRead.boundingBox();
  expect(olderBox).not.toBeNull();
  expect(lastReadBox).not.toBeNull();
  expect(olderBox!.y + olderBox!.height).toBeLessThanOrEqual(lastReadBox!.y);
  await older.click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as unknown as { nativeTimelineFixture: Fixture }
          ).nativeTimelineFixture.commands.filter(
            (command) => command.command === 'matrix_timeline_paginate'
          ).length
      )
    )
    .toBe(1);
  await lastRead.click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (
            window as unknown as { nativeTimelineFixture: Fixture }
          ).nativeTimelineFixture.commands.filter(
            (command) => command.command === 'matrix_timeline_open'
          ).length
      )
    )
    .toBe(2);
});
