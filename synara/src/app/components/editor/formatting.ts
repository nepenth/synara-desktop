/* eslint-disable no-param-reassign */
/**
 * The composer's block formatting: schema normalization, line-break and
 * Backspace behaviour, and the block commands. The rules are written down in
 * docs/composer-formatting-model.md; keep the two in step.
 */
import {
  Descendant,
  Editor,
  Element,
  Node,
  NodeEntry,
  Path,
  PathRef,
  Point,
  Range,
  Text,
  Transforms,
} from 'slate';
import { BlockType } from './types';
import type {
  CustomElement,
  HeadingLevel,
  ListItemElement,
  OrderedListElement,
  ParagraphElement,
  UnorderedListElement,
} from './slate';

type ListElement = OrderedListElement | UnorderedListElement;
type ListFormat = BlockType.OrderedList | BlockType.UnorderedList;

/** Formats a text block can have; containers name the format of their lines. */
export type BlockFormat =
  BlockType.Paragraph | BlockType.Heading | BlockType.BlockQuote | BlockType.CodeBlock | ListFormat;

const TEXT_BLOCK_TYPES: BlockType[] = [
  BlockType.Paragraph,
  BlockType.Heading,
  BlockType.QuoteLine,
  BlockType.CodeLine,
];
const LIST_TYPES: BlockType[] = [BlockType.OrderedList, BlockType.UnorderedList];
const MERGEABLE_CONTAINERS: BlockType[] = [
  BlockType.OrderedList,
  BlockType.UnorderedList,
  BlockType.BlockQuote,
  BlockType.CodeBlock,
];
const CHIP_TYPES: BlockType[] = [BlockType.Mention, BlockType.Emoticon, BlockType.Command];
const INLINE_TYPES: BlockType[] = [...CHIP_TYPES, BlockType.Link];
const MARKS = ['bold', 'italic', 'underline', 'strikeThrough', 'code', 'spoiler'] as const;

const isType = (node: unknown, type: BlockType): boolean =>
  Element.isElement(node) && node.type === type;

export const isList = (node: unknown): node is ListElement =>
  Element.isElement(node) && LIST_TYPES.includes(node.type);

export const isOrderedList = (node: unknown): node is OrderedListElement =>
  isType(node, BlockType.OrderedList);

export const isListItem = (node: unknown): node is ListItemElement =>
  isType(node, BlockType.ListItem);

export const isTextBlock = (node: unknown): node is CustomElement =>
  Element.isElement(node) && TEXT_BLOCK_TYPES.includes(node.type);

/** Inline content: text, links and chips. Decided by type, not by plugins. */
const isInlineContent = (node: unknown): boolean =>
  Text.isText(node) || (Element.isElement(node) && INLINE_TYPES.includes(node.type));

const isChip = (node: unknown): boolean =>
  Element.isElement(node) && CHIP_TYPES.includes(node.type);

const emptyParagraph = (): ParagraphElement => ({
  type: BlockType.Paragraph,
  children: [{ text: '' }],
});

/** True when the nodes hold no text and no mention, emoji or command chip. */
export const isBlankContent = (nodes: Descendant[]): boolean =>
  nodes.every((node) => {
    if (Node.string(node) !== '') return false;
    if (!Element.isElement(node)) return true;
    return !isChip(node) && !Array.from(Node.descendants(node)).some(([child]) => isChip(child));
  });

// ---------------------------------------------------------------------------
// Queries

const firstNumber = (list: OrderedListElement): number =>
  Number.isSafeInteger(list.start) && list.start && list.start > 1 ? list.start : 1;

/** The text block holding a point (default: the selection's start). */
export const textBlockAt = (editor: Editor, at?: Point): NodeEntry<CustomElement> | undefined => {
  const point = at ?? (editor.selection ? Range.start(editor.selection) : undefined);
  if (!point) return undefined;
  return Editor.above(editor, { at: point, match: isTextBlock, mode: 'lowest' }) as
    NodeEntry<CustomElement> | undefined;
};

/** Every text block the selection touches, in document order. */
export const selectedTextBlocks = (editor: Editor): NodeEntry<CustomElement>[] => {
  if (!editor.selection) return [];
  return Array.from(
    Editor.nodes(editor, { at: editor.selection, match: isTextBlock, mode: 'lowest' })
  ) as NodeEntry<CustomElement>[];
};

