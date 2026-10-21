import React, {
  ClipboardEventHandler,
  KeyboardEventHandler,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { Box, Chip, Icon, IconButton, Icons, Line, Spinner, Text, config } from 'folds';
import { Descendant, Editor, Transforms } from 'slate';
import { ReactEditor } from 'slate-react';
import { isKeyHotkey } from 'is-hotkey';
import {
  AUTOCOMPLETE_PREFIXES,
  AutocompletePrefix,
  AutocompleteQuery,
  AutocompleteSelectFirstRef,
  CustomEditor,
  EmoticonAutocomplete,
  RoomMentionAutocomplete,
  Toolbar,
  UserMentionAutocomplete,
  customHtmlEqualsPlainText,
  getAutocompleteQuery,
  getMentions,
  getPrevWorldRange,
  htmlToEditorInput,
  insertClipboardData,
  plainToEditorInput,
  shouldPreventDefaultForClipboardInsert,
  toMatrixCustomHTML,
  toPlainText,
  trimCustomHtml,
  useEditor,
} from '../../components/editor';
import { renderComposerHtml } from '../../components/editor/composerMarkdown';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { useOptionalRoom } from '../../hooks/useRoom';
import * as depthCss from '../../styles/Depth.css';
import * as css from './RoomComposer.css';
import { editMessageWithNativeDesktopOwner } from './nativeEditMessage';
import {
  dropNativeEditDraft,
  isNativeEditUnchanged,
  keepNativeEditDraft,
  nativeEditInitialHtml,
  nativeEditMsgType,
  takeNativeEditDraft,
} from './nativeTimelineEdit';
import {
  nativeTimelineActionErrorMessage,
  TIMELINE_ACTION_FAILED,
} from './nativeTimelineErrorCopy';

type NativeTimelineMessageEditorProps = {
  roomId: string;
  eventId: string;
  body: string;
  formattedBody?: string;
  messageType?: string;
  /** Leave edit mode: after a save, or when the user cancels. */
  onClose: () => void;
};

/**
 * Edits a message in place with the room composer's Slate editor, so its
 * formatting, mentions and msgtype survive the edit (review D76). It sits in
 * the row rather than in a popover, so clicking elsewhere never discards it.
 */
export function NativeTimelineMessageEditor({
  roomId,
  eventId,
  body,
  formattedBody,
  messageType,
  onClose,
}: NativeTimelineMessageEditorProps) {
  const editor = useEditor();
  const room = useOptionalRoom();
  const [enterForNewline] = useSetting(settingsAtom, 'enterForNewline');
  const [isMarkdown] = useSetting(settingsAtom, 'isMarkdown');
  const [globalToolbar] = useSetting(settingsAtom, 'editorToolbar');
  const [toolbar, setToolbar] = useState(globalToolbar);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const [autocompleteQuery, setAutocompleteQuery] =
    useState<AutocompleteQuery<AutocompletePrefix>>();
  const autocompleteSelectFirst: AutocompleteSelectFirstRef = useRef(undefined);
  const savingRef = useRef(false);
  const closedRef = useRef(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const imagePackRooms = React.useMemo(() => [roomId], [roomId]);

  // Start from a draft kept while the row was scrolled away, else the message.
  useEffect(() => {
    const draft = takeNativeEditDraft<Descendant[]>(roomId, eventId);
    const html = nativeEditInitialHtml(formattedBody);
    const initialValue =
      draft ?? (html ? htmlToEditorInput(html, isMarkdown) : plainToEditorInput(body, isMarkdown));
    Transforms.select(editor, { anchor: Editor.start(editor, []), focus: Editor.end(editor, []) });
    editor.insertFragment(initialValue);
    ReactEditor.focus(editor);
    // The initial content is read once, when the editor opens.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor]);

  useEffect(
    () => () => {
      if (!closedRef.current) keepNativeEditDraft(roomId, eventId, editor.children);
    },
    [editor, eventId, roomId]
  );

  const close = useCallback(() => {
    closedRef.current = true;
    dropNativeEditDraft(roomId, eventId);
    // Focus goes back to the message, not to the page, when the editor goes.
    const surface = rootRef.current?.closest<HTMLElement>('[data-native-timeline-row-surface]');
    onClose();
    if (surface) window.requestAnimationFrame(() => surface.focus({ preventScroll: true }));
  }, [eventId, onClose, roomId]);

  const save = useCallback(async () => {
    if (savingRef.current) return;
    const plainText = toPlainText(editor.children, isMarkdown).trim();
    if (!plainText) {
      setError('A message can not be empty. Use Redact to remove it instead.');
      return;
    }
    savingRef.current = true;
    setSaving(true);
    setError(undefined);
    try {
      const html = trimCustomHtml(
        isMarkdown
          ? await renderComposerHtml(editor.children)
          : toMatrixCustomHTML(editor.children, { allowTextFormatting: true })
      );
      const htmlDiffers = !customHtmlEqualsPlainText(html, plainText);
      if (isNativeEditUnchanged({ body, formattedBody }, { body: plainText, html, htmlDiffers })) {
        close();
        return;
      }
      const mentions = getMentions(roomId, editor);
      const owner = await editMessageWithNativeDesktopOwner({
        roomId,
        eventId,
        body: plainText,
        msgType: nativeEditMsgType(messageType),
        formattedBody: htmlDiffers ? html : undefined,
        mentionUserIds: Array.from(mentions.users),
        mentionRoom: mentions.room,
      });
      if (owner !== 'native') {
        throw new Error('Editing needs a signed-in Matrix session. Sign in again, then retry.');
      }
      close();
    } catch (saveError) {
      setError(nativeTimelineActionErrorMessage(saveError, TIMELINE_ACTION_FAILED.edit));
    } finally {
      savingRef.current = false;
      if (!closedRef.current) setSaving(false);
    }
  }, [body, close, editor, eventId, formattedBody, isMarkdown, messageType, roomId]);

  const handleKeyDown: KeyboardEventHandler = useCallback(
    (evt) => {
      if (isKeyHotkey('mod+enter', evt) || (!enterForNewline && isKeyHotkey('enter', evt))) {
        evt.preventDefault();
        const pickSuggestion =
          autocompleteQuery &&
          !isKeyHotkey('mod+enter', evt) &&
          !(
            autocompleteQuery.prefix === AutocompletePrefix.Emoticon &&
            autocompleteQuery.text.length < 2
          );
        if (pickSuggestion && autocompleteSelectFirst.current?.()) return;
        void save();
        return;
      }
      if (isKeyHotkey('escape', evt)) {
        evt.preventDefault();
        evt.stopPropagation();
        if (autocompleteQuery) {
          setAutocompleteQuery(undefined);
          return;
        }
        close();
      }
    },
    [autocompleteQuery, close, enterForNewline, save]
  );

  const handleKeyUp: KeyboardEventHandler = useCallback(
    (evt) => {
      if (isKeyHotkey('escape', evt)) {
        evt.preventDefault();
        return;
      }
      const prevWordRange = getPrevWorldRange(editor);
      setAutocompleteQuery(
        prevWordRange
          ? getAutocompleteQuery<AutocompletePrefix>(editor, prevWordRange, AUTOCOMPLETE_PREFIXES)
          : undefined
      );
    },
    [editor]
  );

  const handlePaste: ClipboardEventHandler = useCallback(
    (evt) => {
      const insertion = insertClipboardData(editor, evt.clipboardData);
      if (shouldPreventDefaultForClipboardInsert(insertion)) evt.preventDefault();
    },
    [editor]
  );

  const handleCloseAutocomplete = useCallback(() => {
    ReactEditor.focus(editor);
    setAutocompleteQuery(undefined);
  }, [editor]);

  return (
    <Box ref={rootRef} direction="Column" gap="100" data-native-timeline-editor="true">
      {autocompleteQuery?.prefix === AutocompletePrefix.RoomMention && (
        <RoomMentionAutocomplete
          roomId={roomId}
          editor={editor}
          query={autocompleteQuery}
          requestClose={handleCloseAutocomplete}
          selectFirstRef={autocompleteSelectFirst}
        />
      )}
      {room && autocompleteQuery?.prefix === AutocompletePrefix.UserMention && (
        <UserMentionAutocomplete
          room={room}
          editor={editor}
          query={autocompleteQuery}
          requestClose={handleCloseAutocomplete}
          selectFirstRef={autocompleteSelectFirst}
        />
      )}
      {autocompleteQuery?.prefix === AutocompletePrefix.Emoticon && (
        <EmoticonAutocomplete
          imagePackRooms={imagePackRooms}
          editor={editor}
          query={autocompleteQuery}
          requestClose={handleCloseAutocomplete}
          selectFirstRef={autocompleteSelectFirst}
        />
      )}
      <CustomEditor
        editor={editor}
        editableName="NativeTimelineMessageEditor"
        ariaLabel="Edit message"
        placeholder="Edit message…"
        onKeyDown={handleKeyDown}
        onKeyUp={handleKeyUp}
        onPaste={handlePaste}
        bottom={
          <>
            <Box
              style={{ padding: config.space.S200, paddingTop: 0 }}
              alignItems="End"
              justifyContent="SpaceBetween"
              gap="100"
            >
              <Box gap="Inherit">
                <Chip
                  onClick={() => void save()}
                  variant="Primary"
                  radii="Pill"
                  disabled={saving}
                  outlined
                  before={saving ? <Spinner variant="Primary" fill="Soft" size="100" /> : undefined}
                >
                  <Text size="B300">Save</Text>
                </Chip>
                <Chip onClick={close} variant="SurfaceVariant" radii="Pill" disabled={saving}>
                  <Text size="B300">Cancel</Text>
                </Chip>
              </Box>
              <IconButton
                className={depthCss.quietActionButton}
                variant="Surface"
                fill="None"
                size="300"
                radii="300"
                onClick={() => setToolbar(!toolbar)}
                aria-expanded={toolbar}
                aria-label={toolbar ? 'Hide formatting toolbar' : 'Show formatting toolbar'}
              >
                <Icon size="400" src={toolbar ? Icons.AlphabetUnderline : Icons.Alphabet} />
              </IconButton>
            </Box>
            {toolbar && (
              <div>
                <Line variant="SurfaceVariant" size="300" />
                <Toolbar />
              </div>
            )}
          </>
        }
      />
      {error ? (
        <Box className={css.ComposerError} gap="200" alignItems="Center" role="alert">
          <Icon src={Icons.Warning} size="100" />
          <Box grow="Yes">
            <Text size="T200">{error}</Text>
          </Box>
        </Box>
      ) : null}
      <Text size="T200" style={{ color: 'var(--synara-content-secondary)' }}>
        {enterForNewline ? 'Ctrl/Cmd+Enter to save' : 'Enter to save'} · Escape to cancel
      </Text>
    </Box>
  );
}
