/**
 * The composer formatting model (docs/composer-formatting-model.md), case by
 * case. Documents are written with `|` marking the caret, or two `|` marking a
 * selection; expected documents with a `|` also check where the caret ends up.
 */
import test from 'node:test';
import assert from 'node:assert/strict';
import { createEditor, Descendant, Editor, Element, Node, Path, Point, Text } from 'slate';
import { withHistory } from 'slate-history';
import { withComposer } from '../composerEditor';
import { toggleKeyboardShortcut } from '../keyboard';
import { isBlockActive, toggleBlock } from '../formatting';
import { isEmptyEditor, toggleMark } from '../utils';
import { htmlToEditorInput } from '../input';
import { toMatrixCustomHTML, toPlainText, trimCustomHtml } from '../output';
import { BlockType, MarkType } from '../types';
import type { HeadingLevel } from '../slate';

// ---------------------------------------------------------------------------
// Document helpers

type Inline = string | Descendant;
const inline = (children: Inline[]): Descendant[] =>
  children.map((child) => (typeof child === 'string' ? { text: child } : child));
const p = (...children: Inline[]): Descendant =>
  ({ type: BlockType.Paragraph, children: inline(children) }) as Descendant;
const h = (level: HeadingLevel, text: string): Descendant =>
  ({ type: BlockType.Heading, level, children: [{ text }] }) as Descendant;
const quote = (...lines: string[]): Descendant =>
  ({
    type: BlockType.BlockQuote,
    children: lines.map((text) => ({ type: BlockType.QuoteLine, children: [{ text }] })),
  }) as Descendant;
const code = (...lines: string[]): Descendant =>
  ({
    type: BlockType.CodeBlock,
    children: lines.map((text) => ({ type: BlockType.CodeLine, children: [{ text }] })),
  }) as Descendant;
type Item = string | [string, ...Descendant[]];
const li = (item: Item): Descendant => {
  const [text, ...lists] = typeof item === 'string' ? [item] : item;
  return { type: BlockType.ListItem, children: [p(text), ...lists] } as Descendant;
};
const ol = (...items: Item[]): Descendant =>
  ({ type: BlockType.OrderedList, children: items.map(li) }) as Descendant;
const olFrom = (start: number, ...items: Item[]): Descendant =>
  ({ type: BlockType.OrderedList, start, children: items.map(li) }) as Descendant;
const ul = (...items: Item[]): Descendant =>
  ({ type: BlockType.UnorderedList, children: items.map(li) }) as Descendant;
const mention = (name: string): Descendant =>
  ({
    type: BlockType.Mention,
    id: '@alice:example.org',
    name,
    highlight: false,
    children: [{ text: '' }],
  }) as Descendant;

/** Removes `|` carets from a document and returns where they were. */
const extractCarets = (doc: Descendant[]): { doc: Descendant[]; points: Point[] } => {
  const points: Point[] = [];
  const walk = (node: Descendant, path: Path): Descendant => {
    if (Text.isText(node)) {
      let { text } = node;
      let index = text.indexOf('|');
      while (index >= 0) {
        points.push({ path, offset: index });
        text = text.slice(0, index) + text.slice(index + 1);
        index = text.indexOf('|');
      }
      return { ...node, text };
    }
    return {
      ...node,
      children: (node as Element).children.map((child, i) => walk(child, [...path, i])),
    } as Descendant;
  };
  return { doc: doc.map((node, i) => walk(node, [i])), points };
};

const editorWith = (doc: Descendant[], options: { markdown?: boolean } = {}) => {
  const editor = withComposer(withHistory(createEditor()));
  const { doc: children, points } = extractCarets(doc);
  editor.children = children;
  editor.markdown = options.markdown;
  if (points.length > 0) {
    editor.selection = { anchor: points[0], focus: points[1] ?? points[0] };
  }
  Editor.normalize(editor, { force: true });
  return editor;
};

const assertDoc = (editor: Editor, expected: Descendant[]) => {
  const { doc, points } = extractCarets(expected);
  assert.deepEqual(editor.children, doc);
  if (points.length > 0) assert.deepEqual(editor.selection?.anchor, points[0]);
};

const lineBreak = (editor: Editor) => editor.insertSoftBreak();
const enter = (editor: Editor) => editor.insertBreak();
const backspace = (editor: Editor) => editor.deleteBackward('character');
const key = (editor: Editor, name: string, init: Record<string, unknown> = {}) => {
  let prevented = false;
  const handled = toggleKeyboardShortcut(editor, {
    key: name,
    preventDefault: () => {
      prevented = true;
    },
    ...init,
  } as unknown as Parameters<typeof toggleKeyboardShortcut>[1]);
  return { handled, prevented };
};

