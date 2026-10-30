import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import type { DesktopInvokeResult } from '../../../utils/desktop';
import {
  COMPOSER_UNFURL_DEBOUNCE_MS,
  fetchDeviceLinkPreview,
  firstTimelinePreviewUrl,
  isPreviewCandidateUrl,
  linkPreviewImagePath,
  linkPreviewSource,
  parseLinkPreview,
  shouldSkipMessageUnfurl,
  timelinePreviewUrl,
  trailingComposerPreviewUrl,
  type NativeInvoke,
} from '../nativeLinkUnfurl';

const okPreview = {
  url: 'https://example.org/x',
  title: 'Example',
  description: 'Hello',
  siteName: 'Example Site',
  imageHandleId: 'ab'.repeat(32),
};

const recordingInvoke = (value: unknown = okPreview) => {
  const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
  const invoke: NativeInvoke = async (command, args) => {
    calls.push({ command, args });
    if (command === 'desktop_link_preview') {
      return { available: true, value } satisfies DesktopInvokeResult<unknown>;
    }
    throw new Error(`unexpected ${command}`);
  };
  return { calls, invoke };
};

test('preview URL candidates reject javascript, credentials, and mxc', () => {
  assert.equal(isPreviewCandidateUrl('https://example.org/x'), true);
  assert.equal(isPreviewCandidateUrl('javascript:alert(1)'), false);
  assert.equal(isPreviewCandidateUrl('https://user:pass@example.org/'), false);
  assert.equal(isPreviewCandidateUrl('mxc://example.org/aa'), false);
  assert.equal(isPreviewCandidateUrl('mailto:user@example.org'), false);
  assert.equal(isPreviewCandidateUrl('file:///etc/passwd'), false);
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
    firstTimelinePreviewUrl('see https://example.org/a and https://example.org/b'),
    'https://example.org/a'
  );
  assert.equal(
    firstTimelinePreviewUrl('ignored', '<p>go <a href="https://example.org/x">here</a></p>'),
    'https://example.org/x'
  );
  assert.equal(shouldSkipMessageUnfurl({ messageType: 'image' }), true);
  assert.equal(shouldSkipMessageUnfurl({ hasMedia: true }), true);
  assert.equal(shouldSkipMessageUnfurl({ messageType: 'text' }), false);
});

test('a row previews only its first link, only with the setting on and no other card', () => {
  const body = 'read https://example.org/a then https://example.org/b';
  assert.equal(timelinePreviewUrl({ enabled: true, body }), 'https://example.org/a');
  assert.equal(timelinePreviewUrl({ enabled: false, body }), undefined);
  // An X card, inline GIF, approval or agent card sets `skip`.
  assert.equal(timelinePreviewUrl({ enabled: true, body, skip: true }), undefined);
  for (const messageType of ['image', 'file', 'audio', 'video']) {
    assert.equal(timelinePreviewUrl({ enabled: true, body, messageType }), undefined);
  }
  assert.equal(timelinePreviewUrl({ enabled: true, body, hasMedia: true }), undefined);
  assert.equal(timelinePreviewUrl({ enabled: true, body: 'no links here' }), undefined);
});

test('the Link previews setting gates the device fetch', async () => {
  const off = recordingInvoke();
  assert.equal(
    await fetchDeviceLinkPreview({
      url: 'https://example.org/x',
      enabled: false,
      desktopAvailable: true,
      invoke: off.invoke,
    }),
    null
  );
  assert.deepEqual(off.calls, []);

  const outsideDesktop = recordingInvoke();
  assert.equal(
    await fetchDeviceLinkPreview({
      url: 'https://example.org/x',
      enabled: true,
      desktopAvailable: false,
      invoke: outsideDesktop.invoke,
    }),
    null
  );
  assert.deepEqual(outsideDesktop.calls, []);

  const on = recordingInvoke();
  const preview = await fetchDeviceLinkPreview({
    url: 'https://example.org/x',
    enabled: true,
    desktopAvailable: true,
    invoke: on.invoke,
  });
  assert.equal(preview?.title, 'Example');
  assert.deepEqual(on.calls, [
    { command: 'desktop_link_preview', args: { url: 'https://example.org/x' } },
  ]);
});

test('encrypted rooms preview: the device fetch never asks the homeserver', async () => {
  const { calls, invoke } = recordingInvoke();
  await fetchDeviceLinkPreview({
    url: 'https://example.org/x',
    enabled: true,
    desktopAvailable: true,
    invoke,
  });
  assert.deepEqual(
    calls.map((call) => call.command),
    ['desktop_link_preview']
  );
  const card = readFileSync('src/app/features/room/nativeLinkUnfurlCard.tsx', 'utf8');
  const unfurl = readFileSync('src/app/features/room/nativeLinkUnfurl.ts', 'utf8');
  const input = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  for (const source of [card, unfurl, input]) {
    assert.doesNotMatch(source, /matrix_media_preview/);
    assert.doesNotMatch(source, /not_encrypted|encryptionStatus/);
  }
  assert.match(card, /useSetting\(settingsAtom, 'linkPreviews'\)/);
  assert.match(input, /useSetting\(settingsAtom, 'linkPreviews'\)/);
});

test('preview parser accepts only a preview of the requested URL', () => {
  assert.equal(parseLinkPreview(okPreview, 'https://example.org/x')?.title, 'Example');
  assert.equal(parseLinkPreview(okPreview, 'https://example.org/other'), null);
  assert.equal(parseLinkPreview({ url: 'https://example.org/x' }, 'https://example.org/x'), null);
  assert.equal(parseLinkPreview({ ...okPreview, title: '' }, 'https://example.org/x'), null);
  assert.equal(
    parseLinkPreview({ ...okPreview, imageHandleId: '../secret' }, 'https://example.org/x'),
    null
  );
  assert.equal(
    parseLinkPreview(
      { ...okPreview, image: 'https://evil.example/a.png' },
      'https://example.org/x'
    ),
    null
  );
  assert.equal(parseLinkPreview(null, 'https://example.org/x'), null);
  assert.equal(linkPreviewImagePath('ab'.repeat(32)), `link-preview/${'ab'.repeat(32)}`);
});

test('composer and timeline wire device previews', () => {
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  const input = readFileSync('src/app/features/room/RoomInput.tsx', 'utf8');
  assert.match(presenter, /NativeTimelineLinkUnfurl/);
  assert.match(presenter, /NativePlainMessageBody/);
  assert.match(
    presenter,
    /skip=\{Boolean\(approvalPrompt \|\| agentPayload \|\| inlineGif \|\| xPostUrl\)\}/
  );
  assert.doesNotMatch(presenter, /mxc:\/\//);
  assert.match(input, /COMPOSER_UNFURL_DEBOUNCE_MS/);
  assert.match(input, /trailingComposerPreviewUrl/);
  assert.match(input, /fetchDeviceLinkPreview/);
});

test('the card names the host first, so a page cannot pose as another site', () => {
  assert.equal(
    linkPreviewSource('https://www.Example.org/x', 'Example Site'),
    'example.org · Example Site'
  );
  assert.equal(linkPreviewSource('https://evil.example/login', 'Apple'), 'evil.example · Apple');
  assert.equal(linkPreviewSource('https://example.org/x', 'Example.org'), 'example.org');
  assert.equal(linkPreviewSource('https://example.org/x', '  '), 'example.org');
  assert.equal(linkPreviewSource('https://example.org/x'), 'example.org');
});
