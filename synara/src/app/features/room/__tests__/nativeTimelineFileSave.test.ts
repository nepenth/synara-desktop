import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  nativeTimelineFileDownloadName,
  saveNativeTimelineFileAttachment,
} from '../nativeTimelineFileSave';

const MARKDOWN_HANDLE = `timeline-media-${'ab'.repeat(32)}`;

const withDesktopInvoke = async (
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>,
  run: () => Promise<void>
) => {
  const originalWindow = globalThis.window;
  (globalThis as { window: unknown }).window = {
    __SYNARA_DESKTOP__: {
      platform: 'tauri',
      invoke,
    },
  };
  try {
    await run();
  } finally {
    (globalThis as { window: unknown }).window = originalWindow;
  }
};

test('file download name keeps the Matrix filename including markdown', () => {
  assert.equal(nativeTimelineFileDownloadName('notes.md'), 'notes.md');
  assert.equal(nativeTimelineFileDownloadName('  notes.md  '), 'notes.md');
  assert.equal(nativeTimelineFileDownloadName(''), 'download');
  assert.equal(nativeTimelineFileDownloadName('   '), 'download');
  assert.equal(nativeTimelineFileDownloadName(undefined), 'download');
});

test('markdown file attachments save in Rust and do not return bytes', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  await withDesktopInvoke(
    async (command, args) => {
      calls.push({ command, args });
      if (command === 'matrix_media_save') return { filename: 'notes.md' };
      throw new Error(`unexpected ${command}`);
    },
    async () => {
      await saveNativeTimelineFileAttachment({
        handleId: MARKDOWN_HANDLE,
        filename: 'notes.md',
        mimeType: 'text/markdown',
      });
    }
  );

  assert.deepEqual(
    calls.map((call) => call.command),
    ['matrix_media_save']
  );
  assert.deepEqual(calls[0]?.args, {
    contentUri: MARKDOWN_HANDLE,
    filename: 'notes.md',
  });
});

test('generic file attachments use the same Rust save path', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  await withDesktopInvoke(
    async (command, args) => {
      calls.push({ command, args });
      if (command === 'matrix_media_save') return { filename: 'archive.zip' };
      throw new Error(`unexpected ${command}`);
    },
    async () => {
      await saveNativeTimelineFileAttachment({
        handleId: MARKDOWN_HANDLE,
        filename: 'archive.zip',
        mimeType: 'application/zip',
      });
    }
  );

  assert.deepEqual(calls[0]?.args, {
    contentUri: MARKDOWN_HANDLE,
    filename: 'archive.zip',
  });
});

test('missing handle fails closed before invoking download', async () => {
  const calls: Array<{ command: string }> = [];
  await withDesktopInvoke(
    async (command) => {
      calls.push({ command });
      throw new Error(`unexpected ${command}`);
    },
    async () => {
      await assert.rejects(
        () =>
          saveNativeTimelineFileAttachment({
            handleId: '   ',
            filename: 'notes.md',
            mimeType: 'text/markdown',
          }),
        /File attachment is unavailable/
      );
    }
  );
  assert.equal(calls.length, 0);
});

test('file save owner does not special-case markdown MIME or extension', () => {
  const source = readFileSync('src/app/features/room/nativeTimelineFileSave.ts', 'utf8');
  assert.match(source, /saveMatrixMediaFile/);
  assert.doesNotMatch(source, /matrix_media_download|desktop_save_file|FileSaver/);
  assert.doesNotMatch(source, /if \([^)]*markdown/i);
  assert.doesNotMatch(source, /endsWith\(['"]\.md['"]\)/);
  assert.doesNotMatch(source, /text\/markdown['"]\s*===/);
});
