/* eslint-disable no-param-reassign */
import { Editor } from 'slate';
import { BlockType } from './types';
import { withFormatting } from './formatting';
import { withLinkEdgeEscape } from './utils';

const INLINE_TYPES = [BlockType.Mention, BlockType.Emoticon, BlockType.Link, BlockType.Command];
const VOID_TYPES = [BlockType.Mention, BlockType.Emoticon, BlockType.Command];

const withComposerElements = (editor: Editor): Editor => {
  const { isInline, isVoid } = editor;
  editor.isInline = (element) => INLINE_TYPES.includes(element.type) || isInline(element);
  editor.isVoid = (element) => VOID_TYPES.includes(element.type) || isVoid(element);
  return editor;
};

/**
 * The composer's editor plugins, in order: chips and links are inline, block
 * formatting follows docs/composer-formatting-model.md, and text typed at a
 * link's edge lands outside the link (outermost, so breaks escape first).
 */
export const withComposer = (editor: Editor): Editor =>
  withLinkEdgeEscape(withFormatting(withComposerElements(editor)));
