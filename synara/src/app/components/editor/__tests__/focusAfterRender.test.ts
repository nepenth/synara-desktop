import test from 'node:test';
import assert from 'node:assert/strict';
import { Descendant, Editor, Transforms, createEditor } from 'slate';
import { IS_NODE_MAP_DIRTY } from 'slate-dom';
import { ReactEditor, withReact } from 'slate-react';
import { BlockType } from '../types';
import { withComposer } from '../composerEditor';
import {
  createEmoticonElement,
  createMentionElement,
  focusEditorAfterRender,
  moveCursor,
} from '../utils';

// Picking an emoji (or a mention) puts the cursor in a new empty text after
// the chip. Until React draws it, slate-dom cannot map that cursor to the DOM,
// and `ReactEditor.focus` threw "Cannot resolve a DOM node from Slate node:
// {"text":""}", which replaced the whole screen with the error page. The
// composer's refocus must wait until the edit is drawn.

const frames: FrameRequestCallback[] = [];
const runFrame = () => {
  const pending = frames.splice(0);
  pending.forEach((callback) => callback(0));
};
const flushSlate = () => new Promise<void>((resolve) => queueMicrotask(resolve));

let focused = false;
let focusCalls = 0;
globalThis.requestAnimationFrame = (callback) => {
  frames.push(callback);
  return frames.length;
};
ReactEditor.isFocused = () => focused;
ReactEditor.focus = () => {
  focusCalls += 1;
};

const composer = (): Editor => {
  const editor = withComposer(withReact(createEditor()));
  editor.children = [{ type: BlockType.Paragraph, children: [{ text: 'hey' }] }] as Descendant[];
  Transforms.select(editor, Editor.end(editor, []));
  return editor;
};

/** What slate-react does when it renders the editor's children. */
const drawn = (editor: Editor) => IS_NODE_MAP_DIRTY.set(editor, false);

const reset = () => {
  frames.length = 0;
  focused = false;
  focusCalls = 0;
};

test('after an emoji pick, the editor is focused only once React has drawn it', async () => {
  reset();
  const editor = composer();
  await flushSlate();
  drawn(editor);

  editor.insertNode(createEmoticonElement('😀', 'grinning'));
  moveCursor(editor);
  assert.deepEqual(editor.selection?.anchor, { path: [0, 2], offset: 0 });
  focusEditorAfterRender(editor);

  // Operations still wait for Slate's flush.
  runFrame();
  assert.equal(focusCalls, 0);

  // Flushed, but React has not drawn the new text after the emoji yet.
  await flushSlate();
  assert.equal(editor.operations.length, 0);
  runFrame();
  assert.equal(focusCalls, 0);

  drawn(editor);
  runFrame();
  assert.equal(focusCalls, 1);
  assert.equal(frames.length, 0);
});

test('after a mention pick, the editor is focused only once React has drawn it', async () => {
  reset();
  const editor = composer();
  await flushSlate();
  drawn(editor);

  Transforms.insertNodes(editor, createMentionElement('@fiona:example.org', '@Fiona', false));
  moveCursor(editor, true);
  focusEditorAfterRender(editor);
  await flushSlate();
  runFrame();
  assert.equal(focusCalls, 0);

  drawn(editor);
  runFrame();
  assert.equal(focusCalls, 1);
});

test('focus is given up, not forced, when the edit is never drawn', async () => {
  reset();
  const editor = composer();
  editor.insertNode(createEmoticonElement('😀', 'grinning'));
  focusEditorAfterRender(editor);
  await flushSlate();

  for (let index = 0; index < 30; index += 1) runFrame();

  assert.equal(focusCalls, 0);
  assert.equal(frames.length, 0);
});

test('a drawn editor is focused on the next frame', async () => {
  reset();
  const editor = composer();
  await flushSlate();
  drawn(editor);

  focusEditorAfterRender(editor);
  assert.equal(focusCalls, 0);
  runFrame();
  assert.equal(focusCalls, 1);
});

test('an editor that already has focus is left alone', async () => {
  reset();
  const editor = composer();
  await flushSlate();
  drawn(editor);
  focused = true;

  focusEditorAfterRender(editor);
  runFrame();

  assert.equal(focusCalls, 0);
});
