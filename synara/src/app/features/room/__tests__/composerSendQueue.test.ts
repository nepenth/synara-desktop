import assert from 'node:assert/strict';
import test from 'node:test';
import { createEditor, Descendant, Editor, Node, Transforms } from 'slate';
import {
  ComposerSendQueue,
  composerContentIsEmpty,
  mergeRestoredComposerContent,
  restoreComposerContent,
  stashComposerSendError,
  takeComposerSendError,
} from '../composerSendQueue';
import { BlockType } from '../../../components/editor/types';

const paragraph = (text: string): Descendant =>
  ({ type: BlockType.Paragraph, children: [{ text }] }) as Descendant;

const editorWith = (nodes: Descendant[]): Editor => {
  const editor = createEditor();
  editor.children = nodes;
  Transforms.select(editor, Editor.end(editor, []));
  return editor;
};

const editorText = (editor: Editor): string[] => editor.children.map((node) => Node.string(node));

const deferred = () => {
  let resolve!: () => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

const tick = () => new Promise((resolve) => setImmediate(resolve));

test('sends run one at a time in submit order', async () => {
  const order: string[] = [];
  const waiting: number[] = [];
  const first = deferred();
  const queue = new ComposerSendQueue<string>(
    () => assert.fail('no failure expected'),
    (count) => waiting.push(count)
  );

  queue.enqueue('ok', async () => {
    order.push('start ok');
    await first.promise;
    order.push('end ok');
  });
  // Enter pressed again while the first send waits for the server.
  queue.enqueue('sounds good', async () => {
    order.push('start sounds good');
  });

  assert.equal(queue.busy, true);
  assert.deepEqual(queue.pending, ['sounds good']);
  await tick();
  assert.deepEqual(order, ['start ok']);

  first.resolve();
  await tick();
  assert.deepEqual(order, ['start ok', 'end ok', 'start sounds good']);
  assert.equal(queue.busy, false);
  // 0 when "ok" starts, 1 while "sounds good" waits, 0 when it starts.
  assert.deepEqual(waiting, [0, 1, 0]);
});

test('a failed send returns itself and every send queued behind it, unsent', async () => {
  const failures: { error: unknown; failed: string; skipped: string[] }[] = [];
  const queue = new ComposerSendQueue<string>((error, failed, skipped) => {
    failures.push({ error, failed, skipped });
  });
  const first = deferred();
  let laterRan = false;

  queue.enqueue('one', () => first.promise);
  queue.enqueue('two', async () => {
    laterRan = true;
  });
  queue.enqueue('three', async () => {
    laterRan = true;
  });
  first.reject(new Error('wedged'));
  await tick();

  assert.equal(laterRan, false, 'later sends would land before the failed one');
  assert.equal(failures.length, 1);
  assert.equal(failures[0].failed, 'one');
  assert.deepEqual(failures[0].skipped, ['two', 'three']);
  assert.equal(queue.busy, false);

  // The queue keeps working after a failure.
  let ran = false;
  queue.enqueue('four', async () => {
    ran = true;
  });
  await tick();
  assert.equal(ran, true);
});

test('restoring into an empty composer replaces the empty line', () => {
  const editor = editorWith([paragraph('')]);

  restoreComposerContent(editor, [paragraph('ok'), paragraph('second line')]);

  assert.deepEqual(editorText(editor), ['ok', 'second line']);
  assert.deepEqual(editor.selection?.anchor, { path: [1, 0], offset: 'second line'.length });
});

test('restoring keeps text typed while the send was in flight, below the unsent text', () => {
  const editor = editorWith([paragraph('sounds good')]);

  restoreComposerContent(editor, [paragraph('ok')]);

  assert.deepEqual(editorText(editor), ['ok', 'sounds good']);
  // The caret stays in the text the user was typing.
  assert.deepEqual(editor.selection?.anchor.path, [1, 0]);
});

test('stored drafts merge the unsent text above a newer draft', () => {
  assert.equal(composerContentIsEmpty(undefined), true);
  assert.equal(composerContentIsEmpty([paragraph('')]), true);
  assert.equal(
    composerContentIsEmpty([
      {
        type: BlockType.Paragraph,
        children: [
          { text: '' },
          { type: BlockType.Mention, id: '@a:x', name: '@a', highlight: false, children: [] },
        ],
      } as unknown as Descendant,
    ]),
    false
  );

  assert.deepEqual(mergeRestoredComposerContent([paragraph('ok')], [paragraph('')]), [
    paragraph('ok'),
  ]);
  assert.deepEqual(mergeRestoredComposerContent([paragraph('ok')], [paragraph('later')]), [
    paragraph('ok'),
    paragraph('later'),
  ]);
});

test('a send error for a closed composer is shown once when it opens again', () => {
  stashComposerSendError('!room:x', 'Your homeserver refused this message.');

  assert.equal(takeComposerSendError('!other:x'), undefined);
  assert.equal(takeComposerSendError('!room:x'), 'Your homeserver refused this message.');
  assert.equal(takeComposerSendError('!room:x'), undefined);
});