const parentNode = (editor: Editor, path: Path): Node | undefined =>
  path.length > 0 && Node.has(editor, Path.parent(path))
    ? Node.get(editor, Path.parent(path))
    : undefined;

/** The list item a text block is the paragraph of. */
const listItemOf = (editor: Editor, [block, path]: NodeEntry<CustomElement>) => {
  if (block.type !== BlockType.Paragraph || path.length < 2) return undefined;
  const itemPath = Path.parent(path);
  const item = Node.get(editor, itemPath);
  return isListItem(item) ? ([item, itemPath] as NodeEntry<ListItemElement>) : undefined;
};

const isNestedItem = (editor: Editor, itemPath: Path): boolean =>
  isListItem(parentNode(editor, Path.parent(itemPath)));

/** The format a text block shows: its own type, or its container's. */
export const blockFormatOf = (
  editor: Editor,
  entry: NodeEntry<CustomElement>
): { format: BlockFormat; level?: HeadingLevel } => {
  const [block] = entry;
  switch (block.type) {
    case BlockType.Heading:
      return { format: BlockType.Heading, level: block.level };
    case BlockType.QuoteLine:
      return { format: BlockType.BlockQuote };
    case BlockType.CodeLine:
      return { format: BlockType.CodeBlock };
    default: {
      const item = listItemOf(editor, entry);
      if (!item) return { format: BlockType.Paragraph };
      const list = Node.get(editor, Path.parent(item[1]));
      return { format: isOrderedList(list) ? BlockType.OrderedList : BlockType.UnorderedList };
    }
  }
};

const hasFormat = (
  editor: Editor,
  entry: NodeEntry<CustomElement>,
  format: BlockFormat,
  level?: HeadingLevel
): boolean => {
  const current = blockFormatOf(editor, entry);
  if (current.format !== format) return false;
  return format !== BlockType.Heading || level === undefined || current.level === level;
};

/** True when every selected text block has the format. */
export const isBlockActive = (editor: Editor, format: BlockType, level?: HeadingLevel): boolean => {
  const blocks = selectedTextBlocks(editor);
  return (
    blocks.length > 0 &&
    blocks.every((entry) => hasFormat(editor, entry, format as BlockFormat, level))
  );
};

/** True when any selected text block is not a plain paragraph. */
export const hasBlockFormatting = (editor: Editor): boolean =>
  selectedTextBlocks(editor).some(
    (entry) => blockFormatOf(editor, entry).format !== BlockType.Paragraph
  );

export const headingLevel = (editor: Editor): HeadingLevel | undefined => {
  const entry = textBlockAt(editor);
  if (!entry) return undefined;
  const { format, level } = blockFormatOf(editor, entry);
  return format === BlockType.Heading ? level : undefined;
};

// ---------------------------------------------------------------------------
// List numbering

/**
 * The number a top-level ordered list at `path` continues from: the ordered
 * list above it, across paragraphs, quotes and code, so "1, 2, 3, a note, 4"
 * reads the way it was meant. A heading or a bulleted list starts a new
 * section, so numbering restarts after one.
 */
export const continuedListStart = (editor: Editor, path: Path): number | undefined => {
  if (path.length !== 1) return undefined;
  for (let index = path[0] - 1; index >= 0; index -= 1) {
    const node = editor.children[index];
    if (isOrderedList(node)) {
      const next = firstNumber(node) + node.children.length;
      return next > 1 ? next : undefined;
    }
    if (isType(node, BlockType.UnorderedList) || isType(node, BlockType.Heading)) return undefined;
  }
  return undefined;
};

const setListStart = (editor: Editor, path: Path, start: number | undefined) => {
  const list = Node.get(editor, path);
  if (!isOrderedList(list) || list.start === start) return;
  if (start === undefined) Transforms.unsetNodes(editor, 'start', { at: path });
  else Transforms.setNodes(editor, { start }, { at: path });
};

/** After a list was split around the top-level block at `path`, its lower half counts on. */
const renumberAfter = (editor: Editor, path: Path) => {
  const top = [path[0]];
  const nextPath = Path.next(top);
  if (!Node.has(editor, nextPath) || !isOrderedList(Node.get(editor, nextPath))) return;
  const start = continuedListStart(editor, nextPath);
  if (start !== undefined) setListStart(editor, nextPath, start);
};

