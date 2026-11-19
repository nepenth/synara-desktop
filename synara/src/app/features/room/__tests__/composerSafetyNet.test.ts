import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import type { Descendant } from 'slate';
import {
  COMPOSER_RECOVERY_WINDOW_MS,
  composerDraftText,
  composerErrorDetail,
  composerFailed,
  composerStartFresh,
  composerTryAgain,
  initialComposerSafetyNetState,
  isRestorableDraft,
  snapshotComposerDraft,
  type ComposerSafetyNetState,
} from '../composerSafetyNetPolicy';

const read = (path: string) => readFileSync(path, 'utf8');

const draft: Descendant[] = [
  {
    type: 'paragraph',
    children: [
      { text: 'Hi ' },
      {
        type: 'mention',
        id: '@fiona:example.org',
        name: '@Fiona',
        highlight: false,
        children: [{ text: '' }],
      },
      { text: ' see ' },
      { type: 'link', href: 'https://example.org', children: [{ text: 'this' }] },
      { type: 'emoticon', key: '😀', shortcode: 'grinning', children: [{ text: '' }] },
      { text: '' },
    ],
  },
  { type: 'paragraph', children: [{ text: 'second line', bold: true }] },
] as Descendant[];

const fail = (state: ComposerSafetyNetState, at: number, value: unknown = draft) =>
  composerFailed(state, at, () => snapshotComposerDraft(value));

test('a first failure remounts a fresh editor with the rich draft and no fallback', () => {
  const next = fail(initialComposerSafetyNetState(), 1_000);
  assert.equal(next.status, 'live');
  assert.equal(next.generation, 1);
  assert.equal(next.restore?.mode, 'rich');
  assert.deepEqual(next.restore?.nodes, draft);
  // A copy: the failed editor's value is not shared with the new one.
  assert.notEqual(next.restore?.nodes, draft);
  assert.equal(next.remountedAt, 1_000);
});

test('a second failure within the window shows the fallback and keeps the draft', () => {
  const recovered = fail(initialComposerSafetyNetState(), 1_000);
  // The remounted editor fails while restoring: what it holds is not trusted.
  const next = fail(recovered, 1_000 + COMPOSER_RECOVERY_WINDOW_MS - 1, [
    { type: 'paragraph', children: [{ text: 'Hi' }] },
  ]);
  assert.equal(next.status, 'fallback');
  assert.equal(next.generation, recovered.generation);
  assert.equal(next.plainOnly, true);
  assert.equal(next.snapshot.text, recovered.snapshot.text);
  assert.equal(next.snapshot.text, 'Hi @Fiona see this😀\nsecond line');
});

test('more errors while the fallback shows change nothing', () => {
  const fallback = fail(fail(initialComposerSafetyNetState(), 0), 10);
  assert.equal(fail(fallback, 20), fallback);
});

test('Try again remounts with the draft, as text once its rich form has failed', () => {
  const fallback = fail(fail(initialComposerSafetyNetState(), 0), 10);
  const again = composerTryAgain(fallback, 50);
  assert.equal(again.status, 'live');
  assert.equal(again.generation, fallback.generation + 1);
  assert.equal(again.restore?.mode, 'plain');
  assert.deepEqual(again.restore?.nodes, [
    { type: 'paragraph', children: [{ text: 'Hi @Fiona see this😀' }] },
    { type: 'paragraph', children: [{ text: 'second line' }] },
  ]);
  // And if that fails at once too, the fallback comes back instead of a loop.
  const again2 = fail(again, 60);
  assert.equal(again2.status, 'fallback');
  assert.equal(composerTryAgain(again2, 70).restore?.mode, 'plain');
});

test('Try again keeps rich content when the rich draft was not what failed', () => {
  // The second failure came after a plain restore, not a rich one.
  let state = fail(initialComposerSafetyNetState(), 0, [
    { type: 'paragraph', children: [{ text: 'only text' }] },
  ]);
  state = { ...state, restore: { mode: 'empty', nodes: [] } };
  state = fail(state, 10);
  assert.equal(state.plainOnly, false);
  assert.equal(composerTryAgain(state, 20).restore?.mode, 'rich');
});

test('Start fresh remounts an empty editor and forgets the held draft', () => {
  const fallback = fail(fail(initialComposerSafetyNetState(), 0), 10);
  const fresh = composerStartFresh(fallback, 50);
  assert.equal(fresh.status, 'live');
  assert.equal(fresh.generation, fallback.generation + 1);
  assert.deepEqual(fresh.restore, { mode: 'empty', nodes: [] });
  assert.equal(fresh.snapshot.text, '');
  assert.equal(fresh.plainOnly, false);
});

