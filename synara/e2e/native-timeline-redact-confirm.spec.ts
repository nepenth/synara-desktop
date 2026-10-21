import { expect, test, type Locator, type Page } from '@playwright/test';

type Fixture = {
  commands: { command: string; args?: Record<string, unknown> }[];
};

const redactions = (page: Page) =>
  page.evaluate(() =>
    (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
      .filter((entry) => entry.command === 'matrix_timeline_redact')
      .map((entry) => entry.args ?? {})
  );

const harness = (query: string) => `/e2e/native-timeline-harness/index.html?${query}`;

const QUESTION = "Remove this message? This can't be undone.";

const row = (scope: Page | Locator, eventId: string) =>
  scope.locator(`[data-native-timeline-event-id="${eventId}"]`);

/** Hover a message, open its menu and choose Redact. */
const chooseRedact = async (scope: Page | Locator, eventId: string) => {
  const message = row(scope, eventId);
  await message.hover();
  await message.getByRole('button', { name: 'More message actions' }).click();
  const menu = message.page().getByRole('menu', { name: 'Message actions' });
  await menu.getByRole('menuitem', { name: 'Redact' }).click();
  return message.page().getByRole('alertdialog', { name: QUESTION });
};

/** Focus lands on the row's menu button and stays: the rail does not unmount under it. */
const expectFocusOnMenuButton = async (message: Locator) => {
  const button = message.getByRole('button', { name: 'More message actions' });
  await expect(button).toBeFocused();
  await message
    .page()
    .evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
    );
  await expect(button).toBeFocused({ timeout: 0 });
};

test.beforeEach(async ({ page }) => {
  await page.goto(harness('scenario=live&polish=1'));
  await expect(row(page, '$4')).toBeVisible();
});

test('Redact asks first, with Cancel focused and Remove named for what it does', async ({
  page,
}) => {
  const dialog = await chooseRedact(page, '$4');
  await expect(dialog).toBeVisible();
  await expect(page.getByRole('menu', { name: 'Message actions' })).toHaveCount(0);
  await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
  await expect(dialog.getByRole('button', { name: 'Remove', exact: true })).toBeVisible();
  // No reason field yet.
  await expect(dialog.getByRole('textbox')).toHaveCount(0);
  expect(await redactions(page)).toHaveLength(0);
});

test('Cancel leaves the message and returns focus to its menu button', async ({ page }) => {
  const dialog = await chooseRedact(page, '$4');
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(row(page, '$4')).toContainText('Message 4');
  await expectFocusOnMenuButton(row(page, '$4'));
  expect(await redactions(page)).toHaveLength(0);
});

test('Remove redacts the message once', async ({ page }) => {
  const dialog = await chooseRedact(page, '$4');
  await dialog.getByRole('button', { name: 'Remove', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await expect.poll(() => redactions(page)).toHaveLength(1);
  expect(await redactions(page)).toEqual([
    { request: { roomId: '!navigation:example.test', eventId: '$4' } },
  ]);
  await expectFocusOnMenuButton(row(page, '$4'));
  await expect(page.locator('[data-native-timeline-notice="error"]')).toHaveCount(0);
  // Nothing settles the closed dialog a second time.
  await page.keyboard.press('Escape');
  expect(await redactions(page)).toHaveLength(1);
});

test('Escape cancels', async ({ page }) => {
  const dialog = await chooseRedact(page, '$4');
  await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(row(page, '$4')).toContainText('Message 4');
  await expectFocusOnMenuButton(row(page, '$4'));
  expect(await redactions(page)).toHaveLength(0);
});

test('a click outside the dialog cancels', async ({ page }) => {
  const dialog = await chooseRedact(page, '$4');
  await page.mouse.click(5, 5);
  await expect(dialog).toHaveCount(0);
  expect(await redactions(page)).toHaveLength(0);
});

test('a thread reply in the side panel asks too, and Escape keeps the panel open', async ({
  page,
}) => {
  await page.goto(harness('scenario=threads&threadDisplay=side&polish=1'));
  await page
    .locator('#native-timeline')
    .getByRole('button', { name: /Thread · 7 replies/ })
    .click();
  const pane = page.locator('[data-thread-side-panel]');
  await expect(row(pane, '$t7')).toBeVisible();

  const dialog = await chooseRedact(pane, '$t7');
  await expect(dialog).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(pane).toBeVisible();
  expect(await redactions(page)).toHaveLength(0);

  await (
    await chooseRedact(pane, '$t7')
  )
    .getByRole('button', { name: 'Remove', exact: true })
    .click();
  await expect.poll(() => redactions(page)).toHaveLength(1);
  expect((await redactions(page))[0]).toMatchObject({ request: { eventId: '$t7' } });
});