// ---------------------------------------------------------------------------
// Structural moves

/** Lifts the block at `ref` out of every container, splitting them around it. */
const liftToTop = (editor: Editor, ref: PathRef) => {
  while (ref.current && ref.current.length > 1) {
    Transforms.liftNodes(editor, { at: ref.current });
  }
};

const setTextBlockType = (editor: Editor, path: Path, type: BlockType, level?: HeadingLevel) => {
  if (type === BlockType.Heading) {
    Transforms.setNodes<Element>(editor, { type, level: level ?? 1 } as Partial<Element>, {
      at: path,
    });
    return;
  }
  Transforms.unsetNodes(editor, 'level', { at: path });
  Transforms.setNodes<Element>(editor, { type } as Partial<Element>, { at: path });
};

/** Moves a list item one level in, under the item above it. */
const indentItem = (editor: Editor, itemPath: Path): boolean => {
  if (!Path.hasPrevious(itemPath)) return false;
  const list = Node.get(editor, Path.parent(itemPath));
  const previousPath = Path.previous(itemPath);
  const previous = Node.get(editor, previousPath);
  if (!isList(list) || !isListItem(previous)) return false;

  const lastIndex = previous.children.length - 1;
  const last = previous.children[lastIndex];
  if (isList(last) && last.type === list.type) {
    Transforms.moveNodes(editor, {
      at: itemPath,
      to: [...previousPath, lastIndex, last.children.length],
    });
    return true;
  }
  Transforms.wrapNodes(editor, { type: list.type, children: [] } as ListElement, { at: itemPath });
  Transforms.moveNodes(editor, { at: itemPath, to: [...previousPath, previous.children.length] });
  return true;
};

/**
 * Moves a nested list item one level out, to just after its parent item. The
 * items that followed it become its sub-items, so the reading order stays.
 */
const outdentItem = (editor: Editor, itemPath: Path): boolean => {
  const listPath = Path.parent(itemPath);
  const list = Node.get(editor, listPath);
  const parentItemPath = Path.parent(listPath);
  if (!isList(list) || !isListItem(parentNode(editor, listPath))) return false;

  const index = itemPath[itemPath.length - 1];
  if (index < list.children.length - 1) {
    const item = Node.get(editor, itemPath) as ListItemElement;
    Transforms.wrapNodes(editor, { type: list.type, children: [] } as ListElement, {
      at: Editor.range(editor, Path.next(itemPath), [...listPath, list.children.length - 1]),
      match: (_node, path) => path.length === itemPath.length && Path.isAfter(path, itemPath),
    });
    Transforms.moveNodes(editor, {
      at: Path.next(itemPath),
      to: [...itemPath, item.children.length],
    });
  }
  Transforms.moveNodes(editor, { at: itemPath, to: Path.next(parentItemPath) });
  return true;
};

/** Takes a top-level list item's line out of the list as a paragraph in its place. */
const unlistItem = (editor: Editor, paragraphPath: Path) => {
  const ref = Editor.pathRef(editor, paragraphPath);
  liftToTop(editor, ref);
  const path = ref.unref();
  if (!path) return;
  setTextBlockType(editor, path, BlockType.Paragraph);
  renumberAfter(editor, path);
};

/** Takes the first or last line out of a quote or code block as a paragraph. */
const liftLine = (editor: Editor, linePath: Path) => {
  const ref = Editor.pathRef(editor, linePath);
  Transforms.liftNodes(editor, { at: linePath });
  const lifted = ref.unref();
  if (lifted) setTextBlockType(editor, lifted, BlockType.Paragraph);
};

/** The top-most list items among the selected text blocks. */
const selectedListItems = (editor: Editor): PathRef[] => {
  const items = selectedTextBlocks(editor)
    .map((entry) => listItemOf(editor, entry)?.[1])
    .filter((path): path is Path => !!path);
  return items
    .filter((path) => !items.some((other) => Path.isAncestor(other, path)))
    .map((path) => Editor.pathRef(editor, path));
};

export const indentListItem = (editor: Editor): boolean => {
  const refs = selectedListItems(editor);
  let changed = false;
  Editor.withoutNormalizing(editor, () => {
    refs.forEach((ref) => {
      if (ref.current && indentItem(editor, ref.current)) changed = true;
    });
  });
  refs.forEach((ref) => ref.unref());
  return changed;
};