// ---------------------------------------------------------------------------
// Schema

test('schema: list items written by older builds get their paragraph', () => {
  const editor = editorWith([
    {
      type: BlockType.OrderedList,
      children: [{ type: BlockType.ListItem, children: [{ text: 'old' }] }],
    } as Descendant,
  ]);
  assertDoc(editor, [ol('old')]);
});

test('schema: same-type neighbours join; different types stay apart', () => {
  assertDoc(editorWith([ol('a'), olFrom(2, 'b')]), [ol('a', 'b')]);
  assertDoc(editorWith([quote('a'), quote('b')]), [quote('a', 'b')]);
  assertDoc(editorWith([code('a'), code('b')]), [code('a', 'b')]);
  assertDoc(editorWith([ol('a'), ul('b')]), [ol('a'), ul('b')]);
});

test('schema: a second line inside a list item becomes the next item', () => {
  const editor = editorWith([
    {
      type: BlockType.UnorderedList,
      children: [{ type: BlockType.ListItem, children: [p('one'), p('two'), ul('sub')] }],
    } as Descendant,
  ]);
  assertDoc(editor, [ul('one', ['two', ul('sub')])]);
});

test('schema: an item with sub-items but no line of its own promotes them', () => {
  const editor = editorWith([
    {
      type: BlockType.UnorderedList,
      children: [li('a'), { type: BlockType.ListItem, children: [ul('b', 'c')] }],
    } as Descendant,
  ]);
  assertDoc(editor, [ul('a', 'b', 'c')]);
});

test('schema: stray lines and items at the top level become paragraphs', () => {
  const editor = editorWith([
    { type: BlockType.QuoteLine, children: [{ text: 'q' }] } as Descendant,
    { type: BlockType.CodeLine, children: [{ text: 'c' }] } as Descendant,
    li('i'),
  ]);
  assertDoc(editor, [p('q'), p('c'), p('i')]);
});

test('schema: code is plain text without marks, links or chips', () => {
  const editor = editorWith([
    {
      type: BlockType.CodeBlock,
      children: [
        {
          type: BlockType.CodeLine,
          children: [
            { text: 'x', bold: true },
            { type: BlockType.Link, href: 'https://example.org', children: [{ text: 'link' }] },
            mention('Alice'),
            { text: '' },
          ],
        },
      ],
    } as Descendant,
  ]);
  assert.equal(Node.string(editor), 'xlinkAlice');
  const [line] = Array.from(Node.texts(editor));
  assert.equal(Object.keys(line[0]).length, 1);
  assert.equal(
    Array.from(Node.descendants(editor)).some(
      ([node]) =>
        Element.isElement(node) &&
        node.type !== BlockType.CodeBlock &&
        node.type !== BlockType.CodeLine
    ),
    false
  );
});

test('schema: a paragraph inside a quote becomes a quote line; empty containers go', () => {
  assertDoc(
    editorWith([
      { type: BlockType.BlockQuote, children: [p('a'), h(2, 'b')] } as Descendant,
      { type: BlockType.UnorderedList, children: [] } as unknown as Descendant,
    ]),
    [quote('a', 'b')]
  );
  assertDoc(editorWith([]), [p('')]);
});

// ---------------------------------------------------------------------------
// Line breaks

test('break: a paragraph splits into a new paragraph', () => {
  const editor = editorWith([p('ab|cd')]);
  lineBreak(editor);
  assertDoc(editor, [p('ab'), p('|cd')]);
});

test('break: a heading never continues into an empty heading', () => {
  const atEnd = editorWith([h(1, 'Title|')]);
  lineBreak(atEnd);
  assertDoc(atEnd, [h(1, 'Title'), p('|')]);

  const inMiddle = editorWith([h(2, 'Ti|tle')]);
  lineBreak(inMiddle);
  assertDoc(inMiddle, [h(2, 'Ti'), h(2, '|tle')]);
});

test('break: a list item starts the next item, taking the text after the caret', () => {
  const atEnd = editorWith([ol('a', 'b|')]);
  lineBreak(atEnd);
  assertDoc(atEnd, [ol('a', 'b', '|')]);

  const inMiddle = editorWith([ol('ab|cd')]);
  lineBreak(inMiddle);
  assertDoc(inMiddle, [ol('ab', '|cd')]);
});

