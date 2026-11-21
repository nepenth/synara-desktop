import { Editor, Point, Range, Transforms } from 'slate';
import { IS_NODE_MAP_DIRTY } from 'slate-dom';
import { ReactEditor } from 'slate-react';

/** Frames to wait for slate-react to draw the replacement before giving up on the caret. */
const CARET_RESTORE_FRAMES = 10;

/**
 * Replaces `range` with `text` and leaves the caret where the user was typing.
 * A caret inside or at the end of the replaced text ends up after the new
 * text; a caret elsewhere keeps its place.
 */
export const replaceTextRange = (editor: Editor, range: Range, text: string): void => {
  const { selection } = editor;
  const caretInRange =
    !!selection && Range.isCollapsed(selection) && Range.includes(range, selection.anchor);
  const start = Editor.pointRef(editor, Range.start(range), { affinity: 'backward' });
  const end = Editor.pointRef(editor, Range.end(range), { affinity: 'forward' });
  // Delete and insert at explicit points: `Transforms.insertText` over a range
  // moves the selection to it, wherever the caret was.
  Editor.withoutNormalizing(editor, () => {
    Transforms.delete(editor, { at: range });
    if (start.current) Transforms.insertText(editor, text, { at: start.current });
  });
  start.unref();
  const after = end.unref();
  if (caretInRange && after && !Point.equals(editor.selection?.anchor ?? after, after)) {
    Transforms.select(editor, after);
  }
};

/**
 * Puts the DOM caret back at the editor's selection when the browser dropped
 * it. slate-react only syncs the DOM selection while it renders, and WebKit
 * clears it afterwards when the text node it was typing into was redrawn.
 */
const restoreDroppedCaret = (editor: Editor): void => {
  let frames = CARET_RESTORE_FRAMES;
  const attempt = () => {
    if (!ReactEditor.isMounted(editor)) return;
    if (editor.operations.length > 0 || IS_NODE_MAP_DIRTY.get(editor) === true) {
      frames -= 1;
      if (frames > 0) requestAnimationFrame(attempt);
      return;
    }
    const { selection } = editor;
    if (!selection) return;
    const el = ReactEditor.toDOMNode(editor, editor);
    const root = ReactEditor.findDocumentOrShadowRoot(editor);
    if (root.activeElement !== el) return;
    const domSelection = el.ownerDocument.getSelection();
    if (!domSelection) return;
    const inEditor =
      domSelection.rangeCount > 0 &&
      !!domSelection.anchorNode &&
      el.contains(domSelection.anchorNode);
    if (inEditor) return;
    try {
      const domRange = ReactEditor.toDOMRange(editor, selection);
      domSelection.setBaseAndExtent(
        domRange.startContainer,
        domRange.startOffset,
        domRange.endContainer,
        domRange.endOffset
      );
    } catch {
      // The selection is not drawn; leave the caret to the next click.
    }
  };
  requestAnimationFrame(attempt);
};

const domRangeText = (editor: Editor, range: StaticRange): string | undefined => {
  try {
    const domRange = ReactEditor.getWindow(editor).document.createRange();
    domRange.setStart(range.startContainer, range.startOffset);
    domRange.setEnd(range.endContainer, range.endOffset);
    return domRange.toString();
  } catch {
    return undefined;
  }
};

/**
 * Applies a browser text replacement (`insertReplacementText`: macOS smart
 * dashes and quotes, text replacements, autocorrect, a picked spelling
 * suggestion) to the editor. Returns true when the event was handled here.
 *
 * WebKit runs these while it inserts the next typed character, and
 * slate-react lets the browser insert plain letters and spaces itself, adding
 * them to its own value afterwards. So the event arrives when the DOM is one
 * character ahead of the value. slate-react then read the DOM caret as its
 * selection, a position past the end of its text, and inserted the
 * replacement there without removing the replaced text ("--" became "--—").
 * The redraw also dropped the DOM caret, so typing stopped until a click.
 *
 * The replaced range comes from the event and lies before the pending
 * character, so it maps to the value as it is. The replacement is applied
 * only when the value holds the same text as the DOM there; otherwise it is
 * dropped and the typed text stays.
 */
export const handleReplacementText = (editor: Editor, event: InputEvent): boolean => {
  if (event.inputType !== 'insertReplacementText') return false;
  const text = event.dataTransfer?.getData('text/plain') ?? event.data;
  const [targetRange] = event.getTargetRanges();
  if (typeof text !== 'string' || !targetRange) return false;

  event.preventDefault();
  const range = ReactEditor.toSlateRange(editor, targetRange, {
    exactMatch: false,
    suppressThrow: true,
  });
  const replaced = domRangeText(editor, targetRange);
  if (range && replaced !== undefined && Editor.string(editor, range) === replaced) {
    replaceTextRange(editor, range, text);
  }
  restoreDroppedCaret(editor);
  return true;
};
