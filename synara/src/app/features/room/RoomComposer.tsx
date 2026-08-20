import React, { ClipboardEventHandler, KeyboardEventHandler, ReactNode, forwardRef } from 'react';
import { Line } from 'folds';
import { Editor } from 'slate';

import { CustomEditor, EditorChangeHandler, Toolbar } from '../../components/editor';
import * as css from './RoomComposer.css';

type RoomComposerProps = {
  editor: Editor;
  editableName?: string;
  placeholder?: string;
  maxHeight?: string;
  replyPreview?: ReactNode;
  leadingAction?: ReactNode;
  floatingActions?: ReactNode;
  toolbarVisible?: boolean;
  toolbar?: ReactNode;
  onKeyDown?: KeyboardEventHandler;
  onKeyUp?: KeyboardEventHandler;
  onPaste?: ClipboardEventHandler;
  onChange?: EditorChangeHandler;
};

export const RoomComposer = forwardRef<HTMLDivElement, RoomComposerProps>(
  (
    {
      editor,
      editableName,
      placeholder,
      maxHeight,
      replyPreview,
      leadingAction,
      floatingActions,
      toolbarVisible,
      toolbar,
      onKeyDown,
      onKeyUp,
      onPaste,
      onChange,
    },
    ref
  ) => (
    <section className={css.RoomComposer} aria-label="Message composer">
      <CustomEditor
        ref={ref}
        editableName={editableName}
        editor={editor}
        placeholder={placeholder}
        maxHeight={maxHeight}
        onKeyDown={onKeyDown}
        onKeyUp={onKeyUp}
        onPaste={onPaste}
        onChange={onChange}
        top={replyPreview ? <div className={css.RoomComposerReply}>{replyPreview}</div> : undefined}
        before={
          leadingAction ? (
            <div className={css.RoomComposerLeadingAction}>{leadingAction}</div>
          ) : undefined
        }
        after={
          floatingActions ? (
            <div className={css.RoomComposerFloatingActions}>{floatingActions}</div>
          ) : undefined
        }
        bottom={
          toolbarVisible ? (
            <div className={css.RoomComposerToolbar}>
              <Line variant="Secondary" size="300" />
              {toolbar ?? <Toolbar />}
            </div>
          ) : undefined
        }
      />
    </section>
  )
);
