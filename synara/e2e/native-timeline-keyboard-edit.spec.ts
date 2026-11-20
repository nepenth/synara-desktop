import { expect, test, type Locator, type Page } from '@playwright/test';

// Slack's quick edit: Up in an empty message box selects my last message in
// that timeline, Up and Down move between messages, E edits a selected message
// of mine in place, and Escape (or ending the edit) returns to the message
// box. The fixture user is Reader 0, who sent the even-numbered messages; with
// `editable` Core allows an edit of every message, so "mine" is the client's
// own rule here. It runs in WebKit too (the desktop app's engine).
// `theme` loads the app's colours, which the focus ring is drawn in.
const harness = (query: string) =>
  `/e2e/native-timeline-harness/index.html?${query}&editable=1&composer=1&theme=dark`;

type Fixture = { commands: { command: string; args?: Record<string, unknown> }[] };
const edits = (page: Page) =>
  page.evaluate(() =>
    (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
      .filter((entry) => entry.command === 'matrix_edit_message')
      .map((entry) => entry.args ?? {})
  );

const room = (page: Page) => page.locator('#native-timeline');
const roomComposer = (page: Page) => room(page).getByRole('textbox', { name: 'Message room' });
const surface = (scope: Locator, eventId: string) =>
  scope.locator(`[data-native-timeline-event-id="${eventId}"] [data-native-timeline-row-surface]`);
const editorIn = (scope: Locator, eventId: string) =>
  scope
    .locator(`[data-native-timeline-event-id="${eventId}"]`)
    .getByRole('textbox', { name: 'Edit message' });

/**
 * Types into a message box and waits until Slate holds the text: it reads
 * the DOM on its own schedule, and the Up rule asks Slate whether the box is
 * empty.
 */
const typeDraft = async (page: Page, text: string, editorName = 'synaraComposerEditor') => {
  await page.keyboard.type(text);
  await page.waitForFunction(
    ([name, expected]) => {
      const editor = (window as unknown as Record<string, { children: unknown[] }>)[name];
      return JSON.stringify(editor.children).includes(expected);
    },
    [editorName, text] as const
  );
};

const openRoom = async (page: Page) => {
  await page.goto(harness('scenario=live'));
  await expect(surface(room(page), '$60')).toBeVisible();
  await roomComposer(page).click();
  await expect(roomComposer(page)).toBeFocused();
};

test('Up in the empty message box selects my last message, and E edits it in place', async ({
  page,
}) => {
  await openRoom(page);

  await page.keyboard.press('ArrowUp');
  const mine = surface(room(page), '$60');
  await expect(mine).toBeFocused();
  // A real focus target that says who wrote what, with a ring that shows it.
  await expect(mine).toHaveAccessibleName(/^Reader 0: Message 60 /);
  await expect(mine).toHaveAttribute('aria-keyshortcuts', 'E');
  expect(
    await mine.evaluate((node) => {
      const style = getComputedStyle(node);
      return style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0;
    })
  ).toBe(true);

  await page.keyboard.press('e');
  const editor = editorIn(room(page), '$60');
  await expect(editor).toBeFocused();
  // The message as it was: the E that opened the editor is not typed into it.
  await expect(editor).toContainText('Line three.');
  await expect(editor).not.toContainText('Line three.e');

  await page.keyboard.type(' Fixed.');
  await expect(editor).toContainText('Line three. Fixed.');
  await page.keyboard.press('Enter');
  await expect(editor).toHaveCount(0);
  const sent = await edits(page);
  expect(sent).toHaveLength(1);
  expect(sent[0].eventId).toBe('$60');
  expect(String(sent[0].body)).toContain('Line three. Fixed.');
  // Saving ends in the message box, ready for the next message.
  await expect(roomComposer(page)).toBeFocused();
  expect(
    await page.evaluate(() =>
      JSON.stringify(
        (window as unknown as { synaraComposerEditor: { children: unknown[] } })
          .synaraComposerEditor.children
      )
    )
  ).not.toContain('Fixed');
});

test('Up and Down move between messages, E leaves other people’s alone, Escape and Down return', async ({
  page,
}) => {
  await openRoom(page);
  await page.keyboard.press('ArrowUp');
  await expect(surface(room(page), '$60')).toBeFocused();

  await page.keyboard.press('ArrowUp');
  const theirs = surface(room(page), '$59');
  await expect(theirs).toBeFocused();
  await expect(theirs).toHaveAccessibleName(/^Reader 1: Message 59 /);
  // Not my message: E does not open an editor on it.
  await page.keyboard.press('e');
  await expect(room(page).getByRole('textbox', { name: 'Edit message' })).toHaveCount(0);
  expect(await edits(page)).toHaveLength(0);

  // Escape goes back to the message box from any selected message.
  await surface(room(page), '$59').focus();
  await page.keyboard.press('Escape');
  await expect(roomComposer(page)).toBeFocused();

  // Down from the newest message is the message box too.
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  await expect(surface(room(page), '$59')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(surface(room(page), '$60')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(roomComposer(page)).toBeFocused();

  // Selection keeps going past the rows that were on screen at the start.
  await page.keyboard.press('ArrowUp');
  for (let step = 0; step < 30; step += 1) {
    await page.keyboard.press('ArrowUp');
    await expect(surface(room(page), `$${59 - step}`)).toBeFocused();
  }
  await expect(surface(room(page), '$30')).toBeInViewport();
});

test('Up keeps moving the caret while there is a draft', async ({ page }) => {
  await openRoom(page);
  await typeDraft(page, 'draft');
  await page.keyboard.press('ArrowUp');
  await expect(roomComposer(page)).toBeFocused();
  await expect(room(page).locator('[data-native-timeline-row-surface]:focus')).toHaveCount(0);

  // Emptied again, Up selects the message.
  // (Up put the caret at the start of the line, so select the draft to delete it.)
  await page.keyboard.press('ControlOrMeta+a');
  await page.keyboard.press('Backspace');
  await page.waitForFunction(
    () =>
      !/"text":"[^"]/.test(
        JSON.stringify(
          (window as unknown as { synaraComposerEditor: { children: unknown[] } })
            .synaraComposerEditor.children
        )
      )
  );
  await page.keyboard.press('ArrowUp');
  await expect(surface(room(page), '$60')).toBeFocused();
});

test('cancelling an edit opened with E returns to the message box; one opened from the row returns to the message', async ({
  page,
}) => {
  await openRoom(page);
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('e');
  await expect(editorIn(room(page), '$60')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(editorIn(room(page), '$60')).toHaveCount(0);
  await expect(roomComposer(page)).toBeFocused();

  // The row's own Edit button keeps its focus rule: back to the message.
  const row = room(page).locator('[data-native-timeline-event-id="$58"]');
  await row.hover();
  await row.getByRole('button', { name: 'Edit' }).click();
  await expect(editorIn(room(page), '$58')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(editorIn(room(page), '$58')).toHaveCount(0);
  await expect(surface(room(page), '$58')).toBeFocused();
  expect(await edits(page)).toHaveLength(0);
});

test('in a thread pane, Up selects my last reply in that thread and Escape keeps the pane open', async ({
  page,
}) => {
  await page.goto(harness('scenario=threads&threadDisplay=side'));
  await room(page)
    .getByRole('button', { name: /Thread · 7 replies/ })
    .click();
  const pane = page.locator('[data-thread-side-panel]');
  await expect(pane.getByText('Thread reply 7')).toBeVisible();
  const threadComposer = pane.getByRole('textbox', { name: 'Reply in thread' });
  await threadComposer.click();
  await expect(threadComposer).toBeFocused();

  // Reply 7 is Reader 1's; my last reply in this thread is reply 6.
  await page.keyboard.press('ArrowUp');
  await expect(surface(pane, '$t6')).toBeFocused();
  await expect(room(page).locator('[data-native-timeline-row-surface]:focus')).toHaveCount(0);

  await page.keyboard.press('Escape');
  await expect(threadComposer).toBeFocused();
  await expect(pane).toBeVisible();

  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('e');
  const editor = editorIn(pane, '$t6');
  await expect(editor).toBeFocused();
  await expect(editor).toContainText('Thread reply 6');
  await page.keyboard.type('!');
  await page.keyboard.press('Enter');
  await expect(editor).toHaveCount(0);
  const sent = await edits(page);
  expect(sent).toHaveLength(1);
  expect(sent[0].eventId).toBe('$t6');
  await expect(threadComposer).toBeFocused();
  await expect(pane).toBeVisible();

  // The room's own message box still selects in the room, not in the thread.
  await roomComposer(page).click();
  await page.keyboard.press('ArrowUp');
  await expect(room(page).locator('[data-native-timeline-row-surface]:focus')).toHaveCount(1);
  await expect(pane.locator('[data-native-timeline-row-surface]:focus')).toHaveCount(0);
});