test('a failure long after a recovery counts as a first failure again', () => {
  const recovered = fail(initialComposerSafetyNetState(), 1_000);
  const later = fail(recovered, 1_000 + COMPOSER_RECOVERY_WINDOW_MS + 1, [
    { type: 'paragraph', children: [{ text: 'typed since' }] },
  ]);
  assert.equal(later.status, 'live');
  assert.equal(later.generation, recovered.generation + 1);
  assert.equal(later.failures, 1);
  assert.equal(later.snapshot.text, 'typed since');
  // Also long after Try again.
  const fallback = fail(fail(initialComposerSafetyNetState(), 0), 10);
  const tried = composerTryAgain(fallback, 100);
  assert.equal(fail(tried, 100 + COMPOSER_RECOVERY_WINDOW_MS).status, 'live');
});

test('an invalid draft is restored as its text, an unreadable one keeps the last good snapshot', () => {
  const invalid = [{ type: 'paragraph', children: [] }, { text: 'loose text' }];
  assert.equal(isRestorableDraft(invalid), false);
  const plain = fail(initialComposerSafetyNetState(), 0, invalid);
  assert.equal(plain.restore?.mode, 'plain');
  assert.equal(plain.snapshot.text, 'loose text');

  const recovered = fail(initialComposerSafetyNetState(), 0);
  const later = composerFailed(recovered, COMPOSER_RECOVERY_WINDOW_MS * 2, () => {
    throw new Error('editor state unreadable');
  });
  assert.equal(later.snapshot, recovered.snapshot);
  assert.equal(later.restore?.mode, 'rich');
});

test('an empty draft remounts empty', () => {
  const next = fail(initialComposerSafetyNetState(), 0, [
    { type: 'paragraph', children: [{ text: '' }] },
  ]);
  assert.deepEqual(next.restore, { mode: 'empty', nodes: [] });
});

test('draft text follows blocks, lists and chips', () => {
  assert.equal(
    composerDraftText([
      { type: 'heading', level: 1, children: [{ text: 'Title' }] },
      {
        type: 'unordered-list',
        children: [
          { type: 'list-item', children: [{ text: 'one' }] },
          {
            type: 'list-item',
            children: [
              { type: 'paragraph', children: [{ text: 'two' }] },
              {
                type: 'ordered-list',
                children: [{ type: 'list-item', children: [{ text: 'nested' }] }],
              },
            ],
          },
        ],
      },
      {
        type: 'paragraph',
        children: [{ type: 'command', command: 'me', children: [{ text: '' }] }],
      },
    ]),
    'Title\none\ntwo\nnested\n/me'
  );
});

test('the diagnostics detail never carries the Slate node, so never the draft', () => {
  const detail = composerErrorDetail(
    new Error('Cannot resolve a DOM node from Slate node: {"text":"my secret draft"}')
  );
  assert.equal(detail, 'Error: Cannot resolve a DOM node from Slate node: [node]');
  assert.doesNotMatch(detail, /secret/);
});

test('every composer is inside the safety net, with the timeline and header outside', () => {
  const roomView = read('src/app/features/room/RoomView.tsx');
  const room = read('src/app/features/room/Room.tsx');
  const editor = read('src/app/features/room/NativeTimelineMessageEditor.tsx');
  const safetyNet = read('src/app/features/room/ComposerSafetyNet.tsx');

  assert.match(
    roomView,
    /<ComposerSafetyNet[\s\S]{0,120}surface="room"[\s\S]{0,400}<RoomInput[\s\S]{0,200}restore=\{restore\}/
  );
  assert.ok(
    roomView.indexOf('<NativeTimelinePresenter') < roomView.indexOf('<ComposerSafetyNet'),
    'the timeline stays outside the composer boundary'
  );
  assert.match(
    room,
    /<ComposerSafetyNet key=\{rootEventId\} surface="thread"[\s\S]{0,300}<RoomInput[\s\S]{0,200}threadRootOverride=\{rootEventId\}/
  );
  assert.match(editor, /<ComposerSafetyNet surface="edit"[\s\S]{0,200}<MessageEditorBody/);
  // A fresh editor per generation, and the boundary keyed by it.
  assert.match(safetyNet, /createComposerEditor\(\)/);
  assert.match(safetyNet, /<ErrorBoundary\s+key=\{generation\}/);
  assert.match(safetyNet, /recordDesktopDiagnostic\(/);
  assert.match(safetyNet, /role="alert"/);
});
