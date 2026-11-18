import { BasePoint, BaseRange, Editor, Element, Point, Range, Text, Transforms } from 'slate';
import { IS_NODE_MAP_DIRTY } from 'slate-dom';
import { ReactEditor } from 'slate-react';
import { BlockType, MarkType } from './types';
import {
  CommandElement,
  EmoticonElement,
  FormattedText,
  LinkElement,
  MentionElement,
} from './slate';
import { isBlankContent } from './formatting';

const ALL_MARK_TYPE: MarkType[] = [
  MarkType.Bold,
  MarkType.Code,
  MarkType.Italic,
  MarkType.Spoiler,
  MarkType.StrikeThrough,
  MarkType.Underline,
];

export const isMarkActive = (editor: Editor, format: MarkType) => {
  const marks = Editor.marks(editor);
  return marks ? marks[format] === true : false;
};

export const isAnyMarkActive = (editor: Editor) => {
  const marks = Editor.marks(editor);
  return marks && !!ALL_MARK_TYPE.find((type) => marks[type] === true);
};

export const toggleMark = (editor: Editor, format: MarkType) => {
  const isActive = isMarkActive(editor, format);

  if (isActive) {
    Editor.removeMark(editor, format);
  } else {
    Editor.addMark(editor, format, true);
  }
};

export const removeAllMark = (editor: Editor) => {
  ALL_MARK_TYPE.forEach((mark) => {
    if (isMarkActive(editor, mark)) Editor.removeMark(editor, mark);
  });
};

/** Empties the composer to one plain paragraph, with no marks pending. */
export const resetEditor = (editor: Editor) => {
  const hadSelection = !!editor.selection;
  Editor.withoutNormalizing(editor, () => {
    for (let index = editor.children.length - 1; index >= 0; index -= 1) {
      Transforms.removeNodes(editor, { at: [index] });
    }
    Transforms.insertNodes(
      editor,
      { type: BlockType.Paragraph, children: [{ text: '' }] },
      { at: [0] }
    );
  });
  if (hadSelection) Transforms.select(editor, Editor.start(editor, [0]));
  editor.marks = null;
};

export const resetEditorHistory = (editor: Editor) => {
  // eslint-disable-next-line no-param-reassign
  editor.history = {
    undos: [],
    redos: [],
  };
};

export const createMentionElement = (
  id: string,
  name: string,
  highlight: boolean,
  eventId?: string,
  viaServers?: string[]
): MentionElement => ({
  type: BlockType.Mention,
  id,
  eventId,
  viaServers,
  highlight,
  name,
  children: [{ text: '' }],
});

export const createEmoticonElement = (key: string, shortcode: string): EmoticonElement => ({
  type: BlockType.Emoticon,
  key,
  shortcode,
  children: [{ text: '' }],
});

export const createLinkElement = (
  href: string,
  children: string | FormattedText[]
): LinkElement => ({
  type: BlockType.Link,
  href,
  children: typeof children === 'string' ? [{ text: children }] : children,
});

const isLinkElement = (node: unknown): node is LinkElement =>
  Element.isElement(node) && node.type === BlockType.Link;

/**
 * Moves a collapsed selection that sits on a link's edge to just outside the
 * link. Slate otherwise keeps typing inside an inline it reached the end of,
 * so everything written after a pasted link became part of that link.
 */
export const escapeLinkEdge = (editor: Editor): void => {
  const { selection } = editor;
  if (!selection || !Range.isCollapsed(selection)) return;
  const link = Editor.above(editor, { at: selection, match: isLinkElement });
  if (!link) return;
  const [, path] = link;
  if (Editor.isEnd(editor, selection.anchor, path)) {
    Transforms.move(editor, { unit: 'offset' });
  } else if (Editor.isStart(editor, selection.anchor, path)) {
    Transforms.move(editor, { unit: 'offset', reverse: true });
  }
};

/**
 * Text, breaks and chips (emoji, mentions) entered at a link's edge land
 * outside the link. An insert at an explicit location is left alone.
 */
export const withLinkEdgeEscape = (editor: Editor): Editor => {
  const { insertText, insertBreak, insertSoftBreak, insertNodes } = editor;
  editor.insertNodes = (nodes, options) => {
    if (!options?.at) escapeLinkEdge(editor);
    insertNodes(nodes, options);
  };
  editor.insertText = (text, options) => {
    escapeLinkEdge(editor);
    insertText(text, options);
  };
  editor.insertBreak = () => {
    escapeLinkEdge(editor);
    insertBreak();
  };
  editor.insertSoftBreak = () => {
    escapeLinkEdge(editor);
    insertSoftBreak();
  };
  return editor;
};

export const createCommandElement = (command: string): CommandElement => ({
  type: BlockType.Command,
  command,
  children: [{ text: '' }],
});

