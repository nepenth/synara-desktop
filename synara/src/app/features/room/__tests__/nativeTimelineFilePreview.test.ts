import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  isNativeTimelineMarkdownAttachment,
  projectNativeTimelineMarkdownPreview,
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

test('markdown preview projects headings through the shared markdown parser', () => {
  const projected = projectNativeTimelineMarkdownPreview(
    '# Agent notes\n\nUse **bold** for emphasis.\n'
  );
  assert.equal(projected.tooLarge, false);
  assert.match(projected.html, /<h1[\s>][\s\S]*Agent notes/);
  assert.match(projected.html, /<strong[\s>][\s\S]*bold/);
  assert.match(projected.plain, /Agent notes/);
});

test('oversized markdown preview fails closed without parsing', () => {
  const projected = projectNativeTimelineMarkdownPreview(
    `# ${'x'.repeat(MAX_NATIVE_MARKDOWN_PREVIEW_BYTES)}`
  );
  assert.equal(projected.tooLarge, true);
  assert.equal(projected.html, '');
});

test('preview owner keeps MIME tables out of the presenter', () => {
  const preview = readFileSync('src/app/features/room/nativeTimelineFilePreview.ts', 'utf8');
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  assert.match(preview, /text\/markdown/);
  assert.match(presenter, /isNativeTimelineMarkdownAttachment/);
  assert.match(presenter, /NativeTimelineMarkdownPreview/);
  assert.doesNotMatch(presenter, /text\/markdown|\.md['"]/);
});