export const outdentListItem = (editor: Editor): boolean => {
  const refs = selectedListItems(editor);
  let changed = false;
  Editor.withoutNormalizing(editor, () => {
    refs.forEach((ref) => {
      if (ref.current && outdentItem(editor, ref.current)) changed = true;
    });
  });
  refs.forEach((ref) => ref.unref());
  return changed;
};

// ---------------------------------------------------------------------------
// Block commands

const CONTAINER_LINE: Partial<Record<BlockFormat, BlockType>> = {
  [BlockType.BlockQuote]: BlockType.QuoteLine,
  [BlockType.CodeBlock]: BlockType.CodeLine,
};

/**
 * Gives every selected text block the format, or takes it away when they all
 * have it already. See "Block commands" in the formatting model.
 */
export const toggleBlock = (
  editor: Editor,
  format: BlockType,
  option?: { level?: HeadingLevel }
): void => {
  const blocks = selectedTextBlocks(editor);
  if (blocks.length === 0) return;
  const target = format as BlockFormat;
  const level = target === BlockType.Heading ? (option?.level ?? 1) : undefined;
  const active =
    target === BlockType.Paragraph ||
    blocks.every((entry) => hasFormat(editor, entry, target, level));
  const result: BlockFormat = active ? BlockType.Paragraph : target;
  const fromOrdered = blocks.some(
    (entry) => blockFormatOf(editor, entry).format === BlockType.OrderedList
  );
  const refs = blocks.map(([, path]) => Editor.pathRef(editor, path));

  Editor.withoutNormalizing(editor, () => {
    refs.forEach((ref) => liftToTop(editor, ref));
    const paths = refs.map((ref) => ref.current).filter((path): path is Path => !!path);
    if (paths.length === 0) return;

    const lineType =
      CONTAINER_LINE[result] ??
      (result === BlockType.Heading ? BlockType.Heading : BlockType.Paragraph);
    paths.forEach((path) => setTextBlockType(editor, path, lineType, level));

    const first = paths[0];
    const last = paths[paths.length - 1];
    const span = { anchor: Editor.start(editor, first), focus: Editor.end(editor, last) };
    const topLevel = (_node: Node, path: Path) => path.length === 1;

    if (result === BlockType.OrderedList || result === BlockType.UnorderedList) {
      paths.forEach((path) => {
        Transforms.wrapNodes(
          editor,
          { type: BlockType.ListItem, children: [] } as ListItemElement,
          {
            at: path,
          }
        );
      });
      Transforms.wrapNodes(editor, { type: result, children: [] } as ListElement, {
        at: span,
        match: topLevel,
      });
      const listPath = refs[0].current?.slice(0, 1);
      if (result === BlockType.OrderedList && listPath) {
        const start = continuedListStart(editor, listPath);
        if (start !== undefined) setListStart(editor, listPath, start);
      }
    } else if (CONTAINER_LINE[result]) {
      Transforms.wrapNodes(editor, { type: result, children: [] } as CustomElement, {
        at: span,
        match: topLevel,
      });
    }

    // Items below a numbered list that was split keep counting on.
    const lastPath = refs[refs.length - 1].current;
    if (fromOrdered && result !== BlockType.OrderedList && lastPath)
      renumberAfter(editor, lastPath);
  });
  refs.forEach((ref) => ref.unref());
};

/** Escape / Mod+E: drop inline marks first, then block formatting. */
export const exitFormatting = (editor: Editor): boolean => {
  const marks = Editor.marks(editor);
  if (marks && MARKS.some((mark) => marks[mark])) {
    MARKS.forEach((mark) => Editor.removeMark(editor, mark));
    return true;
  }
  if (hasBlockFormatting(editor)) {
    toggleBlock(editor, BlockType.Paragraph);
    return true;
  }
  return false;
};

// ---------------------------------------------------------------------------
// Keys

const MARKDOWN_LIST_MARKER = /^(\s*)(?:([-*+])|(\d{1,9})([.)]))( +)/;

