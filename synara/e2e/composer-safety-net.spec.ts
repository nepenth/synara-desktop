import { expect, test, type Page } from '@playwright/test';

// An error inside the composer (3.1.1's "Cannot resolve a DOM node from Slate
// node") put the whole screen on the error page. The composer safety net
// contains it: the first failure remounts the composer with the draft, a
// second right after shows a one-line fallback in its place. The harness
// makes the composer throw while it renders through `synaraComposerFail`.
const COMPOSER = '/e2e/approvals-harness/index.html?composer&toolbar';
const FALLBACK = '[data-composer-fallback]';

type Node = { type?: string; text?: string; children?: Node[] };
type Harness = {
  synaraComposerEditor: { children: Node[] };
  synaraComposerFail: (options?: { alsoNext?: boolean }) => void;
  synaraComposerLog: string[];
};

const editorChildren = (page: Page): Promise<Node[]> =>
  page.evaluate(() => (window as unknown as Harness).synaraComposerEditor.children);

const fail = (page: Page, alsoNext = false) =>
  page.evaluate(
    (next) => (window as unknown as Harness).synaraComposerFail({ alsoNext: next }),
    alsoNext
  );

const composerLog = (page: Page): Promise<string[]> =>
  page.evaluate(() =>
    (window as unknown as Harness).synaraComposerLog.filter((entry) =>
      entry.startsWith('composer-safety-net')
    )
  );

/** A draft with a mention chip and text after it: rich content to keep. */
const writeDraft = async (page: Page) => {
  const editable = page.locator('[data-slate-editor]');
  await editable.click();
  await page.keyboard.type('Hello @F');
  await page.getByRole('button', { name: 'Fiona' }).click();
  await expect(editable).toBeFocused();
  await page.keyboard.type('see you');
  await expect(editable).toContainText('see you');
};

test.beforeEach(async ({ page }) => {
  await page.goto(COMPOSER);
  await expect(page.locator('[data-slate-editor]')).toBeVisible();
});

test('a first failure remounts the composer with its draft and focus, no fallback', async ({
  page,
}) => {
  await writeDraft(page);
  const before = await editorChildren(page);
  await fail(page);

  await expect(page.locator('[data-composer-generation="1"]')).toBeVisible();
  await expect(page.locator(FALLBACK)).toHaveCount(0);
  await expect(page.getByText('Something went wrong')).toHaveCount(0);
  // The same rich draft, mention chip included, in a fresh editor that has focus.
  expect(await editorChildren(page)).toEqual(before);
  expect(before[0].children?.some((node) => node.type === 'mention')).toBe(true);
  await expect(page.locator('[data-slate-editor]')).toBeFocused();
  await page.keyboard.type('!');
  await expect(page.locator('[data-slate-editor]')).toContainText('see you!');

  // Logged once to the desktop log, tagged, without the draft's words.
  const log = await composerLog(page);
  expect(log).toHaveLength(1);
  expect(log[0]).toContain('composer-safety-net contained surface=room outcome=recovered');
  expect(log[0]).not.toContain('draft words');
});

test('a second failure right after shows the inline fallback; Try again restores the text', async ({
  page,
  browserName,
}) => {
  await writeDraft(page);
  const composerHeight = (await page.locator('[data-composer-generation]').boundingBox())!.height;
  // The remount throws again, as a draft that breaks the editor would.
  await fail(page, true);

  const fallback = page.locator(FALLBACK);
  await expect(fallback).toBeVisible();
  await expect(fallback.getByRole('alert')).toHaveText(
    'The message box ran into a problem. Your draft is saved.'
  );
  // In the composer's height band, so the timeline above does not jump.
  const fallbackHeight = (await fallback.boundingBox())!.height;
  expect(Math.abs(fallbackHeight - composerHeight)).toBeLessThanOrEqual(2);
  // The composer had focus, so its first action has it now.
  const tryAgain = fallback.getByRole('button', { name: 'Try again' });
  // WebKit on macOS moves to buttons with Option+Tab, as Safari does by default.
  const tab = browserName === 'webkit' ? 'Alt+Tab' : 'Tab';
  await expect(tryAgain).toBeFocused();
  await page.keyboard.press(tab);
  await expect(fallback.getByRole('button', { name: 'Start fresh' })).toBeFocused();
  await page.keyboard.press(tab);
  await expect(fallback.getByRole('button', { name: 'Copy draft' })).toBeFocused();

  const log = await composerLog(page);
  expect(log.map((entry) => entry.match(/outcome=(\w+)/)?.[1])).toEqual(['recovered', 'fallback']);

  await tryAgain.click();
  await expect(fallback).toHaveCount(0);
  // The rich draft failed once already: it comes back as its text.
  await expect(page.locator('[data-slate-editor]')).toHaveText('Hello @Fiona see you');
  await expect(page.locator('[data-slate-editor]')).toBeFocused();
});

test('Start fresh empties the composer and Copy draft copies its text', async ({
  page,
  context,
  browserName,
}) => {
  test.skip(browserName !== 'chromium', 'clipboard permissions are Chromium-only here');
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await writeDraft(page);
  await fail(page, true);
  const fallback = page.locator(FALLBACK);
  await expect(fallback).toBeVisible();

  await fallback.getByRole('button', { name: 'Copy draft' }).click();
  await expect(fallback.getByRole('button', { name: 'Copied' })).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('Hello @Fiona see you');

  await fallback.getByRole('button', { name: 'Start fresh' }).click();
  await expect(fallback).toHaveCount(0);
  await expect(page.locator('[data-slate-editor]')).toBeFocused();
  expect(await editorChildren(page)).toEqual([{ type: 'paragraph', children: [{ text: '' }] }]);
});

test('a failure long after a recovery is a first failure again', async ({ page }) => {
  await page.goto(`${COMPOSER}&recoveryWindow=400`);
  await writeDraft(page);
  await fail(page);
  await expect(page.locator('[data-composer-generation="1"]')).toBeVisible();
  await page.waitForTimeout(600);
  await fail(page);
  await expect(page.locator('[data-composer-generation="2"]')).toBeVisible();
  await expect(page.locator(FALLBACK)).toHaveCount(0);
  await expect(page.locator('[data-slate-editor]')).toContainText('see you');
});

test('an error thrown by Slate itself while drawing the editor is contained', async ({ page }) => {
  await writeDraft(page);
  // A leaf with no text: slate-react cannot draw it.
  await page.evaluate(() => {
    const editor = (
      window as unknown as { synaraComposerEditor: { children: unknown[]; onChange: () => void } }
    ).synaraComposerEditor;
    editor.children = [...editor.children, { type: 'paragraph', children: [{ bold: true }] }];
    editor.onChange();
  });
  await expect(page.locator('[data-composer-generation="1"]')).toBeVisible();
  await expect(page.getByText('Something went wrong')).toHaveCount(0);
  await expect(page.locator(FALLBACK)).toHaveCount(0);
  await expect(page.locator('[data-slate-editor]')).toContainText('see you');
  expect(await composerLog(page)).toHaveLength(1);
});
