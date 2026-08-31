/* eslint-disable no-param-reassign */
import React, {
  ClipboardEventHandler,
  KeyboardEventHandler,
  ReactNode,
  forwardRef,
  useCallback,
  useState,
} from 'react';
import { Box, Scroll, Text } from 'folds';
import { Descendant, Editor, createEditor } from 'slate';
import {
  Slate,
  Editable,
  withReact,
  RenderLeafProps,
  RenderElementProps,
  RenderPlaceholderProps,
} from 'slate-react';
import { withHistory } from 'slate-history';
import { BlockType } from './types';
import { RenderElement, RenderLeaf } from './Elements';
import { CustomElement } from './slate';
import * as css from './Editor.css';
import { toggleKeyboardShortcut } from './keyboard';
import { enableDesktopSpellcheck } from '../../utils/desktop';

const initialValue: CustomElement[] = [
  {
    type: BlockType.Paragraph,
    children: [{ text: '' }],
  },
];

const withInline = (editor: Editor): Editor => {
  const { isInline } = editor;

  editor.isInline = (element) =>
    [BlockType.Mention, BlockType.Emoticon, BlockType.Link, BlockType.Command].includes(
      element.type
    ) || isInline(element);

  return editor;
};

const withVoid = (editor: Editor): Editor => {
  const { isVoid } = editor;

  editor.isVoid = (element) =>
    [BlockType.Mention, BlockType.Emoticon, BlockType.Command].includes(element.type) ||
    isVoid(element);

  return editor;
};

export const useEditor = (): Editor => {
  const [editor] = useState(() => withInline(withVoid(withReact(withHistory(createEditor())))));
  return editor;
};

export type EditorChangeHandler = (value: Descendant[]) => void;
type CustomEditorProps = {
  className?: string;
  editableName?: string;
  top?: ReactNode;
  bottom?: ReactNode;
  before?: ReactNode;
  after?: ReactNode;
  maxHeight?: string;
  spellCheck?: boolean;
  editor: Editor;
  placeholder?: string;
  onKeyDown?: KeyboardEventHandler;
  onKeyUp?: KeyboardEventHandler;
  onChange?: EditorChangeHandler;
  onPaste?: ClipboardEventHandler;
};
export const CustomEditor = forwardRef<HTMLDivElement, CustomEditorProps>(
  (
    {
      className,
      editableName,
      top,
      bottom,
      before,
      after,
      maxHeight = '50vh',
      spellCheck = true,
      editor,
      placeholder,
      onKeyDown,
      onKeyUp,
      onChange,
      onPaste,
    },
    ref
  ) => {
    const renderElement = useCallback(
      (props: RenderElementProps) => <RenderElement {...props} />,
      []
    );

    const renderLeaf = useCallback((props: RenderLeafProps) => <RenderLeaf {...props} />, []);

    const handleKeydown: KeyboardEventHandler = useCallback(
      (evt) => {
        onKeyDown?.(evt);
        const shortcutToggled = toggleKeyboardShortcut(editor, evt);
        if (shortcutToggled) evt.preventDefault();
      },
      [editor, onKeyDown]
    );

    const renderPlaceholder = useCallback(
      ({ attributes, children }: RenderPlaceholderProps) => (
        <span {...attributes} className={css.EditorPlaceholderContainer}>
          {/* Inner component to style the actual text position and appearance */}
          <Text as="span" className={css.EditorPlaceholderTextVisual} truncate>
            {children}
          </Text>
        </span>
      ),
      []
    );

    const spellCheckLanguage =
      typeof navigator === 'undefined' || !navigator.language ? undefined : navigator.language;

    return (
      <div className={className ? `${css.Editor} ${className}` : css.Editor} ref={ref}>
        <Slate editor={editor} initialValue={initialValue} onChange={onChange}>
          {top}
          <Box alignItems="Start">
            {before && (
              <Box className={css.EditorOptions} alignItems="Center" gap="100" shrink="No">
                {before}
              </Box>
            )}
            <Box className={css.EditorTextareaArea}>
              <Scroll
                className={css.EditorTextareaScroll}
                variant="SurfaceVariant"
                style={{ maxHeight }}
                size="300"
                visibility="Hover"
                hideTrack
              >
                <Editable
                  data-editable-name={editableName}
                  className={`${css.EditorTextarea} ${
                    after ? css.EditorTextareaWithFloatingOptions : ''
                  }`}
                  placeholder={placeholder}
                  renderPlaceholder={renderPlaceholder}
                  renderElement={renderElement}
                  renderLeaf={renderLeaf}
                  lang={spellCheck ? spellCheckLanguage : undefined}
                  autoCorrect={spellCheck ? 'on' : 'off'}
                  autoCapitalize={spellCheck ? 'sentences' : 'none'}
                  spellCheck={spellCheck}
                  onFocus={() => {
                    if (spellCheck) void enableDesktopSpellcheck();
                  }}
                  onKeyDown={handleKeydown}
                  onKeyUp={onKeyUp}
                  onPaste={onPaste}
                />
              </Scroll>
              {after && <Box className={css.EditorFloatingOptions}>{after}</Box>}
            </Box>
          </Box>
          {bottom}
        </Slate>
      </div>
    );
  }
);