export const replaceWithElement = (editor: Editor, selectRange: BaseRange, element: Element) => {
  Transforms.select(editor, selectRange);
  Transforms.insertNodes(editor, element);
  Transforms.collapse(editor, {
    edge: 'end',
  });
};

/** Frames to wait for slate-react to draw an edit before giving up on focus. */
const FOCUS_RENDER_FRAMES = 10;

/**
 * True while the DOM does not show the editor's latest value yet: operations
 * are waiting for Slate's flush, or Slate has flushed them but React has not
 * drawn the result. slate-react checks the same flag before it maps the DOM
 * selection, for the same reason.
 */
const isEditorDomStale = (editor: Editor): boolean =>
  editor.operations.length > 0 || IS_NODE_MAP_DIRTY.get(editor) === true;

/**
 * Focuses the editor once React has drawn the latest edit. Use it, never a
 * bare `ReactEditor.focus`, to refocus the editor after changing its value
 * from outside it (a picker, a menu, an effect).
 *
 * `ReactEditor.focus` maps the selection to the DOM. After an insert the
 * selection sits in nodes React has not drawn yet (an emoji or mention pick
 * leaves the cursor in the new empty text after the chip), and the mapping
 * threw "Cannot resolve a DOM node from Slate node: {"text":""}", which put
 * the whole screen on the error page. slate-dom only waits while operations
 * are pending, not for React to draw them, and a popover's focus trap calls
 * its close handler while React unmounts it, mid-commit, when the new nodes
 * are not mapped yet. So this waits a frame, and more frames while the DOM is
 * behind the value; if the edit is never drawn (the composer went away), it
 * leaves focus alone.
 */
export const focusEditorAfterRender = (editor: Editor): void => {
  let frames = FOCUS_RENDER_FRAMES;
  const attempt = () => {
    if (ReactEditor.isFocused(editor)) return;
    if (isEditorDomStale(editor)) {
      frames -= 1;
      if (frames > 0) requestAnimationFrame(attempt);
      return;
    }
    ReactEditor.focus(editor);
  };
  requestAnimationFrame(attempt);
};

export const moveCursor = (editor: Editor, withSpace?: boolean) => {
  Transforms.move(editor);
  if (withSpace) editor.insertText(' ');
};

interface PointUntilCharOptions {
  match: (char: string) => boolean;
  reverse?: boolean;
}
export const getPointUntilChar = (
  editor: Editor,
  cursorPoint: BasePoint,
  options: PointUntilCharOptions
): BasePoint | undefined => {
  let targetPoint: BasePoint | undefined;
  let prevPoint: BasePoint | undefined;
  let char: string | undefined;

  const pointItr = Editor.positions(editor, {
    at: {
      anchor: Editor.start(editor, []),
      focus: Editor.point(editor, cursorPoint, { edge: 'start' }),
    },
    unit: 'character',
    reverse: options.reverse,
  });

  // eslint-disable-next-line no-restricted-syntax
  for (const point of pointItr) {
    if (!Point.equals(point, cursorPoint) && prevPoint) {
      char = Editor.string(editor, { anchor: point, focus: prevPoint });

      if (options.match(char)) break;
      targetPoint = point;
    }
    prevPoint = point;
  }
  return targetPoint;
};

export const getPrevWorldRange = (editor: Editor): BaseRange | undefined => {
  const { selection } = editor;
  if (!selection || !Range.isCollapsed(selection)) return undefined;
  const [cursorPoint] = Range.edges(selection);
  const worldStartPoint = getPointUntilChar(editor, cursorPoint, {
    reverse: true,
    match: (char) => char === ' ',
  });
  return worldStartPoint && Editor.range(editor, worldStartPoint, cursorPoint);
};

/**
 * True when the composer holds nothing to send. A lone empty list, quote or
 * code block counts as empty, so it is not kept as the room's draft (where it
 * showed a stray bullet beside the placeholder on every visit).
 */
export const isEmptyEditor = (editor: Editor): boolean => {
  const firstChildren = editor.children[0];
  if (firstChildren && Element.isElement(firstChildren)) {
    return editor.children.length === 1 && isBlankContent(editor.children);
  }
  return false;
};

export const getBeginCommand = (editor: Editor): string | undefined => {
  const lineBlock = editor.children[0];
  if (!Element.isElement(lineBlock)) return undefined;
  if (lineBlock.type !== BlockType.Paragraph) return undefined;

  const [firstInline, secondInline] = lineBlock.children;
  const isEmptyText = Text.isText(firstInline) && firstInline.text.trim() === '';
  if (!isEmptyText) return undefined;
  if (Element.isElement(secondInline) && secondInline.type === BlockType.Command)
    return secondInline.command;
  return undefined;
};
