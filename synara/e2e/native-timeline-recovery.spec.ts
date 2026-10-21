import { expect, test, type Page } from '@playwright/test';

type Fixture = {
  commands: { command: string; args?: Record<string, unknown> }[];
};

const commands = (page: Page, command: string) =>
  page.evaluate(
    (name) =>
      (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
        .filter((entry) => entry.command === name)
        .map((entry) => entry.args ?? {}),
    command
  );

const harness = (query: string) => `/e2e/native-timeline-harness/index.html?${query}`;

const rows = (page: Page) => page.locator('#native-timeline [data-native-timeline-event-id]');

/** Hover a message so its action rail shows, then open its menu. */
const openRowMenu = async (page: Page, eventId: string) => {
  const row = page.locator(`[data-native-timeline-event-id="${eventId}"]`);
  await row.hover();
  await row.getByRole('button', { name: 'More message actions' }).click();
  return page.getByRole('menu', { name: 'Message actions' });
};

test('a failed open explains itself without internal wording and retries', async ({ page }) => {
  await page.goto(harness('scenario=live&failOpen=1'));
  const error = page.locator('[data-native-timeline-error="true"]');
  await expect(error.getByRole('alert')).toHaveText(
    'Synara could not load these messages. Try again. (Error code: d0.3-timeline-open-failed)'
  );
  await expect(error).not.toContainText(/native/i);
  // Already at the live position: there is nothing else to go to.
  await expect(error.getByRole('button', { name: 'Go to latest' })).toHaveCount(0);
  await error.getByRole('button', { name: 'Try again' }).click();
  await expect(rows(page).first()).toBeVisible();
  await expect(error).toHaveCount(0);
});

test('a focused open that fails falls back to the live room with a notice', async ({ page }) => {
  await page.goto(harness('scenario=live'));
  await expect(rows(page).first()).toBeVisible();
  await page.getByRole('button', { name: 'Focus broken' }).click();
  const notice = page.locator('[data-native-timeline-notice="info"]');
  await expect(notice).toHaveText(
    'That message could not be opened. Showing the latest messages instead.'
  );
  await expect(notice).toHaveAttribute('role', 'status');
  await expect(page.locator('[data-native-timeline-error="true"]')).toHaveCount(0);
  await expect(page.locator('[data-native-timeline-event-id="$60"]')).toBeVisible();
  await notice.getByRole('button', { name: 'Dismiss' }).click();
  await expect(notice).toHaveCount(0);
});

test('the message menu is a menu of menu items with a toolbar trigger', async ({ page }) => {
  await page.goto(harness('scenario=live&polish=1'));
  const row = page.locator('[data-native-timeline-event-id="$4"]');
  await row.hover();
  await expect(row.getByRole('toolbar', { name: 'Message tools' })).toBeVisible();
  const reaction = row.getByRole('button', { name: 'Add reaction' });
  await expect(reaction).toHaveAttribute('aria-haspopup', 'dialog');
  await expect(reaction).toHaveAttribute('aria-expanded', 'false');
  await expect(reaction).not.toHaveAttribute('aria-pressed', /.*/);

  const menu = await openRowMenu(page, '$4');
  await expect(menu).toBeVisible();
  await expect(menu.getByRole('menuitem', { name: 'Copy Message' })).toBeVisible();
  await expect(menu.getByRole('group', { name: 'Moderation actions' })).toBeVisible();
  // Arrow keys move through the items.
  await page.keyboard.press('ArrowDown');
  await expect(menu.getByRole('menuitem').first()).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(menu.getByRole('menuitem').nth(1)).toBeFocused();
  // Closing returns focus to the message rather than dropping it.
  await page.keyboard.press('Escape');
  await expect(menu).toHaveCount(0);
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          document.activeElement
            ?.closest('[data-native-timeline-event-id]')
            ?.getAttribute('data-native-timeline-event-id') ?? null
      )
    )
    .toBe('$4');
});

test('keyboard focus on rail buttons is visible', async ({ page }) => {
  await page.goto(harness('scenario=live&polish=1'));
  const row = page.locator('[data-native-timeline-event-id="$4"]');
  await row.locator('[role="group"][aria-label="Message actions"]').focus();
  await page.keyboard.press('Tab');
  const focused = row.getByRole('button', { name: 'Add reaction' });
  await expect(focused).toBeFocused();
  const outline = await focused.evaluate((node) => {
    const style = getComputedStyle(node);
    return { width: style.outlineWidth, style: style.outlineStyle };
  });
  expect(outline.style).toBe('solid');
  expect(parseFloat(outline.width)).toBeGreaterThan(0);
});

test('editing happens in place with the composer and keeps the message type', async ({ page }) => {
  await page.goto(harness('scenario=live&polish=1&editable=1'));
  const menu = await openRowMenu(page, '$4');
  await menu.getByRole('menuitem', { name: 'Edit' }).click();
  await expect(menu).toHaveCount(0);

  const row = page.locator('[data-native-timeline-event-id="$4"]');
  const editor = row.getByRole('textbox', { name: 'Edit message' });
  await expect(editor).toBeFocused();
  await expect(editor).toContainText('Message 4');
  // No raw HTML field any more.
  await expect(page.getByLabel('Edit message HTML body')).toHaveCount(0);

  // Clicking elsewhere does not discard the edit.
  await page.locator('[data-native-timeline-event-id="$2"]').click();
  await expect(editor).toBeVisible();

  await editor.click();
  await page.keyboard.press('ControlOrMeta+a');
  await page.keyboard.type('Edited message');
  await page.keyboard.press('Enter');
  await expect(row.locator('[data-native-timeline-editor]')).toHaveCount(0);
  const edits = await commands(page, 'matrix_edit_message');
  expect(edits).toHaveLength(1);
  expect(edits[0]).toMatchObject({
    eventId: '$4',
    body: 'Edited message',
    msgType: 'm.text',
    mentionUserIds: [],
    mentionRoom: false,
  });
});

test('Escape cancels an in-place edit without sending', async ({ page }) => {
  await page.goto(harness('scenario=live&polish=1&editable=1'));
  const menu = await openRowMenu(page, '$4');
  await menu.getByRole('menuitem', { name: 'Edit' }).click();
  const editor = page.getByRole('textbox', { name: 'Edit message' });
  await expect(editor).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(editor).toHaveCount(0);
  expect(await commands(page, 'matrix_edit_message')).toHaveLength(0);
});

test('the timeline is a labelled region with a polite log for arrivals', async ({ page }) => {
  await page.goto(harness('scenario=live&nativeEvents=1'));
  await expect(page.getByRole('region', { name: 'Messages' })).toBeVisible();
  const log = page.getByRole('log');
  await expect(log).toHaveAttribute('aria-live', 'polite');
  await expect(log).toHaveText('');
  await page.evaluate(() =>
    (
      window as unknown as { nativeTimelineFixture: { appendLive(): void } }
    ).nativeTimelineFixture.appendLive()
  );
  // Reader 1 is someone else (the harness signs in as Reader 0).
  await expect(log).toContainText('Reader 1: Message 61');
});
