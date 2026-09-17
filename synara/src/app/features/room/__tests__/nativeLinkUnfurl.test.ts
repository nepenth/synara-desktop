import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../utils/desktop';
import {
  COMPOSER_UNFURL_DEBOUNCE_MS,
  fetchMediaPreviewWithNativeOwner,
  firstTimelinePreviewUrl,
  isPreviewCandidateUrl,
  parseMediaPreviewSnapshot,
  shouldSkipMessageUnfurl,
  trailingComposerPreviewUrl,
  type NativeInvoke,
} from '../nativeLinkUnfurl';

const session = {
  status: 'logged_in',
  user_id: '@alice:example.org',
  device_id: 'DEVICE',
  homeserver_url: 'https://matrix.example.org',
  sessionGeneration: 7,
};

const okPreview = {
  status: 'ok',
  roomId: '!room:example.org',
  sessionGeneration: 7,
  url: 'https://example.org/x',
  title: 'Example',
  description: 'Hello',
  siteName: 'Example Site',
  thumbnailHandleId: 'ab'.repeat(32),
};

test('preview URL candidates reject javascript, credentials, and mxc', () => {
  assert.equal(isPreviewCandidateUrl('https://example.org/x'), true);
  assert.equal(isPreviewCandidateUrl('javascript:alert(1)'), false);
  assert.equal(isPreviewCandidateUrl('https://user:pass@example.org/'), false);
  assert.equal(isPreviewCandidateUrl('mxc://example.org/aa'), false);
  assert.equal(isPreviewCandidateUrl('mailto:user@example.org'), false);
});

test('composer trailing URL requires a single URL at the end', () => {
  assert.equal(trailingComposerPreviewUrl('https://example.org/x'), 'https://example.org/x');
  assert.equal(
    trailingComposerPreviewUrl('check this https://example.org/x'),
    'https://example.org/x'
  );
  assert.equal(trailingComposerPreviewUrl('https://example.org/x and more'), undefined);
  assert.equal(
    trailingComposerPreviewUrl('https://example.org/a https://example.org/b'),
    undefined
  );
  assert.equal(COMPOSER_UNFURL_DEBOUNCE_MS, 400);
});

test('timeline preview uses the first formatted or plain URL and skips attachments', () => {
  assert.equal(
    firstTimelinePreviewUrl('see https://example.org/x please'),
    'https://example.org/x'
  );
  assert.equal(
    firstTimelinePreviewUrl('ignored', '<p>go <a href="https://example.org/x">here</a></p>'),
    'https://example.org/x'
  );
  assert.equal(shouldSkipMessageUnfurl({ messageType: 'image' }), true);
  assert.equal(shouldSkipMessageUnfurl({ hasMedia: true }), true);
  assert.equal(shouldSkipMessageUnfurl({ messageType: 'text' }), false);
});

test('preview DTO parser rejects MXC and raw OpenGraph image fields', () => {
  assert.equal(
    parseMediaPreviewSnapshot(okPreview, '!room:example.org', 7, 'https://example.org/x')?.title,
    'Example'
  );
  assert.equal(
    parseMediaPreviewSnapshot(
      { ...okPreview, status: 'skipped', title: undefined, thumbnailHandleId: undefined },
      '!room:example.org',
      7,
      'https://example.org/x'
    ),
    null
  );
  assert.throws(() =>
    parseMediaPreviewSnapshot(
      { ...okPreview, imageMxc: 'mxc://example.org/thumb' },
      '!room:example.org',
      7,
      'https://example.org/x'
    )
  );
});

test('preview owner skips encrypted rooms and never invokes matrix_media_preview', async () => {
  const calls: string[] = [];
  const invoke: NativeInvoke = async (command) => {
    calls.push(command);
    throw new Error(`unexpected ${command}`);
  };
  const result = await fetchMediaPreviewWithNativeOwner({
    roomId: '!room:example.org',
    url: 'https://example.org/x',
    encryptionStatus: 'encrypted',
    desktopAvailable: true,
    invoke,
  });
  assert.equal(result, null);
  assert.deepEqual(calls, []);
});

test('preview owner invokes matrix_media_preview for unencrypted rooms', async () => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'matrix_session_snapshot') {
      return { available: true, value: session } satisfies DesktopInvokeResult<unknown>;
    }
    if (command === 'matrix_media_preview') {
      return { available: true, value: okPreview } satisfies DesktopInvokeResult<unknown>;
    }
    throw new Error(`unexpected ${command}`);
  };
  const result = await fetchMediaPreviewWithNativeOwner({
    roomId: '!room:example.org',
    url: 'https://example.org/x',
    encryptionStatus: 'not_encrypted',
    desktopAvailable: true,
    invoke,
  });
  assert.equal(result?.title, 'Example');
  assert.deepEqual(calls[1], {
    command: 'matrix_media_preview',
    args: {
      roomId: '!room:example.org',
      sessionGeneration: 7,
      url: 'https://example.org/x',
    },
  });
});

test('composer and timeline wire unfurls without sending MXC to the presenter', () => {
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  const input = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  const live = readFileSync('../crates/synara-core/src/app/room_profile/live.rs', 'utf8');
  assert.match(presenter, /NativeTimelineLinkUnfurl/);
  assert.match(presenter, /NativePlainMessageBody/);
  assert.doesNotMatch(presenter, /mxc:\/\//);
  assert.match(input, /COMPOSER_UNFURL_DEBOUNCE_MS/);
  assert.match(input, /trailingComposerPreviewUrl/);
  const method = live.split('pub async fn get_media_preview').at(1) ?? '';
  assert.match(method, /encryption_state\(\)/);
  assert.match(method, /room_media_preview/);
});