test('break: an item with sub-items starts a new item, which takes them', () => {
  const editor = editorWith([ol(['a|', ol('sub')])]);
  lineBreak(editor);
  assertDoc(editor, [ol('a', ['|', ol('sub')])]);
});

test('break: an empty nested item moves out a level', () => {
  const editor = editorWith([ol(['a', ol('sub', '|')])]);
  lineBreak(editor);
  assertDoc(editor, [ol(['a', ol('sub')], '|')]);
});

test('break: an empty top-level item leaves the list where it is', () => {
  const last = editorWith([ol('a', '|')]);
  lineBreak(last);
  assertDoc(last, [ol('a'), p('|')]);

  const middle = editorWith([ol('a', '|', 'c')]);
  lineBreak(middle);
  assertDoc(middle, [ol('a'), p('|'), olFrom(2, 'c')]);

  const only = editorWith([ul('|')]);
  lineBreak(only);
  assertDoc(only, [p('|')]);
});

test('break: quote and code lines continue; an empty last line leaves the block', () => {
  for (const block of [quote, code]) {
    const typing = editorWith([block('a|')]);
    lineBreak(typing);
    assertDoc(typing, [block('a', '|')]);

    const leaving = editorWith([block('a', '|')]);
    lineBreak(leaving);
    assertDoc(leaving, [block('a'), p('|')]);

    // An empty line in the middle is content (a blank line in code or a quote).
    const blank = editorWith([block('a', '|', 'c')]);
    lineBreak(blank);
    assertDoc(blank, [block('a', '', '|', 'c')]);
  }
});

test('break: Enter that does not send behaves like Shift+Enter', () => {
  const editor = editorWith([ul('a|')]);
  enter(editor);
  assertDoc(editor, [ul('a', '|')]);
});

test('break: a selection is deleted first', () => {
  const editor = editorWith([ol('a|bc', 'd|e')]);
  lineBreak(editor);
  assertDoc(editor, [ol('a', '|e')]);
});

test('break: Markdown mode continues typed list markers', () => {
  const bullet = editorWith([p('- one|')], { markdown: true });
  lineBreak(bullet);
  assertDoc(bullet, [p('- one'), p('- |')]);

  const numbered = editorWith([p('  9. nine|')], { markdown: true });
  lineBreak(numbered);
  assertDoc(numbered, [p('  9. nine'), p('  10. |')]);

  const split = editorWith([p('1) ab|cd')], { markdown: true });
  lineBreak(split);
  assertDoc(split, [p('1) ab'), p('2) |cd')]);

  const ending = editorWith([p('- one'), p('- |')], { markdown: true });
  lineBreak(ending);
  assertDoc(ending, [p('- one'), p('|')]);

  const plain = editorWith([p('- one|')]);
  lineBreak(plain);
  assertDoc(plain, [p('- one'), p('|')]);
});

// ---------------------------------------------------------------------------
// Backspace

test('backspace: at the start of a heading it becomes a paragraph', () => {
  const editor = editorWith([h(1, '|Title')]);
  backspace(editor);
  assertDoc(editor, [p('|Title')]);
});

test('backspace: at the start of a list item it removes the marker first', () => {
  const nested = editorWith([ol(['a', ol('|sub')])]);
  backspace(nested);
  assertDoc(nested, [ol('a', '|sub')]);

  const middle = editorWith([ol('a', '|b', 'c')]);
  backspace(middle);
  assertDoc(middle, [ol('a'), p('|b'), olFrom(2, 'c')]);

  const first = editorWith([olFrom(10, '|a', 'b')]);
  backspace(first);
  assertDoc(first, [p('|a'), olFrom(10, 'b')]);
});

test('backspace: the first quote or code line leaves the block; later lines join', () => {
  for (const block of [quote, code]) {
    const first = editorWith([block('|a', 'b')]);
    backspace(first);
    assertDoc(first, [p('|a'), block('b')]);

    const later = editorWith([block('a', '|b')]);
    backspace(later);
    assertDoc(later, [block('a|b')]);
  }
});

test('backspace: a paragraph below a list joins its last item, keeping all text', () => {
  const editor = editorWith([ol('a', 'b'), p('|x')]);
  backspace(editor);
  assertDoc(editor, [ol('a', 'b|x')]);

  const empty = editorWith([ol('a', 'b'), p('|')]);
  backspace(empty);
  assertDoc(empty, [ol('a', 'b|')]);
});

