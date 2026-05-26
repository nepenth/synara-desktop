import { isKeyHotkey } from 'is-hotkey';
import { KeyboardEvent } from 'react';
import { Editor, Element as SlateElement, Node, Path, Point, Range, Transforms } from 'slate';
import { isAnyMarkActive, isBlockActive, removeAllMark, toggleBlock, toggleMark } from './utils';
import { BlockType, MarkType } from './types';
import type {
  ListItemElement,
  OrderedListElement,
  ParagraphElement,
  UnorderedListElement,
} from './slate';

export const INLINE_HOTKEYS: Record<string, MarkType> = {
  'mod+b': MarkType.Bold,
  'mod+i': MarkType.Italic,
  'mod+u': MarkType.Underline,
  'mod+s': MarkType.StrikeThrough,
  'mod+[': MarkType.Code,
  'mod+h': MarkType.Spoiler,
};
const INLINE_KEYS = Object.keys(INLINE_HOTKEYS);

export const BLOCK_HOTKEYS: Record<string, BlockType> = {
  'mod+7': BlockType.OrderedList,
  'mod+8': BlockType.UnorderedList,
  "mod+'": BlockType.BlockQuote,
  'mod+;': BlockType.CodeBlock,
};
const BLOCK_KEYS = Object.keys(BLOCK_HOTKEYS);
const isHeading1 = isKeyHotkey('mod+1');
const isHeading2 = isKeyHotkey('mod+2');
const isHeading3 = isKeyHotkey('mod+3');
const isEnter = isKeyHotkey('enter');
const isShiftEnter = isKeyHotkey('shift+enter');
const isTab = isKeyHotkey('tab');
const isShiftTab = isKeyHotkey('shift+tab');

const isLineBlock = (node: SlateElement): boolean =>
  node.type === BlockType.CodeLine ||
  node.type === BlockType.QuoteLine ||
  node.type === BlockType.ListItem;

const isEmptyElement = (editor: Editor, node: SlateElement): boolean =>
  Node.string(node).length === 0 && Editor.isEmpty(editor, node);

const isList = (node: unknown): node is OrderedListElement | UnorderedListElement =>
  SlateElement.isElement(node) &&
  (node.type === BlockType.OrderedList || node.type === BlockType.UnorderedList);

const isListItem = (node: unknown): node is ListItemElement =>
  SlateElement.isElement(node) && node.type === BlockType.ListItem;

const normalizeListItemChildrenForNested = (
  children: ListItemElement['children']
): ListItemElement['children'] => {
  const inlineChildren = children.filter((child) => !isList(child));
  const nestedLists = children.filter(isList);

  const blockChildren: ListItemElement['children'] = [];
  if (
    inlineChildren.length > 0 &&
    !(
      inlineChildren.length === 1 &&
      SlateElement.isElement(inlineChildren[0]) &&
      inlineChildren[0].type === BlockType.Paragraph
    )
  ) {
    blockChildren.push({
      type: BlockType.Paragraph,
      children: inlineChildren,
    } as ParagraphElement);
  } else {
    blockChildren.push(...inlineChildren);
  }
  blockChildren.push(...nestedLists);
  return blockChildren;
};

const getActiveListItem = (editor: Editor): [SlateElement, Path] | undefined => {
  const [entry] = Editor.nodes(editor, {
    match: isListItem,
    mode: 'lowest',
  });
  if (!entry) return undefined;
  return entry as [SlateElement, Path];
};

const getFirstEditableTextPath = (path: Path, listItem: ListItemElement): Path => {
  const firstChild = listItem.children[0];
  if (SlateElement.isElement(firstChild) && firstChild.type === BlockType.Paragraph) {
    return [...path, 0, 0];
  }
  return [...path, 0];
};

const selectListItemText = (editor: Editor, path: Path, listItem: ListItemElement): void => {
  const textPath = getFirstEditableTextPath(path, listItem);
  if (!Node.has(editor, textPath)) return;
  const textStart = Editor.start(editor, textPath);
  const textEnd = Editor.end(editor, textPath);
  Transforms.select(editor, {
    anchor: textEnd,
    focus: textEnd,
  });
  if (Point.equals(textStart, textEnd)) {
    Transforms.collapse(editor, { edge: 'end' });
  }
};

