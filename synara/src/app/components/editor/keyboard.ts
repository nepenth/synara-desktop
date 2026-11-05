import { isKeyHotkey } from 'is-hotkey';
import { KeyboardEvent } from 'react';
import { Editor } from 'slate';
import {
  isBlockActive,
  toggleBlock,
  exitFormatting,
  indentListItem,
  outdentListItem,
} from './formatting';
import { toggleMark } from './utils';
import { BlockType, MarkType } from './types';

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
const HEADING_HOTKEYS: Record<string, 1 | 2 | 3> = { 'mod+1': 1, 'mod+2': 2, 'mod+3': 3 };
const HEADING_KEYS = Object.keys(HEADING_HOTKEYS);

/**
 * Formatting shortcuts that have no input event of their own. Enter,
 * Shift+Enter and Backspace are not handled here: Slate routes them through
 * `insertBreak`, `insertSoftBreak` and `deleteBackward`, which `withFormatting`
 * implements for every input path.
 *
 * @return boolean true if the event was used.
 */
export const toggleKeyboardShortcut = (editor: Editor, event: KeyboardEvent<Element>): boolean => {
  if (isKeyHotkey('tab', event) || isKeyHotkey('shift+tab', event)) {
    // Outside a list Tab keeps moving focus.
    const changed = isKeyHotkey('shift+tab', event)
      ? outdentListItem(editor)
      : indentListItem(editor);
    if (changed) event.preventDefault();
    return changed;
  }

  // Escape is the composer's first (closing suggestions, a reply, the
  // message editor); it exits formatting only when nothing else used it.
  if ((isKeyHotkey('escape', event) && !event.defaultPrevented) || isKeyHotkey('mod+e', event)) {
    if (exitFormatting(editor)) {
      event.preventDefault();
      return true;
    }
    return false;
  }

  const blockKey = BLOCK_KEYS.find((hotkey) => isKeyHotkey(hotkey, event));
  if (blockKey) {
    event.preventDefault();
    toggleBlock(editor, BLOCK_HOTKEYS[blockKey]);
    return true;
  }

  const headingKey = HEADING_KEYS.find((hotkey) => isKeyHotkey(hotkey, event));
  if (headingKey) {
    event.preventDefault();
    toggleBlock(editor, BlockType.Heading, { level: HEADING_HOTKEYS[headingKey] });
    return true;
  }

  const markKey = INLINE_KEYS.find((hotkey) => isKeyHotkey(hotkey, event));
  if (markKey) {
    event.preventDefault();
    // Code is plain text; marks there would be stripped anyway.
    if (!isBlockActive(editor, BlockType.CodeBlock)) toggleMark(editor, INLINE_HOTKEYS[markKey]);
    return true;
  }
  return false;
};