test('delete: removing everything leaves one plain paragraph', () => {
  const editor = editorWith([ul('|a', 'b'), quote('c|')]);
  Editor.deleteFragment(editor);
  assertDoc(editor, [p('|')]);
});

// ---------------------------------------------------------------------------
// Tab, Escape and shortcuts

test('tab: indents under the item above, joining its sub-list', () => {
  const editor = editorWith([ol(['a', ol('x')], '|b')]);
  assert.deepEqual(key(editor, 'Tab'), { handled: true, prevented: true });
  assertDoc(editor, [ol(['a', ol('x', '|b')])]);
});

test('tab: the first item and lines outside a list keep Tab for focus', () => {
  assert.deepEqual(key(editorWith([ol('|a')]), 'Tab'), { handled: false, prevented: false });
  assert.deepEqual(key(editorWith([p('|a')]), 'Tab'), { handled: false, prevented: false });
});

test('shift-tab: outdenting keeps reading order', () => {
  const editor = editorWith([ol(['a', ol('b', '|c', 'd')])]);
  key(editor, 'Tab', { shiftKey: true });
  assertDoc(editor, [ol(['a', ol('b')], ['|c', ol('d')])]);
});

test('tab: every selected item moves', () => {
  const editor = editorWith([ul('a', '|b', 'c|')]);
  key(editor, 'Tab');
  assertDoc(editor, [ul(['a', ul('b', 'c')])]);
  key(editor, 'Tab', { shiftKey: true });
  assertDoc(editor, [ul('a', 'b', 'c')]);
});

test('escape: removes marks first, then block formatting; not when already used', () => {
  const editor = editorWith([quote('a|')]);
  toggleMark(editor, MarkType.Bold);
  assert.equal(key(editor, 'Escape').handled, true);
  assert.equal(Editor.marks(editor)?.bold, undefined);
  assert.equal(key(editor, 'Escape', { defaultPrevented: true }).handled, false);
  assertDoc(editor, [quote('a|')]);
  assert.equal(key(editor, 'Escape').handled, true);
  assertDoc(editor, [p('a|')]);
  assert.equal(key(editor, 'Escape').handled, false);
});

test('shortcuts: block and heading hotkeys toggle the format', () => {
  const editor = editorWith([p('a|')]);
  key(editor, '7', { ctrlKey: true });
  assertDoc(editor, [ol('a|')]);
  key(editor, '1', { ctrlKey: true });
  assertDoc(editor, [h(1, 'a|')]);
});

test('shortcuts: mark hotkeys do nothing inside code', () => {
  const editor = editorWith([code('a|')]);
  key(editor, 'b', { ctrlKey: true });
  assert.equal(Editor.marks(editor)?.bold, undefined);
});

// ---------------------------------------------------------------------------
// Block commands

test('toggle: several selected lines become one list, quote or code block', () => {
  for (const [format, expected] of [
    [BlockType.OrderedList, ol('one', 'two', 'three')],
    [BlockType.UnorderedList, ul('one', 'two', 'three')],
    [BlockType.BlockQuote, quote('one', 'two', 'three')],
    [BlockType.CodeBlock, code('one', 'two', 'three')],
  ] as const) {
    const editor = editorWith([p('o|ne'), h(2, 'two'), p('th|ree')]);
    toggleBlock(editor, format);
    assertDoc(editor, [expected]);
  }
});

test('toggle: several selected lines become headings', () => {
  const editor = editorWith([p('|one'), quote('two|')]);
  toggleBlock(editor, BlockType.Heading, { level: 2 });
  assertDoc(editor, [h(2, 'one'), h(2, 'two')]);
});

test('toggle: when every selected line has the format it is removed', () => {
  const list = editorWith([ol('|a', 'b|')]);
  toggleBlock(list, BlockType.OrderedList);
  assertDoc(list, [p('a'), p('b')]);

  const middle = editorWith([ol('a', 'b|', 'c')]);
  toggleBlock(middle, BlockType.OrderedList);
  assertDoc(middle, [ol('a'), p('b|'), olFrom(2, 'c')]);

  const heading = editorWith([h(3, 'a|')]);
  toggleBlock(heading, BlockType.Heading, { level: 3 });
  assertDoc(heading, [p('a|')]);
});

test('toggle: switching a whole list to the other type keeps it one list', () => {
  const editor = editorWith([ol('|a', ['b', ol('c')], 'd|')]);
  toggleBlock(editor, BlockType.UnorderedList);
  assertDoc(editor, [ul('a', 'b', 'c', 'd')]);
});

