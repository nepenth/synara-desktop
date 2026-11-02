import test from 'node:test';
import assert from 'node:assert/strict';
import { Descendant, Editor, Element, Transforms, createEditor } from 'slate';
import { withHistory } from 'slate-history';
import { BlockType } from '../types';
import { createLinkElement, withLinkEdgeEscape } from '../utils';

const linkEditor = (): Editor => {
  const editor = withLinkEdgeEscape(withHistory(createEditor()));
  const { isInline } = editor;
  editor.isInline = (element) => element.type === BlockType.Link || isInline(element);
  editor.children = [
    {
      type: BlockType.Paragraph,
      children: [
        { text: 'see ' },
        createLinkElement('https://example.org/docs', 'example.org/docs'),
        { text: '' },
      ],
    },
  ] as Descendant[];
  return editor;
};

const paragraphChildren = (editor: Editor): Descendant[] =>
  (editor.children[0] as Element).children as Descendant[];

const linkText = (editor: Editor): string =>
  ((paragraphChildren(editor)[1] as Element).children[0] as { text: string }).text;

const LINK_END = { path: [0, 1, 0], offset: 'example.org/docs'.length };
const LINK_START = { path: [0, 1, 0], offset: 0 };

test('typing after a link, with a space first, lands outside the link', () => {
  const editor = linkEditor();
  Transforms.select(editor, LINK_END);

  editor.insertText(' ');
  editor.insertText('then');

  assert.equal(linkText(editor), 'example.org/docs');
  assert.deepEqual(paragraphChildren(editor)[2], { text: ' then' });
});

test('typing right after a link without a space also lands outside it', () => {
  const editor = linkEditor();
  Transforms.select(editor, LINK_END);

  editor.insertText('!');

  assert.equal(linkText(editor), 'example.org/docs');
  assert.deepEqual(paragraphChildren(editor)[2], { text: '!' });
});

test('typing before a link lands in the text before it', () => {
  const editor = linkEditor();
  Transforms.select(editor, LINK_START);

  editor.insertText('also ');

  assert.equal(linkText(editor), 'example.org/docs');
  assert.deepEqual(paragraphChildren(editor)[0], { text: 'see also ' });
});

test('typing inside a link still edits the link text', () => {
  const editor = linkEditor();
  Transforms.select(editor, { path: [0, 1, 0], offset: 'example.org'.length });

  editor.insertText('.test');

  assert.equal(linkText(editor), 'example.org.test/docs');
});

test('a line break at the end of a link starts a plain paragraph', () => {
  const editor = linkEditor();
  Transforms.select(editor, LINK_END);

  editor.insertBreak();
  editor.insertText('next');

  assert.equal(editor.children.length, 2);
  assert.equal(linkText(editor), 'example.org/docs');
  const next = editor.children[1] as Element;
  assert.ok(
    next.children.every((child) => !Element.isElement(child)),
    'the new paragraph holds no link'
  );
  assert.equal(Editor.string(editor, [1]), 'next');
});