/** Markdown mode: "- a" or "3. a" + line break continues with "- " or "4. ". */
const continueMarkdownList = (editor: Editor, [block, path]: NodeEntry<CustomElement>) => {
  const text = Node.string(block);
  const match = MARKDOWN_LIST_MARKER.exec(text);
  if (!match || !editor.selection) return false;
  const [marker, indent, bullet, number, delimiter, gap] = match;
  const atEnd = Point.equals(editor.selection.anchor, Editor.end(editor, path));
  if (text.slice(marker.length).trim() === '' && atEnd) {
    // An empty item ends the list: clear its marker instead of adding another.
    Transforms.delete(editor, { at: Editor.range(editor, path) });
    return true;
  }
  const next = bullet ?? `${Number(number) + 1}${delimiter}`;
  Transforms.splitNodes(editor, { always: true });
  Transforms.insertText(editor, `${indent}${next}${gap}`);
  return true;
};

/** Shift+Enter, and Enter when it does not send. */
const breakLine = (editor: Editor, splitBlock: () => void) => {
  if (!editor.selection) return;
  Editor.withoutNormalizing(editor, () => breakLineUnnormalized(editor, splitBlock));
};

const breakLineUnnormalized = (editor: Editor, splitBlock: () => void) => {
  if (!editor.selection) return;
  if (Range.isExpanded(editor.selection)) Editor.deleteFragment(editor);
  const entry = textBlockAt(editor);
  if (!entry) {
    splitBlock();
    return;
  }
  const [block, path] = entry;
  const empty = isBlankContent([block]);
  const item = listItemOf(editor, entry);

  if (item) {
    if (!empty) {
      Transforms.splitNodes(editor, { match: isListItem, mode: 'lowest', always: true });
    } else if (isNestedItem(editor, item[1])) {
      outdentItem(editor, item[1]);
    } else {
      unlistItem(editor, path);
    }
    return;
  }

  switch (block.type) {
    case BlockType.Heading: {
      splitBlock();
      const nextEntry = textBlockAt(editor);
      if (
        nextEntry &&
        Node.string(nextEntry[0]) === '' &&
        nextEntry[0].type === BlockType.Heading
      ) {
        setTextBlockType(editor, nextEntry[1], BlockType.Paragraph);
      }
      return;
    }
    case BlockType.QuoteLine:
    case BlockType.CodeLine: {
      const isLastLine = !Node.has(editor, Path.next(path));
      if (empty && isLastLine) {
        liftLine(editor, path);
        return;
      }
      splitBlock();
      return;
    }
    default:
      if (editor.markdown && continueMarkdownList(editor, entry)) return;
      splitBlock();
  }
};

/** Backspace at the very start of a text block: formatting goes before text joins. */
const backspaceAtStart = (editor: Editor, entry: NodeEntry<CustomElement>): boolean => {
  const [block, path] = entry;
  const item = listItemOf(editor, entry);
  if (item) {
    if (isNestedItem(editor, item[1])) return outdentItem(editor, item[1]);
    unlistItem(editor, path);
    return true;
  }
  switch (block.type) {
    case BlockType.Heading:
      setTextBlockType(editor, path, BlockType.Paragraph);
      return true;
    case BlockType.QuoteLine:
    case BlockType.CodeLine: {
      if (path[path.length - 1] !== 0) return false;
      liftLine(editor, path);
      return true;
    }
    default:
      return false;
  }
};

// ---------------------------------------------------------------------------
// Schema

const wrapInlineRun = (editor: Editor, parentPath: Path, from: number, type: BlockType) => {
  const parent = Node.get(editor, parentPath) as Element;
  let to = from;
  while (to + 1 < parent.children.length && isInlineContent(parent.children[to + 1])) to += 1;
  Transforms.wrapNodes(editor, { type, children: [] } as CustomElement, {
    at: Editor.range(editor, [...parentPath, from], [...parentPath, to]),
    match: (node, path) =>
      path.length === parentPath.length + 1 &&
      Path.isParent(parentPath, path) &&
      path[path.length - 1] >= from &&
      path[path.length - 1] <= to &&
      isInlineContent(node),
  });
};

const removeIfEmpty = (editor: Editor, node: Element, path: Path): boolean => {
  if (node.children.length > 0) return false;
  Transforms.removeNodes(editor, { at: path });
  return true;
};

