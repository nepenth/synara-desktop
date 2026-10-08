import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  isNativeTimelineMarkdownAttachment,
  renderNativeTimelineMarkdownPreview,
  MAX_NATIVE_MARKDOWN_PREVIEW_BYTES,
} from '../nativeTimelineFilePreview';

test('markdown attachments are recognized from declared MIME or filename', () => {
  assert.equal(
    isNativeTimelineMarkdownAttachment({
      filename: 'notes.md',
      mimeType: 'text/markdown',
    }),
    true
  );
  assert.equal(
    isNativeTimelineMarkdownAttachment({
      filename: 'NOTES.MD',
      mimeType: 'application/octet-stream',
    }),
    true
  );
  assert.equal(
    isNativeTimelineMarkdownAttachment({
      filename: 'readme.markdown',
      mimeType: undefined,
    }),
    true
  );
  assert.equal(
    isNativeTimelineMarkdownAttachment({ filename: 'notes.txt', mimeType: 'text/markdown' }),
    true
  );
});

test('non-markdown files do not open the markdown preview route', () => {
  assert.equal(
    isNativeTimelineMarkdownAttachment({
      filename: 'archive.zip',
      mimeType: 'application/zip',
    }),
    false
  );
  assert.equal(
    isNativeTimelineMarkdownAttachment({ filename: 'data.csv', mimeType: 'text/csv' }),
    false
  );
  assert.equal(
    isNativeTimelineMarkdownAttachment({ filename: 'notes.txt', mimeType: 'text/plain' }),
    false
  );
  assert.equal(isNativeTimelineMarkdownAttachment({}), false);
});

// Markdown rendering itself is Core's (`render_composer_markdown` tests in Rust).

test('markdown preview renders through Core with no fragments', async () => {
  const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
  const projected = await renderNativeTimelineMarkdownPreview(
    '# Agent notes',
    async (command, args) => {
      calls.push({ command, args });
      return { available: true, value: '<h1>Agent notes</h1>' };
    }
  );
  assert.deepEqual(calls, [
    { command: 'desktop_render_markdown', args: { source: '# Agent notes', fragments: [] } },
  ]);
  assert.deepEqual(projected, {
    html: '<h1>Agent notes</h1>',
    plain: '# Agent notes',
    tooLarge: false,
  });
});

test('plain text, an unavailable bridge or a failed render shows the plain text', async () => {
  const plain = await renderNativeTimelineMarkdownPreview('just text', async () => ({
    available: true,
    value: null,
  }));
  assert.equal(plain.html, '');
  const unavailable = await renderNativeTimelineMarkdownPreview('x', async () => ({
    available: false,
  }));
  assert.equal(unavailable.html, '');
  const failed = await renderNativeTimelineMarkdownPreview('x', async () => {
    throw new Error('boom');
  });
  assert.equal(failed.html, '');
  assert.equal(failed.plain, 'x');
});

test('oversized markdown preview fails closed without calling Core', async () => {
  let called = false;
  const projected = await renderNativeTimelineMarkdownPreview(
    `# ${'x'.repeat(MAX_NATIVE_MARKDOWN_PREVIEW_BYTES)}`,
    async () => {
      called = true;
      return { available: true, value: '<p>x</p>' };
    }
  );
  assert.equal(projected.tooLarge, true);
  assert.equal(projected.html, '');
  assert.equal(called, false);
});

test('preview owner keeps MIME tables out of the presenter', () => {
  const preview = readFileSync('src/app/features/room/nativeTimelineFilePreview.ts', 'utf8');
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  assert.match(preview, /text\/markdown/);
  assert.match(presenter, /isNativeTimelineMarkdownAttachment/);
  assert.match(presenter, /NativeTimelineMarkdownPreview/);
  assert.doesNotMatch(presenter, /text\/markdown|\.md['"]/);
  const previewUi = readFileSync('src/app/features/room/NativeTimelineMarkdownPreview.tsx', 'utf8');
  assert.equal([...previewUi.matchAll(/data-native-timeline-file-preview="true"/g)].length, 1);
  assert.match(previewUi, /variant="Surface"/);
  assert.match(previewUi, /Copy markdown/);
  assert.match(previewUi, /copyToClipboard/);
});
