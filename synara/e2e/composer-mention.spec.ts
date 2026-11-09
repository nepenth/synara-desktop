import { expect, test } from '@playwright/test';

// Picking a member from the mention autocomplete (the production
// AutocompleteMenu). The click moves focus off the editor, as WebKitGTK does
// on Linux; the pick inserts a pill and a space; then the menu closes. Its
// focus trap deactivates while React unmounts it and asks the composer to
// close, which refocused the editor in the middle of React's commit. The new
// space had no DOM node yet, so slate-dom threw "Cannot resolve a DOM node
// from Slate node" and the screen went to the error page.
const COMPOSER = '/e2e/approvals-harness/index.html?composer&toolbar';

test('picking a mention from the list keeps the composer working', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(COMPOSER);
  const editor = page.locator('[data-slate-editor]');
  await editor.click();
  await page.keyboard.type('hi @F');
  const row = page.getByRole('button', { name: 'Fiona' });
  await expect(row).toBeVisible();
  // Move focus off the editor first, as a WebKitGTK click on the row does.
  await row.focus();
  await row.click();

  await expect(page.getByRole('button', { name: 'Fiona' })).toHaveCount(0);
  await expect(editor).toBeFocused();
  await page.keyboard.type('hello');
  await expect(editor).toContainText('hi @Fiona');
  await expect(editor).toContainText('hello');
  expect(errors).toEqual([]);
  // A real mention (it sends as a matrix.to pill), not the text "@Fiona".
  const doc = await page.evaluate(() =>
    JSON.stringify((window as unknown as { synaraComposerEditor: { children: unknown } }).synaraComposerEditor.children)
  );
  expect(doc).toContain('"type":"mention"');
  expect(doc).toContain('"id":"@fiona:example.org"');
});
