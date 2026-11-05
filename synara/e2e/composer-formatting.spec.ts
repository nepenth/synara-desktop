import { expect, Page, test } from '@playwright/test';

// Composer formatting through real key presses, so Enter, Shift+Enter and
// Backspace reach the editor the way they do in the app: as `beforeinput`
// events Slate turns into insertBreak / insertSoftBreak / deleteBackward.
// The rules are docs/composer-formatting-model.md.
const COMPOSER = '/e2e/approvals-harness/index.html?composer&toolbar';

type Doc = Array<{ type: string; start?: number; children: unknown[] }>;
const doc = (page: Page) =>
  page.evaluate(
    () =>
      (window as unknown as { synaraComposerEditor: { children: Doc } }).synaraComposerEditor
        .children
  );
const outline = async (page: Page) =>
  (await doc(page)).map((node) => `${node.type}${node.start ? `@${node.start}` : ''}`);

/** Puts the caret at the start of the first text node containing `word`. */
const caretBefore = (page: Page, word: string) =>
  page.evaluate((target) => {
    const editable = document.querySelector('[data-slate-editor]')!;
    const walker = document.createTreeWalker(editable, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const index = node.textContent?.indexOf(target) ?? -1;
      if (index >= 0) {
        const range = document.createRange();
        range.setStart(node, index);
        range.collapse(true);
        getSelection()!.removeAllRanges();
        getSelection()!.addRange(range);
        return;
      }
    }
    throw new Error(`no text ${target}`);
  }, word);

/**
 * Selects the whole document. Headless browsers on macOS do not run Cmd+A as
 * an editing command, so the selection is made the way the browser reports
 * a real select-all.
 */
const selectAll = async (page: Page) => {
  await page.evaluate(() => {
    const range = document.createRange();
    range.selectNodeContents(document.querySelector('[data-slate-editor]')!);
    getSelection()!.removeAllRanges();
    getSelection()!.addRange(range);
  });
  // Slate takes DOM selection changes on a throttled selectionchange.
  await page.waitForFunction(() => {
    const { selection } = (
      window as unknown as {
        synaraComposerEditor: { selection: { anchor: unknown; focus: unknown } | null };
      }
    ).synaraComposerEditor;
    return !!selection && JSON.stringify(selection.anchor) !== JSON.stringify(selection.focus);
  });
};

const open = async (page: Page) => {
  await page.goto(COMPOSER);
  const editable = page.getByRole('textbox', { name: 'Send a message...' });
  await editable.click();
  return editable;
};

test('numbered list: Shift+Enter, Tab, leaving the list and carrying the numbering on', async ({
  page,
}) => {
  const editable = await open(page);
  await page.getByRole('button', { name: 'Ordered list' }).click();
  await page.keyboard.type('one');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('sub');
  await page.keyboard.press('Tab');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.press('Shift+Enter'); // empty nested item moves out
  await page.keyboard.type('two');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.press('Shift+Enter'); // empty item leaves the list
  await page.keyboard.type('a note');
  await page.keyboard.press('Shift+Enter');
  await page.getByRole('button', { name: 'Ordered list' }).click();
  await page.keyboard.type('three');

  expect(await outline(page)).toEqual(['ordered-list', 'paragraph', 'ordered-list@3']);
  await expect(editable.locator('ol[start="3"] > li')).toHaveText(['three']);
  await expect(editable.locator('ol').first().locator(':scope > li')).toHaveCount(2);
  await expect(editable).toBeFocused();
});

test('quote: an empty last line leaves it; Backspace at its start removes it', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Block quote' }).click();
  await page.keyboard.type('quoted');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('after');
  expect(await outline(page)).toEqual(['block-quote', 'paragraph']);

  await caretBefore(page, 'quoted');
  await page.keyboard.press('Backspace');
  expect(await outline(page)).toEqual(['paragraph', 'paragraph']);
});

test('several selected lines become one list, and deleting everything leaves a plain line', async ({
  page,
}) => {
  const editable = await open(page);
  await page.keyboard.type('alpha');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('beta');
  await selectAll(page);
  await page.getByRole('button', { name: 'Bulleted list' }).click();
  expect(await outline(page)).toEqual(['unordered-list']);
  await expect(editable.locator('ul > li')).toHaveText(['alpha', 'beta']);

  await selectAll(page);
  await page.keyboard.press('Backspace');
  expect(await outline(page)).toEqual(['paragraph']);
  await expect(editable.locator('ul')).toHaveCount(0);
});

test('Backspace below a list joins the last item without losing text', async ({ page }) => {
  await open(page);
  await page.getByRole('button', { name: 'Ordered list' }).click();
  await page.keyboard.type('a');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('b');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.press('Shift+Enter');
  await page.keyboard.type('x');
  await caretBefore(page, 'x');
  await page.keyboard.press('Backspace');

  const [list] = await doc(page);
  expect(list.type).toBe('ordered-list');
  expect(JSON.stringify(list)).toContain('"bx"');
});
