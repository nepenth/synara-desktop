import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  DEFAULT_THREAD_DISPLAY,
  DEFAULT_THREAD_PANE_WIDTH,
  INLINE_THREAD_INITIAL_REPLIES,
  THREAD_PANE_MAX_WIDTH,
  THREAD_PANE_MIN_WIDTH,
  clampThreadPaneWidth,
  normalizeThreadDisplay,
  visibleInlineReplies,
} from '../../../utils/threadDisplay';
import { resolveThreadPane } from '../threadPane';
import { inlineThreadReplies } from '../nativeInlineThreadModel';
import { defaultSettings, splitSettings } from '../../../state/settings';
import type { NativeTimelineViewRow } from '../nativeTimelineView';

test('thread display defaults to inline and rejects unknown modes', () => {
  assert.equal(DEFAULT_THREAD_DISPLAY, 'inline');
  assert.equal(defaultSettings.threadDisplay, 'inline');
  assert.equal(defaultSettings.threadDisplayChosen, false);
  assert.equal(normalizeThreadDisplay('side'), 'side');
  assert.equal(normalizeThreadDisplay('full'), 'full');
  assert.equal(normalizeThreadDisplay('sideways'), 'inline');
  assert.equal(normalizeThreadDisplay(undefined), 'inline');
  const stored = splitSettings({
    ...defaultSettings,
    threadDisplay: 'bogus' as never,
    threadDisplayChosen: true,
  });
  assert.equal(stored.shared.threadDisplay, 'inline');
});

test('a stored thread display counts only once the user has chosen one', () => {
  // Every save writes the whole snapshot, so 'full' stored by 2.2.1 without a
  // choice is the old default, not a preference.
  const unchosen = splitSettings({ ...defaultSettings, threadDisplay: 'full' });
  assert.equal(unchosen.shared.threadDisplay, 'inline');
  assert.equal(unchosen.shared.threadDisplayChosen, false);

  const chosen = splitSettings({
    ...defaultSettings,
    threadDisplay: 'full',
    threadDisplayChosen: true,
  });
  assert.equal(chosen.shared.threadDisplay, 'full');
  assert.equal(chosen.shared.threadDisplayChosen, true);
});

test('the pane width stays within its bounds and 60% of the room', () => {
  assert.equal(clampThreadPaneWidth(undefined), DEFAULT_THREAD_PANE_WIDTH);
  assert.equal(clampThreadPaneWidth(Number.NaN), DEFAULT_THREAD_PANE_WIDTH);
  assert.equal(clampThreadPaneWidth(10), THREAD_PANE_MIN_WIDTH);
  assert.equal(clampThreadPaneWidth(5000), THREAD_PANE_MAX_WIDTH);
  assert.equal(clampThreadPaneWidth(700, 1000), 600);
  // A narrow room still gets the minimum pane.
  assert.equal(clampThreadPaneWidth(500, 400), THREAD_PANE_MIN_WIDTH);
  const stored = splitSettings({ ...defaultSettings, threadPaneWidth: 9999 });
  assert.equal(stored.shared.threadPaneWidth, THREAD_PANE_MAX_WIDTH);
});

test('the side pane follows the mode, the opened thread and a thread route', () => {
  const opened = { rootEventId: '$root' };
  assert.equal(resolveThreadPane({ mode: 'full', opened, desktop: true }), undefined);
  assert.deepEqual(resolveThreadPane({ mode: 'side', opened, desktop: true }), opened);
  assert.equal(resolveThreadPane({ mode: 'side', opened, desktop: false }), undefined);
  // A notification or link into a thread opens the pane in side mode...
  assert.deepEqual(
    resolveThreadPane({ mode: 'side', routeThreadRootId: '$route', desktop: true }),
    { rootEventId: '$route' }
  );
  // ...but keeps full view in inline mode, where the pane is only for replying.
  assert.equal(
    resolveThreadPane({ mode: 'inline', routeThreadRootId: '$route', desktop: true }),
    undefined
  );
  assert.deepEqual(resolveThreadPane({ mode: 'inline', opened, desktop: true }), opened);
});

test('inline threads show the latest replies first and reveal earlier ones', () => {
  const replies = Array.from({ length: 12 }, (_, index) => index + 1);
  const initial = visibleInlineReplies(replies, INLINE_THREAD_INITIAL_REPLIES);
  assert.deepEqual(initial.shown, [8, 9, 10, 11, 12]);
  assert.equal(initial.hidden, 7);
  assert.deepEqual(visibleInlineReplies(replies, 50), { shown: replies, hidden: 0 });
  assert.deepEqual(visibleInlineReplies([], 5), { shown: [], hidden: 0 });
});

