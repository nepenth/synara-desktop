import { mkdirSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';

// Download feedback on a file attachment: progress on the file, the saved
// line with Show in folder, and failure with Retry. `saveControl` makes the
// harness's `matrix_media_save` wait while the test drives its channel.

type Saves = {
  pending(): number;
  started(total?: number): void;
  progress(received: number): void;
  finishing(): void;
  resolve(filename?: string): void;
  reject(diagnosticId?: string): void;
};
type Fixture = {
  saves: Saves;
  commands: { command: string; args?: Record<string, unknown> }[];
};

const ZIP_HANDLE = `timeline-media-${'cd'.repeat(32)}`;
const MD_HANDLE = `timeline-media-${'ab'.repeat(32)}`;
const SCREENSHOTS = process.env.SYNARA_FILE_SAVE_SCREENSHOTS;

const saves = <K extends keyof Saves>(page: Page, key: K, ...args: Parameters<Saves[K]>) =>
  page.evaluate(
    ([name, values]) => {
      const fixture = (window as unknown as { nativeTimelineFixture: Fixture })
        .nativeTimelineFixture;
      return (fixture.saves[name] as (...input: unknown[]) => unknown)(...values);
    },
    [key, args] as const
  );

const pendingSaves = (page: Page) =>
  page.evaluate(() =>
    (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.saves.pending()
  );

const commandArgs = (page: Page, command: string) =>
  page.evaluate(
    (name) =>
      (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
        .filter((entry) => entry.command === name)
        .map(({ args }) => ({
          contentUri: args?.contentUri,
          filename: args?.filename,
          hasChannel: args?.onProgress !== undefined,
        })),
    command
  );

const screenshot = async (page: Page, name: string) => {
  if (!SCREENSHOTS) return;
  mkdirSync(SCREENSHOTS, { recursive: true });
  await page
    .locator('[data-native-timeline-event-id]')
    .first()
    .screenshot({ path: `${SCREENSHOTS}/${name}.png` });
};

const openFile = async (page: Page, scenario: 'file-zip' | 'file-md') => {
  await page.goto(`/e2e/native-timeline-harness/index.html?scenario=${scenario}&saveControl`);
  await expect(page.locator('[data-native-timeline-event-id]')).toHaveCount(1);
};

test('a slow download shows progress on the file, then where it was saved', async ({ page }) => {
  await openFile(page, 'file-zip');
  const row = page.locator('[data-native-timeline-event-id]');
  const chip = page.getByRole('button', { name: 'Download archive.zip' });

  // A double click starts one save.
  await chip.dblclick();
  await expect.poll(() => pendingSaves(page)).toBe(1);
  await expect(chip).toBeDisabled();
  await saves(page, 'started', 2000);
  await saves(page, 'progress', 500);

  const progress = row.getByRole('progressbar', { name: 'Downloading archive.zip' });
  await expect(progress).toBeVisible();
  await expect(progress).toHaveAttribute('aria-valuetext', '25%');
  await expect(row.getByText('25%', { exact: true })).toBeVisible();
  await screenshot(page, 'file-save-progress');

  await saves(page, 'finishing');
  await expect(row.getByText('Finishing…')).toBeVisible();
  await expect(progress).toHaveCount(0);

  // Downloads already had archive.zip, so Rust saved it under another name.
  await saves(page, 'resolve', 'archive (1).zip');
  const saved = row.getByRole('status');
  await expect(saved).toHaveText(/Saved to Downloads as archive \(1\)\.zip/);
  await expect(chip).toBeEnabled();
  await screenshot(page, 'file-save-done');

  await saved.getByRole('button', { name: 'Show in folder' }).click();
  await expect
    .poll(() =>
      page.evaluate(() =>
        (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
          .filter((entry) => entry.command === 'desktop_reveal_download')
          .map((entry) => entry.args)
      )
    )
    .toEqual([{ filename: 'archive (1).zip' }]);

  await saved.getByRole('button', { name: 'Dismiss' }).click();
  await expect(row.getByRole('status')).toHaveCount(0);
  expect(await commandArgs(page, 'matrix_media_save')).toEqual([
    { contentUri: ZIP_HANDLE, filename: 'archive.zip', hasChannel: true },
  ]);
});

test('a download without a known size shows indeterminate progress', async ({ page }) => {
  await openFile(page, 'file-zip');
  const row = page.locator('[data-native-timeline-event-id]');
  await page.getByRole('button', { name: 'Download archive.zip' }).click();
  await expect.poll(() => pendingSaves(page)).toBe(1);
  await saves(page, 'started');
  await saves(page, 'progress', 4096);
  const progress = row.getByRole('progressbar', { name: 'Downloading archive.zip' });
  await expect(progress).toBeVisible();
  await expect(progress).not.toHaveAttribute('aria-valuenow', /.*/);
  await expect(row.getByText('Downloading…')).toBeVisible();
  await saves(page, 'resolve');
  await expect(row.getByRole('status')).toHaveText(/Saved to Downloads as archive\.zip/);
});

test('a failed download shows the error on the file and Retry saves it', async ({ page }) => {
  await openFile(page, 'file-zip');
  const row = page.locator('[data-native-timeline-event-id]');
  await page.getByRole('button', { name: 'Download archive.zip' }).click();
  await expect.poll(() => pendingSaves(page)).toBe(1);
  await saves(page, 'reject');

  const failure = row.getByRole('alert');
  await expect(failure).toContainText('Synara could not download this file. Try again.');
  // The error is on the file, not in the banner above the timeline.
  await expect(page.locator('[data-native-timeline-notice]')).toHaveCount(0);
  await screenshot(page, 'file-save-failed');

  await failure.getByRole('button', { name: 'Retry' }).click();
  await expect(row.getByRole('alert')).toHaveCount(0);
  await expect.poll(() => pendingSaves(page)).toBe(1);
  await saves(page, 'resolve');
  await expect(row.getByRole('status')).toHaveText(/Saved to Downloads as archive\.zip/);
  expect(await commandArgs(page, 'matrix_media_save')).toHaveLength(2);
});

test('a file over the save limit says so and offers no Retry', async ({ page }) => {
  await openFile(page, 'file-zip');
  const row = page.locator('[data-native-timeline-event-id]');
  await page.getByRole('button', { name: 'Download archive.zip' }).click();
  await expect.poll(() => pendingSaves(page)).toBe(1);
  await saves(page, 'reject', 'v-send.r-media-download-too-large');
  const failure = row.getByRole('alert');
  await expect(failure).toContainText('This file is larger than Synara can save (300 MB).');
  await expect(failure.getByRole('button', { name: 'Retry' })).toHaveCount(0);
});

test('the markdown preview and the file share one save and its result', async ({ page }) => {
  await openFile(page, 'file-md');
  await page.getByRole('button', { name: 'Preview notes.md' }).click();
  const preview = page.locator('[data-native-timeline-file-preview="true"]');
  const download = preview.getByRole('button', { name: 'Download notes.md' });
  await download.click();
  await expect.poll(() => pendingSaves(page)).toBe(1);
  await expect(download).toBeDisabled();
  await saves(page, 'started', 100);
  await saves(page, 'progress', 40);
  await expect(download).toHaveText('Downloading 40%');
  await saves(page, 'resolve');
  await expect(preview.getByRole('status')).toContainText('Saved to Downloads as notes.md');

  await preview.getByRole('button', { name: 'Close preview' }).click();
  await expect(preview).toHaveCount(0);
  const row = page.locator('[data-native-timeline-event-id]');
  await expect(row.getByRole('status')).toHaveText(/Saved to Downloads as notes\.md/);
  expect(await commandArgs(page, 'matrix_media_save')).toEqual([
    { contentUri: MD_HANDLE, filename: 'notes.md', hasChannel: true },
  ]);
});
