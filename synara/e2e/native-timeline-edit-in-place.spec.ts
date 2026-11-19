import { expect, test, type Page } from '@playwright/test';

// Editing a message with several blocks in place. The editor loads the
// message with one insertFragment over a select-all and then focuses, so the
// first frame has the new blocks in the model but not yet in the DOM (the same
// window as the 3.1.0 "Cannot resolve a DOM node" crash). It runs in WebKit
// too: the desktop app is WKWebView on macOS and WebKitGTK on Linux.
type Fixture = { commands: { command: string; args?: Record<string, unknown> }[] };

const edits = (page: Page) =>
  page.evaluate(() =>
    (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
      .filter((entry) => entry.command === 'matrix_edit_message')
      .map((entry) => entry.args ?? {})
  );

test('a message with a quote, a list and code opens whole in the in-place editor and saves', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => {
    // WebKit reports a ResizeObserver loop as the editor grows to fit the
    // blocks (on main too); it is a notice, not a failure of the page.
    if (!error.message.startsWith('ResizeObserver loop')) errors.push(error.message);
  });
  await page.goto(
    '/e2e/native-timeline-harness/index.html?scenario=live&polish=1&editable=1&richEdit=1'
  );
  const row = page.locator('[data-native-timeline-event-id="$4"]');
  await row.hover();
  await row.getByRole('button', { name: 'More message actions' }).click();
  await page
    .getByRole('menu', { name: 'Message actions' })
    .getByRole('menuitem', { name: 'Edit' })
    .click();

  const editor = row.getByRole('textbox', { name: 'Edit message' });
  await expect(editor).toBeFocused();
  await expect(editor.locator('blockquote')).toContainText('quoted');
  await expect(editor.locator('ul > li')).toHaveText(['one', 'two']);
  await expect(editor.locator('pre')).toContainText('code()');
  await expect(editor).toContainText('Intro line');

  // The caret is at the end of the message, so typing goes on there.
  await page.keyboard.type('X');
  await expect(editor.locator('pre')).toContainText('code()X');

  await page.keyboard.press('Enter');
  await expect(row.locator('[data-native-timeline-editor]')).toHaveCount(0);
  const sent = await edits(page);
  expect(sent).toHaveLength(1);
  const html = String(sent[0].formattedBody ?? '');
  expect(html).toContain('<blockquote>');
  expect(html).toContain('<ul>');
  expect(html).toContain('<pre>');
  expect(html).toContain('code()X');
  expect(errors).toEqual([]);
});
