import { KeyboardEvent, useCallback, useLayoutEffect } from 'react';
import { Editor } from 'slate';
import { ReactEditor } from 'slate-react';
import { isEmptyEditor } from '../../components/editor';
import {
  composerArrowUpLeavesForTimeline,
  focusLastOwnTimelineMessage,
  registerTimelineComposerFocus,
} from './nativeTimelineKeyboardSelection';

/**
 * The message box's half of Slack's quick edit. It offers itself as the place
 * focus returns to from its timeline, and returns the key handler that sends
 * Up from an empty box to the user's last message there. The handler answers
 * true when it moved focus (the key is then spent).
 */
export const useComposerTimelineSelection = (
  editor: Editor,
  roomId: string,
  threadRootId: string | undefined
): ((evt: KeyboardEvent, suggestionsOpen: boolean) => boolean) => {
  // In a layout effect, like the app shortcut handlers: registered in the
  // commit that shows the box, so Escape on a message never finds nobody.
  useLayoutEffect(
    () => registerTimelineComposerFocus(roomId, threadRootId, () => ReactEditor.focus(editor)),
    [editor, roomId, threadRootId]
  );

  return useCallback(
    (evt, suggestionsOpen) => {
      if (
        !composerArrowUpLeavesForTimeline(evt, {
          empty: isEmptyEditor(editor),
          suggestionsOpen,
          composing: evt.nativeEvent.isComposing,
        })
      ) {
        return false;
      }
      if (!focusLastOwnTimelineMessage(roomId, threadRootId)) return false;
      evt.preventDefault();
      return true;
    },
    [editor, roomId, threadRootId]
  );
};
