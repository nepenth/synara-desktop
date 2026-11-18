import { expect, test, type Page } from '@playwright/test';

// A message with a mention and a link, then an emoji as the last thing. Every
// emoji is an inline void, so the cursor ends in a new empty text after it
// that React has not drawn yet. The emoji board's focus trap closes it while
// React unmounts it, mid-commit, and the composer refocused the editor there:
// slate-dom could not map the cursor to the DOM ("Cannot resolve a DOM node
// from Slate node: {"text":""}") and the error page replaced the screen.
// Each path that inserts an emoji, and each order of mention and link, must
// leave the composer focused and typable.
const COMPOSER = '/e2e/approvals-harness/index.html?composer&toolbar';
const LINK = 'https://example.org/page';

type Node = { type?: string; text?: string; children?: Node[] };

const paragraph = async (page: Page): Promise<Node[]> =>
  page.evaluate(
    () =>
      (window as unknown as { synaraComposerEditor: { children: { children: unknown }[] } })
        .synaraComposerEditor.children[0].children as Node[]
  );

const mention = async (page: Page) => {
  await page.keyboard.type('@F');
  const row = page.getByRole('button', { name: 'Fiona' });
  await row.click();
  await expect(row).toHaveCount(0);
  // The composer refocuses on the next frame; keys sent before it go nowhere.
  await expect(page.locator('[data-slate-editor]')).toBeFocused();
};

// A copied link arrives as HTML and becomes a link element.
const pasteLink = async (page: Page) => {
  await page.locator('[data-slate-editor]').evaluate((editor, href) => {
    const data = new DataTransfer();
    data.setData('text/html', `<a href="${href}">${href}</a>`);
    data.setData('text/plain', href);
    editor.dispatchEvent(
      new ClipboardEvent('paste', { clipboardData: data, bubbles: true, cancelable: true })
    );
  }, LINK);
  await expect(page.locator(`[data-slate-editor] a[href="${LINK}"]`)).toBeVisible();
};

type Insert = (page: Page) => Promise<void>;

const fromPicker: Insert = async (page) => {
  await page.getByRole('button', { name: 'Emoji' }).click();
  await page.locator('[data-emoji-type]').first().click();
};

const fromPickerSearch: Insert = async (page) => {
  await page.getByRole('button', { name: 'Emoji' }).click();
  await page.getByPlaceholder('Search').fill('smile');
  await page.locator('[data-emoji-shortcode="smile"]').click();
};

const fromShortcode: Insert = async (page) => {
  await page.keyboard.type(' :smile');
  // The menu opens above the composer, off this page's top edge: focus the row
  // (as a click on it does) and click it there.
  const row = page.locator('button', { hasText: ':smile:' }).first();
  await row.focus();
  await row.dispatchEvent('click');
};

// Direct text input, as the macOS emoji panel and an IME commit it.
const typed: Insert = async (page) => {
  await page.keyboard.insertText('😀');
};

const insertions: [string, Insert, boolean][] = [
  ['picker', fromPicker, true],
  ['picker search', fromPickerSearch, true],
  [':shortcode autocomplete', fromShortcode, true],
  ['typed character', typed, false],
];

const contents: [string, (page: Page) => Promise<void>][] = [
  [
    'mention then link',
    async (page) => {
      await mention(page);
      await pasteLink(page);
    },
  ],
  [
    'link then mention',
    async (page) => {
      await pasteLink(page);
      await page.keyboard.type(' ');
      await mention(page);
    },
  ],
  [
    'mention and link, then text',
    async (page) => {
      await mention(page);
      await pasteLink(page);
      await page.keyboard.type(' see');
    },
  ],
];

for (const [contentName, compose] of contents) {
  for (const [insertName, insert, inline] of insertions) {
    test(`${contentName}, emoji from ${insertName}`, async ({ page }) => {
      const errors: string[] = [];
      page.on('pageerror', (error) => errors.push(error.message));
      await page.goto(COMPOSER);
      const editor = page.locator('[data-slate-editor]');
      await editor.click();
      await page.keyboard.type('hey ');
      await compose(page);

      await insert(page);

      await expect(editor).toBeFocused();
      await page.keyboard.type('ok');
      await expect(editor).toContainText('ok');
      expect(errors).toEqual([]);
      const nodes = await paragraph(page);
      const types = nodes.map((node) => node.type ?? 'text');
      expect(types).toContain('mention');
      expect(types).toContain('link');
      if (inline) expect(types).toContain('emoticon');
      else expect(JSON.stringify(nodes)).toContain('😀');
      // Typing goes on after the emoji, not into it or into the link.
      expect(nodes[nodes.length - 1].text?.endsWith('ok')).toBe(true);
    });
  }
}
