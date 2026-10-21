import { expect, test, type Page } from '@playwright/test';

const IMAGE_HANDLE = `timeline-media-${'1a'.repeat(32)}`;
const VIDEO_HANDLE = `timeline-media-${'2b'.repeat(32)}`;
const BROKEN_HANDLE = `timeline-media-${'3c'.repeat(32)}`;
const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=',
  'base64'
);

/** Core's media protocol: the image loads, the broken image 404s, the video never answers. */
const routeMedia = async (page: Page) => {
  await page.route('**/__synara-media/**', async (route) => {
    const handle = decodeURIComponent(new URL(route.request().url()).pathname.split('/').pop()!);
    if (handle === IMAGE_HANDLE) {
      await route.fulfill({ status: 200, contentType: 'image/png', body: PNG });
      return;
    }
    if (handle === BROKEN_HANDLE) {
      await route.fulfill({ status: 404, body: '' });
      return;
    }
    // The video stays loading, so it neither plays nor fails.
  });
};

const openMedia = async (page: Page, query = '') => {
  await routeMedia(page);
  await page.goto(`/e2e/native-timeline-harness/index.html?scenario=media${query}`);
  await expect(page.locator('[data-native-timeline-event-id]')).toHaveCount(3);
};

const saveCommands = (page: Page) =>
  page.evaluate(() =>
    (
      window as unknown as {
        nativeTimelineFixture: {
          commands: { command: string; args?: Record<string, unknown> }[];
        };
      }
    ).nativeTimelineFixture.commands
      .filter((entry) => entry.command === 'matrix_media_save')
      .map((entry) => entry.args)
  );

test('an image opens in the viewer at full size, saves, and closes on Escape back to the image', async ({
  page,
}) => {
  await openMedia(page);
  const opener = page.getByRole('button', { name: 'Open image: sunrise.png' });
  await expect(opener).toBeVisible();
  await opener.focus();
  await opener.press('Enter');

  const viewer = page.getByRole('dialog', { name: 'View image: sunrise.png' });
  await expect(viewer).toBeVisible();
  // The original through the row's handle, not a thumbnail URI.
  await expect(viewer.locator('img')).toHaveAttribute(
    'src',
    new RegExp(`/__synara-media/${IMAGE_HANDLE}$`)
  );

  await viewer.getByRole('button', { name: 'Save sunrise.png' }).click();
  await expect
    .poll(() => saveCommands(page))
    .toEqual([{ contentUri: IMAGE_HANDLE, filename: 'sunrise.png' }]);

  await page.keyboard.press('Escape');
  await expect(viewer).toHaveCount(0);
  await expect(opener).toBeFocused();
});

test('a mouse click opens the viewer, and its close button returns focus to the image', async ({
  page,
}) => {
  await openMedia(page);
  const opener = page.getByRole('button', { name: 'Open image: sunrise.png' });
  await opener.click();
  const viewer = page.getByRole('dialog', { name: 'View image: sunrise.png' });
  await expect(viewer).toBeVisible();
  await viewer.getByRole('button', { name: 'Close viewer' }).click();
  await expect(viewer).toHaveCount(0);
  await expect(opener).toBeFocused();
});

test('a video keeps its own controls and opens the viewer from its corner button', async ({
  page,
}) => {
  await openMedia(page);
  await expect(page.locator('#native-timeline video')).toHaveCount(1);
  await page.getByRole('button', { name: 'Open video: clip.mp4' }).click();
  const viewer = page.getByRole('dialog', { name: 'View video: clip.mp4' });
  await expect(viewer).toBeVisible();
  await expect(viewer.locator('video')).toHaveAttribute(
    'src',
    new RegExp(`/__synara-media/${VIDEO_HANDLE}$`)
  );
  await viewer.getByRole('button', { name: 'Save clip.mp4' }).click();
  await expect
    .poll(() => saveCommands(page))
    .toEqual([{ contentUri: VIDEO_HANDLE, filename: 'clip.mp4' }]);
  await page.keyboard.press('Escape');
  await expect(viewer).toHaveCount(0);
});

test('media the webview cannot load falls back to the file chip', async ({ page }) => {
  await openMedia(page);
  const failed = page.locator('[data-native-timeline-media-failed="image"]');
  await expect(failed).toBeVisible();
  await expect(failed.getByText('This image could not be shown here.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open image: broken.png' })).toHaveCount(0);

  await failed.getByRole('button', { name: 'Download broken.png' }).click();
  await expect
    .poll(() => saveCommands(page))
    .toEqual([{ contentUri: BROKEN_HANDLE, filename: 'broken.png' }]);
});

test('with automatic media loading off, an image is loaded before it can be opened', async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem('settings', JSON.stringify({ mediaAutoLoad: false }))
  );
  await openMedia(page);
  await expect(page.getByRole('button', { name: 'Open image: sunrise.png' })).toHaveCount(0);
  await expect(page.getByRole('dialog')).toHaveCount(0);

  await page.getByRole('button', { name: 'Load image: sunrise.png' }).click();
  await page.getByRole('button', { name: 'Open image: sunrise.png' }).click();
  await expect(page.getByRole('dialog', { name: 'View image: sunrise.png' })).toBeVisible();
});