const exitEmptyTopLevelListItem = (editor: Editor, listItemPath: Path): boolean => {
  const movingListItem = Node.get(editor, listItemPath);
  if (!isListItem(movingListItem) || Node.string(movingListItem).length > 0) return false;

  const parentListPath = Path.parent(listItemPath);
  const parentList = Node.get(editor, parentListPath);
  if (!isList(parentList)) return false;

  const parentPath = Path.parent(parentListPath);
  const parent =
    parentPath.length > 0 && Node.has(editor, parentPath)
      ? Node.get(editor, parentPath)
      : undefined;
  if (isListItem(parent)) return false;

  const paragraph: ParagraphElement = {
    type: BlockType.Paragraph,
    children: [{ text: '' }],
  };
  const insertPath = parentList.children.length === 1 ? parentListPath : Path.next(parentListPath);

  Editor.withoutNormalizing(editor, () => {
    if (parentList.children.length === 1) {
      Transforms.removeNodes(editor, { at: parentListPath });
    } else {
      Transforms.removeNodes(editor, { at: listItemPath });
    }
    Transforms.insertNodes(editor, paragraph, { at: insertPath });
    Transforms.select(editor, Editor.start(editor, insertPath));
  });

  return true;
};

export const indentListItem = (editor: Editor): boolean => {
  const entry = getActiveListItem(editor);
  if (!entry) return false;

  const [, listItemPath] = entry;
  const parentListPath = Path.parent(listItemPath);
  const parentList = Node.get(editor, parentListPath);
  if (!isList(parentList)) return false;

  const itemIndex = listItemPath[listItemPath.length - 1];
  if (itemIndex === 0) return false;

  const previousItemPath = [...parentListPath, itemIndex - 1];
  const previousItem = Node.get(editor, previousItemPath);
  if (!isListItem(previousItem)) return false;
  const movingListItem = Node.get(editor, listItemPath);
  if (!isListItem(movingListItem)) return false;

  const existingNestedIndex = previousItem.children.findIndex(
    (child) => isList(child) && child.type === parentList.type
  );

  Editor.withoutNormalizing(editor, () => {
    const normalizedPreviousChildren = normalizeListItemChildrenForNested(previousItem.children);
    const normalizedNestedIndex = normalizedPreviousChildren.findIndex(
      (child) => isList(child) && child.type === parentList.type
    );

    if (existingNestedIndex >= 0) {
      const nestedList = normalizedPreviousChildren[normalizedNestedIndex];
      if (!isList(nestedList)) return;
      const nextListItemPath = [
        ...previousItemPath,
        normalizedNestedIndex,
        nestedList.children.length,
      ];

      normalizedPreviousChildren[normalizedNestedIndex] = {
        ...nestedList,
        children: [...nestedList.children, movingListItem],
      };
      const updatedPreviousItem = {
        ...previousItem,
        children: normalizedPreviousChildren,
      };
      Transforms.removeNodes(editor, { at: listItemPath });
      Transforms.removeNodes(editor, { at: previousItemPath });
      Transforms.insertNodes(editor, updatedPreviousItem, { at: previousItemPath });
      selectListItemText(editor, nextListItemPath, movingListItem);
      return;
    }

    const nestedIndex = normalizedPreviousChildren.length;
    const nestedList: OrderedListElement | UnorderedListElement =
      parentList.type === BlockType.OrderedList
        ? {
            type: BlockType.OrderedList,
            children: [movingListItem],
          }
        : {
            type: BlockType.UnorderedList,
            children: [movingListItem],
          };
    normalizedPreviousChildren.push(nestedList);
    const updatedPreviousItem = {
      ...previousItem,
      children: normalizedPreviousChildren,
    };
    const nextListItemPath = [...previousItemPath, nestedIndex, 0];
    Transforms.removeNodes(editor, { at: listItemPath });
    Transforms.removeNodes(editor, { at: previousItemPath });
    Transforms.insertNodes(editor, updatedPreviousItem, { at: previousItemPath });
    selectListItemText(editor, nextListItemPath, movingListItem);
  });
  return true;
};

export const outdentListItem = (editor: Editor): boolean => {
  const entry = getActiveListItem(editor);
  if (!entry) return false;

  const [, listItemPath] = entry;
  const movingListItem = Node.get(editor, listItemPath);
  if (!isListItem(movingListItem)) return false;
  const parentListPath = Path.parent(listItemPath);
  const parentList = Node.get(editor, parentListPath);
  if (!isList(parentList)) return false;

  const parentListItemPath = Path.parent(parentListPath);
  const parentListItem = Node.has(editor, parentListItemPath)
    ? Node.get(editor, parentListItemPath)
    : undefined;
  if (!isListItem(parentListItem)) return false;

  const targetListPath = Path.parent(parentListItemPath);
  const targetIndex = parentListItemPath[parentListItemPath.length - 1] + 1;
  const nextListItemPath = [...targetListPath, targetIndex];
  Editor.withoutNormalizing(editor, () => {
    const removeParentList = parentList.children.length === 1;
    Transforms.removeNodes(editor, { at: listItemPath });
    Transforms.insertNodes(editor, movingListItem, { at: [...targetListPath, targetIndex] });
    if (removeParentList && Node.has(editor, parentListPath)) {
      Transforms.removeNodes(editor, { at: parentListPath });
    }
    selectListItemText(editor, nextListItemPath, movingListItem);
  });

  return true;
};

