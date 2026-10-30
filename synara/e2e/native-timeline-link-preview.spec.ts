import { expect, test, type Page } from '@playwright/test';

type Fixture = { commands: { command: string; args?: Record<string, unknown> }[] };

const commandCalls = (page: Page, command: string) =>
  page.evaluate(
    (name) =>
      (window as unknown as { nativeTimelineFixture: Fixture }).nativeTimelineFixture.commands
        .filter((entry) => entry.command === name)
        .map((entry) => entry.args?.url),
    command
  );

test('the first link previews from this device without a known room encryption', async ({
  page,
}) => {
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=link-preview');
  const card = page.getByRole('link', { name: /An article worth reading/ });
  await expect(card).toBeVisible();
  await expect(card).toContainText('Example Press');
  await expect(card).toHaveAttribute('href', 'https://example.org/article');
  await expect(card).toHaveCount(1);
  // Only the message's first link is fetched; nothing asks the homeserver.
  const calls = await commandCalls(page, 'desktop_link_preview');
  expect(calls).toContain('https://example.org/article');
  expect(calls).not.toContain('https://example.org/second');
  expect(await commandCalls(page, 'matrix_media_preview')).toEqual([]);
});

test('with Link previews off nothing is fetched or shown', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      'settings',
      JSON.stringify({ linkPreviews: false, previewDefaultsVersion: 3 })
    );
  });
  await page.goto('/e2e/native-timeline-harness/index.html?scenario=link-preview');
  await expect(page.getByRole('link', { name: 'https://example.org/second' })).toBeVisible();
  await expect(page.getByRole('link', { name: /An article worth reading/ })).toHaveCount(0);
  expect(await commandCalls(page, 'desktop_link_preview')).toEqual([]);
});