const message = (eventId: string, body: string): NativeTimelineViewRow =>
  ({
    kind: 'message',
    itemId: eventId,
    eventId,
    senderId: '@a:example.test',
    senderName: 'Alice',
    originServerTs: 1,
    body,
    edited: false,
    capabilities: {},
  }) as unknown as NativeTimelineViewRow;

test('inline replies drop the root and non-message rows', () => {
  const rows = [
    message('$root', 'Question'),
    message('$r1', 'First answer'),
    { kind: 'day_divider', itemId: 'd1' } as unknown as NativeTimelineViewRow,
    message('$r2', 'Second answer'),
  ];
  assert.deepEqual(
    inlineThreadReplies(rows, '$root').map((reply) => reply.body),
    ['First answer', 'Second answer']
  );
});

test('inline replies carry Matrix HTML for text, never for media captions (D27)', () => {
  const formatted = {
    ...message('$r1', 'Plain'),
    formattedBody: '<strong>Plain</strong>',
  } as unknown as NativeTimelineViewRow;
  const media = {
    ...message('$r2', 'photo.png'),
    formattedBody: '<em>caption</em>',
    messageType: 'image',
    media: { handleId: 'timeline-media-x' },
  } as unknown as NativeTimelineViewRow;
  assert.deepEqual(
    inlineThreadReplies([message('$root', 'Q'), formatted, media], '$root').map(
      (reply) => reply.formattedBody
    ),
    ['<strong>Plain</strong>', undefined]
  );
});

test('inline threads retry, surface pagination errors and cache replies across remounts (D27)', () => {
  const source = readFileSync('src/app/features/room/NativeInlineThread.tsx', 'utf8');
  // Open failures offer Retry through the controller's reopen.
  assert.match(source, /action=\{\{ label: 'Retry', onClick: reload \}\}/);
  // Earlier-reply failures are caught and shown, never left unhandled.
  assert.match(source, /paginate\('backwards'\)[\s\S]*\.catch\(\(\) => setEarlierError\(true\)\)/);
  assert.doesNotMatch(source, /void paginate\(/);
  assert.match(source, /<NativeTimelineNotice/);
  // Replies render through the timeline's formatted renderer.
  assert.match(source, /<NativeFormattedBody html=\{reply\.formattedBody\}/);
  // A remounted row starts from the presenter-owned cache.
  assert.match(source, /useState\(\(\) => replyCache\?\.get\(rootEventId\)\)/);
  assert.match(source, /const rows = snapshot\?\.rows \?\? cached\?\.rows;/);
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  assert.match(presenter, /replyCache: inlineExpansion\.replyCache/);
});

test('timeline images and videos open the viewer, save, and fall back to the file chip (D14)', () => {
  const presenter = readFileSync('src/app/features/room/NativeTimelinePresenter.tsx', 'utf8');
  const viewer = readFileSync('src/app/features/room/NativeTimelineMediaViewer.tsx', 'utf8');
  const media = presenter.slice(
    presenter.indexOf('const NativeTimelineMedia = '),
    presenter.indexOf('const NativeTimelineSenderAvatar')
  );
  assert.match(media, /<img[\s\S]*onError=\{markLoadFailed\}/);
  assert.match(media, /<video[\s\S]*onError=\{markLoadFailed\}/);
  assert.match(media, /visualKind && media && loadFailed[\s\S]*<NativeTimelineFileChip/);
  // Auto-load off keeps the click-to-load placeholder ahead of the viewer.
  assert.ok(media.indexOf('if (deferVisualMedia && media)') < media.indexOf('const openViewer'));
  // The viewer shows the original handle, saves through the file-save path,
  // and is a modal dialog that returns focus to its opener.
  assert.match(viewer, /nativeTimelineMediaSrc\(\{ handleId: target\.handleId/);
  assert.match(viewer, /saveNativeTimelineFileAttachment\(/);
  assert.match(viewer, /import \{ Modal \} from '\.\.\/\.\.\/components\/modal-dialog'/);
  assert.match(viewer, /escapeDeactivates: stopPropagation/);
  assert.match(viewer, /setReturnFocus: \(previous\) => viewerReturnFocus\(target\) \?\? previous/);
  assert.doesNotMatch(viewer, /nativeThumbnailContentUri|nativeTimelineInlineImageSrc/);
});