/**
 * @return boolean true if shortcut is toggled.
 */
export const toggleKeyboardShortcut = (editor: Editor, event: KeyboardEvent<Element>): boolean => {
  if (
    (isTab(event) || isShiftTab(event)) &&
    editor.selection &&
    Range.isCollapsed(editor.selection)
  ) {
    const changed = isShiftTab(event) ? outdentListItem(editor) : indentListItem(editor);
    if (changed) {
      event.preventDefault();
      return true;
    }
  }

  if (isShiftEnter(event) && editor.selection && Range.isCollapsed(editor.selection)) {
    event.preventDefault();
    const [listItemEntry] = Editor.nodes(editor, {
      match: isListItem,
      mode: 'lowest',
    });
    if (listItemEntry) {
      const [, listItemPath] = listItemEntry as [ListItemElement, Path];
      const listItem = Node.get(editor, listItemPath);
      if (isListItem(listItem) && Node.string(listItem).length === 0) {
        if (outdentListItem(editor) || exitEmptyTopLevelListItem(editor, listItemPath)) {
          return true;
        }
      }
      editor.insertBreak();
      return true;
    }
    editor.insertBreak();
    return true;
  }

  if (isEnter(event) && editor.selection && Range.isCollapsed(editor.selection)) {
    const startPoint = Range.start(editor.selection);
    const [parentNode] = Editor.parent(editor, startPoint);

    if (
      !Editor.isEditor(parentNode) &&
      isLineBlock(parentNode) &&
      isEmptyElement(editor, parentNode)
    ) {
      event.preventDefault();
      toggleBlock(editor, BlockType.Paragraph);
      return true;
    }
  }

  if (isKeyHotkey('backspace', event) && editor.selection && Range.isCollapsed(editor.selection)) {
    const startPoint = Range.start(editor.selection);
    if (startPoint.offset !== 0) return false;

    const [parentNode, parentPath] = Editor.parent(editor, startPoint);
    const parentLocation = { at: parentPath };
    const [previousNode] = Editor.previous(editor, parentLocation) ?? [];
    const [nextNode] = Editor.next(editor, parentLocation) ?? [];

    if (Editor.isEditor(parentNode)) return false;

    if (parentNode.type === BlockType.Heading) {
      toggleBlock(editor, BlockType.Paragraph);
      return true;
    }
    if (
      parentNode.type === BlockType.CodeLine ||
      parentNode.type === BlockType.QuoteLine ||
      parentNode.type === BlockType.ListItem
    ) {
      // exit formatting only when line block
      // is first of last of it's parent
      if (!previousNode || !nextNode) {
        toggleBlock(editor, BlockType.Paragraph);
        return true;
      }
    }
    // Unwrap paragraph children to put them
    // in previous none paragraph element
    if (SlateElement.isElement(previousNode) && previousNode.type !== BlockType.Paragraph) {
      Transforms.unwrapNodes(editor, {
        at: startPoint,
      });
    }
    Editor.deleteBackward(editor);
    return true;
  }

  if (isKeyHotkey('mod+e', event) || isKeyHotkey('escape', event)) {
    if (isAnyMarkActive(editor)) {
      removeAllMark(editor);
      return true;
    }

    if (!isBlockActive(editor, BlockType.Paragraph)) {
      toggleBlock(editor, BlockType.Paragraph);
      return true;
    }
    return false;
  }

  const blockToggled = BLOCK_KEYS.find((hotkey) => {
    if (isKeyHotkey(hotkey, event)) {
      event.preventDefault();
      toggleBlock(editor, BLOCK_HOTKEYS[hotkey]);
      return true;
    }
    return false;
  });
  if (blockToggled) return true;
  if (isHeading1(event)) {
    toggleBlock(editor, BlockType.Heading, { level: 1 });
    return true;
  }
  if (isHeading2(event)) {
    toggleBlock(editor, BlockType.Heading, { level: 2 });
    return true;
  }
  if (isHeading3(event)) {
    toggleBlock(editor, BlockType.Heading, { level: 3 });
    return true;
  }

  const inlineToggled = isBlockActive(editor, BlockType.CodeBlock)
    ? false
    : INLINE_KEYS.find((hotkey) => {
        if (isKeyHotkey(hotkey, event)) {
          event.preventDefault();
          toggleMark(editor, INLINE_HOTKEYS[hotkey]);
          return true;
        }
        return false;
      });
  return !!inlineToggled;
};