const normalizeTopLevel = (editor: Editor): boolean => {
  if (editor.children.length === 0) {
    Transforms.insertNodes(editor, emptyParagraph(), { at: [0] });
    return true;
  }
  for (let index = 0; index < editor.children.length; index += 1) {
    const node = editor.children[index];
    const path = [index];
    if (isInlineContent(node)) {
      wrapInlineRun(editor, [], index, BlockType.Paragraph);
      return true;
    }
    if (!Element.isElement(node)) continue;
    if (node.type === BlockType.QuoteLine || node.type === BlockType.CodeLine) {
      setTextBlockType(editor, path, BlockType.Paragraph);
      return true;
    }
    if (node.type === BlockType.ListItem) {
      Transforms.unwrapNodes(editor, { at: path });
      return true;
    }
    const previous = editor.children[index - 1];
    if (
      index > 0 &&
      MERGEABLE_CONTAINERS.includes(node.type) &&
      Element.isElement(previous) &&
      previous.type === node.type
    ) {
      Transforms.mergeNodes(editor, { at: path });
      return true;
    }
  }
  return false;
};

const normalizeList = (editor: Editor, list: ListElement, path: Path): boolean => {
  if (removeIfEmpty(editor, list, path)) return true;
  if (
    list.type === BlockType.OrderedList &&
    list.start !== undefined &&
    !(Number.isSafeInteger(list.start) && list.start > 1)
  ) {
    Transforms.unsetNodes(editor, 'start', { at: path });
    return true;
  }
  for (let index = 0; index < list.children.length; index += 1) {
    const child = list.children[index] as Descendant;
    const childPath = [...path, index];
    if (isListItem(child)) continue;
    if (isInlineContent(child)) {
      wrapInlineRun(editor, path, index, BlockType.Paragraph);
      return true;
    }
    if (
      isList(child) ||
      isType(child, BlockType.BlockQuote) ||
      isType(child, BlockType.CodeBlock)
    ) {
      // A list inside a list promotes its items; a quote or code block keeps its lines.
      Transforms.unwrapNodes(editor, { at: childPath });
      return true;
    }
    Transforms.wrapNodes(editor, { type: BlockType.ListItem, children: [] } as ListItemElement, {
      at: childPath,
    });
    return true;
  }
  return false;
};

const normalizeListItem = (editor: Editor, item: ListItemElement, path: Path): boolean => {
  if (removeIfEmpty(editor, item, path)) return true;
  const [first] = item.children;
  if (isInlineContent(first)) {
    wrapInlineRun(editor, path, 0, BlockType.Paragraph);
    return true;
  }
  if (isList(first)) {
    // An item with sub-items but no line of its own: its sub-items move up.
    Transforms.unwrapNodes(editor, { at: path });
    return true;
  }
  if (isType(first, BlockType.BlockQuote) || isType(first, BlockType.CodeBlock)) {
    Transforms.unwrapNodes(editor, { at: [...path, 0] });
    return true;
  }
  if (!isType(first, BlockType.Paragraph)) {
    setTextBlockType(editor, [...path, 0], BlockType.Paragraph);
    return true;
  }
  for (let index = 1; index < item.children.length; index += 1) {
    const child = item.children[index] as Descendant;
    if (isList(child)) continue;
    if (isInlineContent(child)) {
      wrapInlineRun(editor, path, index, BlockType.Paragraph);
      return true;
    }
    if (isType(child, BlockType.BlockQuote) || isType(child, BlockType.CodeBlock)) {
      Transforms.unwrapNodes(editor, { at: [...path, index] });
      return true;
    }
    // Every line in a list carries a marker: a further line is the next item.
    Transforms.splitNodes(editor, { at: [...path, index] });
    return true;
  }
  return false;
};

const normalizeLineContainer = (
  editor: Editor,
  container: Element,
  path: Path,
  lineType: BlockType
): boolean => {
  if (removeIfEmpty(editor, container, path)) return true;
  for (let index = 0; index < container.children.length; index += 1) {
    const child = container.children[index] as Descendant;
    const childPath = [...path, index];
    if (isType(child, lineType)) continue;
    if (isInlineContent(child)) {
      wrapInlineRun(editor, path, index, lineType);
      return true;
    }
    if (isTextBlock(child)) {
      setTextBlockType(editor, childPath, lineType);
      return true;
    }
    Transforms.unwrapNodes(editor, { at: childPath });
    return true;
  }
  return false;
};

