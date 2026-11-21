import test from 'node:test';
import assert from 'node:assert/strict';
import { Descendant, Editor, Node, Transforms, createEditor } from 'slate';
import { withHistory } from 'slate-history';
import { BlockType } from '../types';
import { withComposer } from '../composerEditor';
import { replaceTextRange } from '../replacementText';

const editorWith = (text: string): Editor => {
  const editor = withComposer(withHistory(createEditor()));
  editor.children = [{ type: BlockType.Paragraph, children: [{ text }] }] as Descendant[];
  return editor;
};

const at = (start: number, end: number) => ({
  anchor: { path: [0, 0], offset: start },
  focus: { path: [0, 0], offset: end },
});

const text = (editor: Editor): string => Node.string(editor);

test('a smart dash replaces the typed hyphens and the caret follows it', () => {
  const editor = editorWith('--');
  Transforms.select(editor, { path: [0, 0], offset: 2 });

  replaceTextRange(editor, at(0, 2), '—');

  assert.equal(text(editor), '—');
  assert.deepEqual(editor.selection, at(1, 1));

  // slate-react adds the character the browser was inserting afterwards.
  editor.insertText('x');
  assert.equal(text(editor), '—x');
});

test('a replacement mid-line keeps the text around it', () => {
  const editor = editorWith('ls --');
  Transforms.select(editor, { path: [0, 0], offset: 5 });

  replaceTextRange(editor, at(3, 5), '—');

  assert.equal(text(editor), 'ls —');
  assert.deepEqual(editor.selection, at(4, 4));
});

test('a longer replacement leaves the caret after the new text', () => {
  const editor = editorWith('see omw');
  Transforms.select(editor, { path: [0, 0], offset: 7 });

  replaceTextRange(editor, at(4, 7), 'On my way!');

  assert.equal(text(editor), 'see On my way!');
  assert.deepEqual(editor.selection, at(14, 14));
});

test('a caret after the replaced text keeps its place in the text', () => {
  const editor = editorWith('teh cat');
  Transforms.select(editor, { path: [0, 0], offset: 7 });

  replaceTextRange(editor, at(0, 3), 'their');

  assert.equal(text(editor), 'their cat');
  assert.deepEqual(editor.selection, at(9, 9));
});

test('a caret before the replaced text does not move', () => {
  const editor = editorWith('a teh');
  Transforms.select(editor, { path: [0, 0], offset: 0 });

  replaceTextRange(editor, at(2, 5), 'the');

  assert.equal(text(editor), 'a the');
  assert.deepEqual(editor.selection, at(0, 0));
});

test('a replacement is one undo step', () => {
  const editor = editorWith('--');
  Transforms.select(editor, { path: [0, 0], offset: 2 });

  replaceTextRange(editor, at(0, 2), '—');
  editor.undo();

  assert.equal(text(editor), '--');
});
