import assert from 'node:assert/strict';
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

test('thread display defaults to full view and rejects unknown modes', () => {
  assert.equal(DEFAULT_THREAD_DISPLAY, 'full');
  assert.equal(defaultSettings.threadDisplay, 'full');
  assert.equal(normalizeThreadDisplay('side'), 'side');
  assert.equal(normalizeThreadDisplay('inline'), 'inline');
  assert.equal(normalizeThreadDisplay('sideways'), 'full');
  assert.equal(normalizeThreadDisplay(undefined), 'full');
  const stored = splitSettings({ ...defaultSettings, threadDisplay: 'bogus' as never });
  assert.equal(stored.shared.threadDisplay, 'full');
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