test('toggle: a numbered list joins or continues the one above', () => {
  const below = editorWith([ol('a', 'b', 'c'), p('|')]);
  toggleBlock(below, BlockType.OrderedList);
  assertDoc(below, [ol('a', 'b', 'c', '|')]);

  const afterNote = editorWith([ol('a', 'b', 'c'), p('note'), p('|')]);
  toggleBlock(afterNote, BlockType.OrderedList);
  assertDoc(afterNote, [ol('a', 'b', 'c'), p('note'), olFrom(4, '|')]);

  const afterHeading = editorWith([ol('a'), h(2, 'Next'), p('|')]);
  toggleBlock(afterHeading, BlockType.OrderedList);
  assertDoc(afterHeading, [ol('a'), h(2, 'Next'), ol('|')]);

  const afterBullets = editorWith([ol('a'), ul('x'), p('|')]);
  toggleBlock(afterBullets, BlockType.OrderedList);
  assertDoc(afterBullets, [ol('a'), ul('x'), ol('|')]);
});

test('toggle: a list line made into a quote leaves the list, splitting it', () => {
  const editor = editorWith([ol('a', 'b|', 'c')]);
  toggleBlock(editor, BlockType.BlockQuote);
  assertDoc(editor, [ol('a'), quote('b|'), olFrom(2, 'c')]);
});

test('toggle: deleting the line between two halves of a list joins them', () => {
  const editor = editorWith([ol('a'), p('|'), olFrom(2, 'b')]);
  backspace(editor);
  assertDoc(editor, [ol('a|', 'b')]);
});

test('toggle: one toggle is one undo step', () => {
  const editor = editorWith([p('|one'), p('two|')]);
  toggleBlock(editor, BlockType.OrderedList);
  editor.undo();
  assertDoc(editor, [p('one'), p('two')]);
});

test('active state: true only when every selected line has the format', () => {
  const editor = editorWith([ol('|a'), p('b|')]);
  assert.equal(isBlockActive(editor, BlockType.OrderedList), false);
  const inList = editorWith([ol('a', ['b', ul('|c')])]);
  assert.equal(isBlockActive(inList, BlockType.UnorderedList), true);
  assert.equal(isBlockActive(inList, BlockType.OrderedList), false);
});

// ---------------------------------------------------------------------------
// Empty composer

test('empty: blank structure is an empty composer; a chip is not', () => {
  assert.equal(isEmptyEditor(editorWith([ul('')])), true);
  assert.equal(isEmptyEditor(editorWith([quote('', '')])), true);
  assert.equal(isEmptyEditor(editorWith([code('')])), true);
  assert.equal(isEmptyEditor(editorWith([ul('a')])), false);
  assert.equal(isEmptyEditor(editorWith([p('', mention('Alice'), '')])), false);
});

// ---------------------------------------------------------------------------
// Reading HTML back and writing it out

const parsed = (html: string) => {
  const editor = withComposer(withHistory(createEditor()));
  editor.children = htmlToEditorInput(html);
  Editor.normalize(editor, { force: true });
  return editor.children;
};

test('input: line breaks inside paragraphs and list items are kept', () => {
  assert.deepEqual(parsed('<p>one<br>two</p>'), [p('one'), p('two')]);
  assert.deepEqual(parsed('<ul><li>one<br>two</li><li><p>three</p><p>four</p></li></ul>'), [
    ul('one', 'two', 'three', 'four'),
  ]);
});

test('input: lists, headings and code inside a quote become quote lines', () => {
  assert.deepEqual(
    parsed(
      '<blockquote><p>intro</p><ol start="3"><li>a</li><li>b<ul><li>c</li></ul></li></ol><h2>Head</h2><pre><code>x\ny\n</code></pre></blockquote>'
    ),
    [quote('intro', '3. a', '4. b', '  - c', 'Head', 'x', 'y')]
  );
});

test('output: quotes fall back to "> " lines in the plain body', () => {
  assert.equal(toPlainText([quote('a', 'b')], false).trim(), '> a\n> b');
});

test('output: a continued list is sent with its start number', () => {
  const editor = editorWith([ol('a', 'b'), p('note'), p('|')]);
  toggleBlock(editor, BlockType.OrderedList);
  editor.insertText('c');
  assert.equal(
    trimCustomHtml(toMatrixCustomHTML(editor.children, { allowTextFormatting: true })),
    '<ol><li><p>a</p></li><li><p>b</p></li></ol>note<br/><ol start="3"><li><p>c</p></li></ol>'
  );
});
