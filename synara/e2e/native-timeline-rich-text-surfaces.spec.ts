import { expect, test, type Page } from '@playwright/test';

// Inbox, pins, search and room topics draw message text with the timeline's
// renderer (D84): one sanitizer, in-app permalinks, Core-loaded custom emoji.

const PNG = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=',
  'base64'
);

const openSurfaces = async (page: Page) => {
  const requests: string[] = [];
  page.on('request', (request) => requests.push(request.url()));
  await page.route('**/__synara-media/**', (route) =>
    route.fulfill({ status: 200, contentType: 'image/png', body: PNG })
  );
  await page.goto('/e2e/native-timeline-harness/index.html?surface=rich-text');
  await expect(page.getByRole('region', { name: 'Inbox message' })).toBeVisible();
  return requests;
};

const opened = async (page: Page) =>
  JSON.parse((await page.getByTestId('opened-permalink').textContent()) || 'null');

test('inbox text renders mentions, permalinks and custom emoji without row context', async ({
  page,
}) => {
  const requests = await openSurfaces(page);
  const inbox = page.getByRole('region', { name: 'Inbox message' });

  await expect(inbox.locator('strong')).toHaveText('bold');
  const mention = inbox.getByRole('link', { name: 'Reader Zero' });
  await expect(mention).toHaveAttribute('data-synara-in-app-link', '');
  await mention.click();
  expect(await opened(page)).toEqual({ kind: 'user', userId: '@reader0:example.test' });

  await inbox.getByRole('link', { name: 'this message' }).click();
  expect(await opened(page)).toMatchObject({
    kind: 'room',
    roomIdOrAlias: '!room:example.test',
    eventId: '$event',
  });

  // Custom emoji load through Core's media protocol as a small thumbnail.
  const emoji = inbox.locator('img[data-mx-emoticon]');
  await expect(emoji).toHaveAttribute('alt', ':party:');
  await expect(emoji).toHaveAttribute('src', /\/__synara-media\/.*example\.test.*party/);
  expect(requests.some((url) => url.startsWith('mxc:'))).toBe(false);
});

test('surface text keeps the native sanitizer: no remote images, no scripts', async ({ page }) => {
  const requests = await openSurfaces(page);
  const inbox = page.getByRole('region', { name: 'Inbox message' });
  // A remote image keeps only its alt text, on an inert span.
  await expect(inbox.locator('span', { hasText: /^tracker$/ })).toBeVisible();
  await expect(inbox.locator('img:not([data-mx-emoticon])')).toHaveCount(0);
  expect(requests.some((url) => url.includes('tracker.example'))).toBe(false);
  expect(await page.evaluate(() => 'surfaceScriptRan' in window)).toBe(false);
  await expect(inbox.locator('script')).toHaveCount(0);
});

test('search results mark the terms in formatted and plain text, but not inside code', async ({
  page,
}) => {
  await openSurfaces(page);
  const search = page.getByRole('region', { name: 'Search result' });
  const marks = search.locator('span', { hasText: /^needle$/ });
  // One in the formatted result, one in the plain one; `needle()` stays code.
  await expect(marks).toHaveCount(2);
  await expect(search.locator('code')).toHaveText('needle()');
  await expect(search.getByRole('link', { name: 'https://example.org' })).toHaveAttribute(
    'target',
    '_blank'
  );
});

test('plain notices link web addresses and open matrix.to links in Synara', async ({ page }) => {
  await openSurfaces(page);
  const notice = page.getByRole('region', { name: 'Pinned notice' });
  await expect(notice.getByRole('link', { name: 'https://example.org/docs' })).toHaveAttribute(
    'title',
    'https://example.org/docs'
  );
  // The desktop opener cannot open ftp, so it stays text.
  await expect(notice.getByText(/ftp:\/\/files\.example/)).toBeVisible();
  await expect(notice.getByRole('link', { name: /ftp:/ })).toHaveCount(0);

  await notice.getByRole('link', { name: 'https://matrix.to/#/@reader1:example.test' }).click();
  expect(await opened(page)).toEqual({ kind: 'user', userId: '@reader1:example.test' });

  const emote = page.getByRole('region', { name: 'Emote' });
  await expect(emote).toContainText('Reader One waves');

  const plain = page.getByRole('region', { name: 'Plain message body' });
  await expect(plain.getByRole('link', { name: 'https://example.org/article' })).toBeVisible();
});

test('bare web addresses in formatted messages and replies are links too', async ({ page }) => {
  await openSurfaces(page);
  const formatted = page.getByRole('region', { name: 'Formatted message body' });
  await expect(formatted.getByRole('link', { name: 'https://example.org/guide' })).toBeVisible();
  // An existing link keeps its own text and target, with nothing nested inside.
  await expect(formatted.getByRole('link', { name: 'this one' })).toHaveCount(1);
  // Inline code stays literal.
  await expect(formatted.getByRole('link', { name: /example\.org\/code/ })).toHaveCount(0);
  await expect(formatted.locator('code')).toHaveText('https://example.org/code');
});

test('the room topic viewer links the topic with the same renderer', async ({ page }) => {
  await openSurfaces(page);
  await page.getByRole('button', { name: 'Open topic' }).click();
  const topic = page.getByRole('dialog', { name: 'Topic room' });
  await expect(topic.getByRole('link', { name: 'https://example.org/topic' })).toBeVisible();
  await topic.getByRole('link', { name: 'https://matrix.to/#/#lobby:example.test' }).click();
  expect(await opened(page)).toMatchObject({ kind: 'room', roomIdOrAlias: '#lobby:example.test' });
});