const chipText = (node: CustomElement): string => {
  switch (node.type) {
    case BlockType.Mention:
      return node.name;
    case BlockType.Emoticon:
      return node.key.startsWith('mxc://') ? `:${node.shortcode}:` : node.key;
    case BlockType.Command:
      return `/${node.command}`;
    default:
      return Node.string(node);
  }
};

/** Code is plain text: no marks, links, mentions or emoji. */
const normalizeCodeLine = (editor: Editor, line: Element, path: Path): boolean => {
  for (let index = 0; index < line.children.length; index += 1) {
    const child = line.children[index] as Descendant;
    const childPath = [...path, index];
    if (Text.isText(child)) {
      const marked = MARKS.filter((mark) => (child as Record<string, unknown>)[mark]);
      if (marked.length > 0) {
        Transforms.unsetNodes(editor, [...marked], { at: childPath });
        return true;
      }
      continue;
    }
    if (isType(child, BlockType.Link)) {
      Transforms.unwrapNodes(editor, { at: childPath });
      return true;
    }
    if (Element.isElement(child)) {
      Editor.withoutNormalizing(editor, () => {
        Transforms.removeNodes(editor, { at: childPath });
        Transforms.insertNodes(editor, { text: chipText(child) }, { at: childPath });
      });
      return true;
    }
  }
  return false;
};

declare module 'slate' {
  interface BaseEditor {
    /** Markdown mode: typed list markers continue on the next line. */
    markdown?: boolean;
  }
}

/**
 * The composer's formatting plugin. Apply it outside link-edge escape so
 * breaks typed at a link's edge still land outside the link.
 */
export const withFormatting = (editor: Editor): Editor => {
  const { normalizeNode, insertBreak, insertSoftBreak, deleteBackward, deleteFragment } = editor;

  editor.normalizeNode = (entry, options) => {
    const [node, path] = entry;
    if (Editor.isEditor(node)) {
      if (normalizeTopLevel(editor)) return;
    } else if (Element.isElement(node)) {
      if (isList(node) && normalizeList(editor, node, path)) return;
      if (isListItem(node) && normalizeListItem(editor, node, path)) return;
      if (
        node.type === BlockType.BlockQuote &&
        normalizeLineContainer(editor, node, path, BlockType.QuoteLine)
      ) {
        return;
      }
      if (
        node.type === BlockType.CodeBlock &&
        normalizeLineContainer(editor, node, path, BlockType.CodeLine)
      ) {
        return;
      }
      if (node.type === BlockType.CodeLine && normalizeCodeLine(editor, node, path)) return;
      if (isListItem(node)) {
        // Same-type sub-lists side by side inside a list item join too.
        for (let index = 1; index < node.children.length; index += 1) {
          const previous = node.children[index - 1];
          const current = node.children[index];
          if (isList(previous) && isList(current) && previous.type === current.type) {
            Transforms.mergeNodes(editor, { at: [...path, index] });
            return;
          }
        }
      }
    }
    normalizeNode(entry, options);
  };

  editor.insertBreak = () => breakLine(editor, insertBreak);
  editor.insertSoftBreak = () => breakLine(editor, insertSoftBreak);

  editor.deleteBackward = (unit) => {
    const { selection } = editor;
    if (unit !== 'line' && selection && Range.isCollapsed(selection)) {
      const entry = textBlockAt(editor);
      if (entry && Point.equals(selection.anchor, Editor.start(editor, entry[1]))) {
        let handled = false;
        Editor.withoutNormalizing(editor, () => {
          handled = backspaceAtStart(editor, entry);
        });
        if (handled) return;
      }
    }
    deleteBackward(unit);
  };

  editor.deleteFragment = (options) => {
    deleteFragment(options);
    // Deleting everything leaves a plain line, not an empty list, quote or code block.
    const [first] = editor.children;
    if (
      editor.children.length === 1 &&
      Element.isElement(first) &&
      first.type !== BlockType.Paragraph &&
      isBlankContent(editor.children)
    ) {
      Editor.withoutNormalizing(editor, () => {
        Transforms.removeNodes(editor, { at: [0] });
        Transforms.insertNodes(editor, emptyParagraph(), { at: [0], select: true });
      });
    }
  };

  return editor;
};
