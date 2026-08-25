import React, {
  ClipboardEventHandler,
  ChangeEventHandler,
  KeyboardEventHandler,
  forwardRef,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { useAtom, useAtomValue } from 'jotai';
import { isKeyHotkey } from 'is-hotkey';
import type { EventedRoomReading } from '../../utils/roomEvents';

type RelationTypeRelatesTo = {
  'm.in_reply_to'?: { event_id: string };
  event_id?: string;
  rel_type?: string;
  is_falling_back?: boolean;
  [key: string]: unknown;
};
import { IContent, MsgType, RelationType } from '../../utils/messageContent';
import { useTranslation } from 'react-i18next';
import { ReactEditor } from 'slate-react';
import { Transforms, Editor } from 'slate';
import {
  Box,
  Button,
  Dialog,
  Icon,
  IconButton,
  Icons,
  Input,
  Menu,
  MenuItem,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  PopOut,
  RectCords,
  Scroll,
  Text,
  config,
  toRem,
} from 'folds';

import { useMatrixClient } from '../../hooks/useMatrixClient';
import {
  EditorChangeHandler,
  Toolbar,
  toMatrixCustomHTML,
  toPlainText,
  AUTOCOMPLETE_PREFIXES,
  AutocompletePrefix,
  AutocompleteQuery,
  getAutocompleteQuery,
  getPrevWorldRange,
  resetEditor,
  RoomMentionAutocomplete,
  UserMentionAutocomplete,
  EmoticonAutocomplete,
  createEmoticonElement,
  moveCursor,
  resetEditorHistory,
  customHtmlEqualsPlainText,
  trimCustomHtml,
  isEmptyEditor,
  getBeginCommand,
  trimCommand,
  getMentions,
  insertClipboardData,
} from '../../components/editor';
import { EmojiBoard, EmojiBoardTab } from '../../components/emoji-board';
import { TUploadContent, getImageInfo, getMxIdLocalPart } from '../../utils/matrix';
import { useTypingStatusUpdater } from '../../hooks/useTypingStatusUpdater';
import { useFilePicker } from '../../hooks/useFilePicker';
import { useFileDropZone } from '../../hooks/useFileDrop';
import {
  TUploadItem,
  TUploadMetadata,
  roomIdToMsgDraftAtomFamily,
  roomIdToReplyDraftAtomFamily,
  roomIdToUploadItemsAtomFamily,
  roomUploadAtomFamily,
} from '../../state/room/roomInputDrafts';
import { UploadCardRenderer } from '../../components/upload-card';
import {
  UploadBoard,
  UploadBoardContent,
  UploadBoardHeader,
  UploadBoardImperativeHandlers,
} from '../../components/upload-board';
import {
  Upload,
  UploadStatus,
  UploadSuccess,
  createUploadFamilyObserverAtom,
} from '../../state/upload';
import {
  editableActiveElement,
  getDataTransferFiles,
  getImageUrlBlob,
  shouldProbeNativeClipboardImage,
  loadImageElement,
} from '../../utils/dom';
import { safeFile } from '../../utils/mimeTypes';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { getMemberDisplayName, getMentionContent, trimReplyFromBody } from '../../utils/room';
import { CommandAutocomplete } from './CommandAutocomplete';
import { Command, SHRUG, TABLEFLIP, UNFLIP, useCommands } from '../../hooks/useCommands';
import { mobileOrTablet } from '../../utils/user-agent';
import { ReplyLayout, ThreadIndicator } from '../../components/message';
import { roomToParentsAtom } from '../../state/room/roomToParents';
import { useMediaAuthentication } from '../../hooks/useMediaAuthentication';
import { useImagePackRooms } from '../../hooks/useImagePackRooms';
import { usePowerLevelsContext } from '../../hooks/usePowerLevels';
import colorMXID from '../../../util/colorMXID';
import { useIsDirectRoom } from '../../hooks/useRoom';
import { useAccessiblePowerTagColors, useGetMemberPowerTag } from '../../hooks/useMemberPowerTag';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { useTheme } from '../../hooks/useTheme';
import { useRoomCreatorsTag } from '../../hooks/useRoomCreatorsTag';
import { usePowerLevelTags } from '../../hooks/usePowerLevelTags';
import { resolveOptionalMatrixMediaUrl } from '../../matrix/media';
import { useComposingCheck } from '../../hooks/useComposingCheck';
import { useClientConfig } from '../../hooks/useClientConfig';
import {
  isDesktopPlatform,
  readPlatformClipboardImage,
  readPlatformClipboardText,
} from '../../platform';
import { gifPickerEnabled, gifSearchAvailable } from '../../utils/gifProvider';
import type { GifResult } from '../../utils/gifProvider';
import { GifPicker } from './gif/GifPicker';
import { clearRoomDraft, loadRoomDraft, saveRoomDraft } from '../../utils/drafts';
import {
  DEFAULT_POLL_SELECTIONS,
  MAX_POLL_SELECTIONS,
  normalizePollParts,
} from '../../utils/polls';
import { RoomComposer } from './RoomComposer';
import * as css from './RoomComposer.css';
import {
  fileToNativeAttachmentBytes,
  nativeComposerAttachmentReady,
  sendComposerAttachmentsWithNativeOwner,
} from './nativeSendAttachment';
import { sendComposerGifWithNativeOwner } from './nativeSendGif';
import { sendComposerStickerWithNativeOwner } from './nativeSendSticker';
import { sendPollWithNativeDesktopOwner } from './nativePoll';
import { sendPlainTextWithNativeOwner } from './nativeSendText';
import { clearNativeComposerReplyDraft } from './nativeComposerDraft';

const NATIVE_PASTE_EVENT = 'synara://native-paste';

interface RoomInputProps {
  editor: Editor;
  roomId: string;
  room: EventedRoomReading;
}
export const RoomInput = forwardRef<HTMLDivElement, RoomInputProps>(
  ({ editor, roomId, room }, ref) => {
    const mx = useMatrixClient();
    const clientConfig = useClientConfig();
    const useAuthentication = useMediaAuthentication();
    const [enterForNewline] = useSetting(settingsAtom, 'enterForNewline');
    const [isMarkdown] = useSetting(settingsAtom, 'isMarkdown');
    const [hideActivity] = useSetting(settingsAtom, 'hideActivity');
    const [legacyUsernameColor] = useSetting(settingsAtom, 'legacyUsernameColor');
    const [gifSearchEnabled, setGifSearchEnabled] = useSetting(settingsAtom, 'gifSearchEnabled');
    const [gifOnboardingDismissed, setGifOnboardingDismissed] = useSetting(
      settingsAtom,
      'gifOnboardingDismissed'
    );
    const { t } = useTranslation();
    const direct = useIsDirectRoom();
    const commands = useCommands(mx, room as unknown as Parameters<typeof useCommands>[1]);
    const emojiBtnRef = useRef<HTMLButtonElement>(null);
    const composerToolsBtnRef = useRef<HTMLButtonElement>(null);
    const roomToParents = useAtomValue(roomToParentsAtom);
    const powerLevels = usePowerLevelsContext();
    const creators = useRoomCreators(room);

    const [msgDraft, setMsgDraft] = useAtom(roomIdToMsgDraftAtomFamily(roomId));
    const [replyDraft, setReplyDraft] = useAtom(roomIdToReplyDraftAtomFamily(roomId));
    const clearReplyDraft = useCallback(() => {
      setReplyDraft(undefined);
      void clearNativeComposerReplyDraft({ roomId });
    }, [roomId, setReplyDraft]);
    const replyUserID = replyDraft?.userId;

    const powerLevelTags = usePowerLevelTags(room, powerLevels);
    const creatorsTag = useRoomCreatorsTag();
    const getMemberPowerTag = useGetMemberPowerTag(room, creators, powerLevels);
    const theme = useTheme();
    const accessibleTagColors = useAccessiblePowerTagColors(
      theme.kind,
      creatorsTag,
      powerLevelTags
    );

    const replyPowerTag = replyUserID ? getMemberPowerTag(replyUserID) : undefined;
    const replyPowerColor = replyPowerTag?.color
      ? accessibleTagColors.get(replyPowerTag.color)
      : undefined;
    const replyUsernameColor =
      legacyUsernameColor || direct ? colorMXID(replyUserID ?? '') : replyPowerColor;

    const [uploadBoard, setUploadBoard] = useState(true);
    const [selectedFiles, setSelectedFiles] = useAtom(roomIdToUploadItemsAtomFamily(roomId));
    const [nativeComposerSend, setNativeComposerSend] = useState(false);
    const uploadFamilyObserverAtom = createUploadFamilyObserverAtom(
      roomUploadAtomFamily,
      selectedFiles.map((f) => f.file)
    );
    const uploadBoardHandlers = useRef<UploadBoardImperativeHandlers | undefined>(undefined);

    useEffect(() => {
      let cancelled = false;
      void nativeComposerAttachmentReady().then((ready) => {
        if (!cancelled) setNativeComposerSend(ready);
      });
      return () => {
        cancelled = true;
      };
    }, [roomId]);

    const imagePackRooms: string[] = useImagePackRooms(roomId, roomToParents);

    const [toolbar, setToolbar] = useSetting(settingsAtom, 'editorToolbar');
    const [composerToolsAnchor, setComposerToolsAnchor] = useState<RectCords>();
    const [emojiBoardTab, setEmojiBoardTab] = useState<EmojiBoardTab>();
    const [autocompleteQuery, setAutocompleteQuery] =
      useState<AutocompleteQuery<AutocompletePrefix>>();
    const [gifPickerAnchor, setGifPickerAnchor] = useState<RectCords>();
    const [gifSending, setGifSending] = useState(false);
    const [gifSendError, setGifSendError] = useState<string>();
    const [sendingMessage, setSendingMessage] = useState(false);
    const [sendError, setSendError] = useState<string>();
    const [pollAnchor, setPollAnchor] = useState<RectCords>();
    const [pollQuestion, setPollQuestion] = useState('');
    const [pollAnswers, setPollAnswers] = useState(['', '']);
    const [pollMaxSelections, setPollMaxSelections] = useState(DEFAULT_POLL_SELECTIONS);
    const [pollError, setPollError] = useState<string>();
    const gifProviderAvailable = gifPickerEnabled(clientConfig.gifPicker);
    const gifPickerAvailable = gifSearchAvailable(clientConfig.gifPicker, gifSearchEnabled);
    const gifOnboardingVisible =
      gifProviderAvailable && !gifSearchEnabled && !gifOnboardingDismissed;

    const sendTypingStatus = useTypingStatusUpdater(roomId);

    const handleFiles = useCallback(
      async (files: File[]) => {
        setUploadBoard(true);
        const safeFiles = files.map(safeFile);
        const fileItems: TUploadItem[] = [];
        // Native Rust encrypts in e2e rooms. JS encrypt is retired.
        const nativeReady = await nativeComposerAttachmentReady();
        setNativeComposerSend(nativeReady);
        if (!nativeReady) {
          throw new Error('Native Matrix attachment send is unavailable.');
        }
        safeFiles.forEach((f) =>
          fileItems.push({
            file: f,
            originalFile: f,
            encInfo: undefined,
            metadata: {
              markedAsSpoiler: false,
            },
          })
        );
        setSelectedFiles({
          type: 'PUT',
          item: fileItems,
        });
      },
      [setSelectedFiles]
    );
    const pickFile = useFilePicker(handleFiles, true);
    const handleNativeClipboardImage = useCallback(async () => {
      const file = await readPlatformClipboardImage();
      if (file) {
        handleFiles([file]);
        return true;
      }
      return false;
    }, [handleFiles]);
    const roomInputHasFocus = useCallback((): boolean => {
      const activeElement = document.activeElement;
      if (
        activeElement instanceof HTMLElement &&
        activeElement.getAttribute('data-editable-name') === 'RoomInput'
      ) {
        return true;
      }

      try {
        return ReactEditor.isFocused(editor);
      } catch {
        return false;
      }
    }, [editor]);
    const roomInputPasteAvailable = useCallback((): boolean => {
      const portalContainer = document.getElementById('portalContainer');
      if (portalContainer && portalContainer.children.length > 0) return false;
      if (roomInputHasFocus()) return true;
      return !editableActiveElement();
    }, [roomInputHasFocus]);
    const handleNativeClipboardPaste = useCallback(async () => {
      if (!roomInputPasteAvailable()) return false;

      const imageHandled = await handleNativeClipboardImage();
      if (imageHandled) return true;

      if (!roomInputHasFocus()) return false;

      const text = await readPlatformClipboardText();
      if (!text) return false;

      return insertClipboardData(
        editor,
        {
          getData: (format) => (format === 'text/plain' ? text : ''),
        },
        isMarkdown
      );
    }, [
      editor,
      handleNativeClipboardImage,
      isMarkdown,
      roomInputHasFocus,
      roomInputPasteAvailable,
    ]);
    const handlePaste: ClipboardEventHandler = useCallback(
      (evt) => {
        const files = getDataTransferFiles(evt.clipboardData);
        if (files) {
          evt.preventDefault();
          handleFiles(files);
          return;
        }

        if (isDesktopPlatform() && shouldProbeNativeClipboardImage(evt.clipboardData)) {
          evt.preventDefault();
          const { clipboardData } = evt;
          void handleNativeClipboardImage().then((handled) => {
            if (!handled) {
              insertClipboardData(editor, clipboardData, isMarkdown);
            }
          });
          return;
        }

        if (insertClipboardData(editor, evt.clipboardData, isMarkdown)) {
          evt.preventDefault();
        }
      },
      [editor, handleFiles, handleNativeClipboardImage, isMarkdown]
    );
    useEffect(() => {
      const handleWindowPaste = (evt: ClipboardEvent) => {
        if (editableActiveElement()) return;
        const portalContainer = document.getElementById('portalContainer');
        if (portalContainer && portalContainer.children.length > 0) return;
        if (!evt.clipboardData) return;

        const files = getDataTransferFiles(evt.clipboardData);
        if (!files) {
          if (!isDesktopPlatform() || !shouldProbeNativeClipboardImage(evt.clipboardData)) return;
          evt.preventDefault();
          void handleNativeClipboardImage();
          return;
        }

        evt.preventDefault();
        handleFiles(files);
      };

      window.addEventListener('paste', handleWindowPaste);
      return () => {
        window.removeEventListener('paste', handleWindowPaste);
      };
    }, [handleFiles, handleNativeClipboardImage]);
    useEffect(() => {
      if (!isDesktopPlatform()) return undefined;

      const runNativePaste = (evt?: Event) => {
        if (!roomInputPasteAvailable()) return;
        evt?.preventDefault();
        void handleNativeClipboardPaste();
      };
      const handleNativePasteEvent = (evt: Event) => {
        runNativePaste(evt);
      };
      const handleNativePasteKey = (evt: KeyboardEvent) => {
        if (!isKeyHotkey('mod+v', evt)) return;
        runNativePaste(evt);
      };

      window.addEventListener(NATIVE_PASTE_EVENT, handleNativePasteEvent);
      window.addEventListener('keydown', handleNativePasteKey, true);
      return () => {
        window.removeEventListener(NATIVE_PASTE_EVENT, handleNativePasteEvent);
        window.removeEventListener('keydown', handleNativePasteKey, true);
      };
    }, [handleNativeClipboardPaste, roomInputPasteAvailable]);
    const dropZoneVisible = useFileDropZone(handleFiles);

    const isComposing = useComposingCheck();

    const getReplyRelation = useCallback(() => {
      if (!replyDraft) return undefined;

      const relation: RelationTypeRelatesTo = {
        'm.in_reply_to': {
          event_id: replyDraft.eventId,
        },
      };
      if (replyDraft.relation?.rel_type === RelationType.Thread) {
        relation.event_id = replyDraft.relation.event_id;
        relation.rel_type = RelationType.Thread;
        relation.is_falling_back = false;
      }

      return relation;
    }, [replyDraft]);

    const addReplyRelationToContent = useCallback(
      (content: IContent): IContent => {
        const relation = getReplyRelation();
        if (!replyDraft || !relation) return content;

        const relatedContent: IContent = {
          ...content,
          'm.relates_to': relation,
        };

        if (replyDraft.userId !== mx.getUserId()) {
          relatedContent['m.mentions'] = getMentionContent([replyDraft.userId], false);
        }

        return relatedContent;
      },
      [mx, replyDraft, getReplyRelation]
    );

    useEffect(() => {
      const storedDraft = loadRoomDraft(window.localStorage, mx.getSafeUserId(), roomId);
      const draft = msgDraft.length > 0 ? msgDraft : storedDraft;
      if (draft && draft.length > 0) {
        Transforms.insertFragment(editor, draft);
      }
    }, [mx, roomId, editor, msgDraft]);

    useEffect(
      () => () => {
        if (!isEmptyEditor(editor)) {
          const parsedDraft = JSON.parse(JSON.stringify(editor.children));
          setMsgDraft(parsedDraft);
          saveRoomDraft(window.localStorage, mx.getSafeUserId(), roomId, parsedDraft);
        } else {
          setMsgDraft([]);
          clearRoomDraft(window.localStorage, mx.getSafeUserId(), roomId);
        }
        resetEditor(editor);
        resetEditorHistory(editor);
      },
      [mx, roomId, editor, setMsgDraft]
    );

    const handleEditorChange = useCallback(
      (value: Parameters<EditorChangeHandler>[0]) => {
        if (isEmptyEditor(editor)) {
          clearRoomDraft(window.localStorage, mx.getSafeUserId(), roomId);
          return;
        }
        saveRoomDraft(window.localStorage, mx.getSafeUserId(), roomId, value);
      },
      [mx, roomId, editor]
    );

    const handleFileMetadata = useCallback(
      (fileItem: TUploadItem, metadata: TUploadMetadata) => {
        setSelectedFiles({
          type: 'REPLACE',
          item: fileItem,
          replacement: { ...fileItem, metadata },
        });
      },
      [setSelectedFiles]
    );

    const handleRemoveUpload = useCallback(
      (upload: TUploadContent | TUploadContent[]) => {
        const uploads = Array.isArray(upload) ? upload : [upload];
        setSelectedFiles({
          type: 'DELETE',
          item: selectedFiles.filter((f) => uploads.find((u) => u === f.file)),
        });
        uploads.forEach((u) => roomUploadAtomFamily.remove(u));
      },
      [setSelectedFiles, selectedFiles]
    );

    const handleCancelUpload = (uploads: Upload[]) => {
      uploads.forEach((upload) => {
        if (upload.status === UploadStatus.Loading) {
          mx.cancelUpload(upload.promise);
        }
      });
      handleRemoveUpload(uploads.map((upload) => upload.file));
    };

    const handleSendUpload = async (uploads: UploadSuccess[]) => {
      const replyTo = typeof replyDraft?.eventId === 'string' ? replyDraft.eventId : undefined;
      const threadRoot =
        replyDraft?.relation?.rel_type === RelationType.Thread &&
        typeof replyDraft.relation.event_id === 'string'
          ? replyDraft.relation.event_id
          : undefined;
      const nativeFiles = await Promise.all(
        uploads.map(async (upload) => {
          const fileItem = selectedFiles.find((f) => f.file === upload.file);
          if (!fileItem) throw new Error('Broken upload');
          const source = fileItem.originalFile;
          return {
            filename: source.name || 'attachment',
            mimeType: source.type || 'application/octet-stream',
            bytes: await fileToNativeAttachmentBytes(source),
          };
        })
      );
      const owner = await sendComposerAttachmentsWithNativeOwner(
        roomId,
        nativeFiles,
        replyTo,
        threadRoot
      );
      if (owner !== 'native') {
        throw new Error('Native Matrix attachment send is unavailable.');
      }
      handleCancelUpload(uploads);
      if (nativeFiles.length > 0) {
        setReplyDraft(undefined);
      }
    };

    const submit = useCallback(async () => {
      if (sendingMessage) return;
      uploadBoardHandlers.current?.handleSend();

      const commandName = getBeginCommand(editor);
      let plainText = toPlainText(editor.children, isMarkdown).trim();
      let customHtml = trimCustomHtml(
        toMatrixCustomHTML(editor.children, {
          allowTextFormatting: true,
          allowBlockMarkdown: isMarkdown,
          allowInlineMarkdown: isMarkdown,
        })
      );
      let msgType: 'm.text' | 'm.emote' | 'm.notice' = MsgType.Text;

      if (commandName) {
        plainText = trimCommand(commandName, plainText);
        customHtml = trimCommand(commandName, customHtml);
      }
      if (commandName === Command.Me) {
        msgType = MsgType.Emote;
      } else if (commandName === Command.Notice) {
        msgType = MsgType.Notice;
      } else if (commandName === Command.Shrug) {
        plainText = `${SHRUG} ${plainText}`;
        customHtml = `${SHRUG} ${customHtml}`;
      } else if (commandName === Command.TableFlip) {
        plainText = `${TABLEFLIP} ${plainText}`;
        customHtml = `${TABLEFLIP} ${customHtml}`;
      } else if (commandName === Command.UnFlip) {
        plainText = `${UNFLIP} ${plainText}`;
        customHtml = `${UNFLIP} ${customHtml}`;
      } else if (commandName) {
        const commandContent = commands[commandName as Command];
        if (commandContent) {
          commandContent.exe(plainText);
        }
        resetEditor(editor);
        resetEditorHistory(editor);
        sendTypingStatus(false);
        return;
      }

      if (plainText === '') return;

      const body = plainText;
      const formattedBody = customHtml;
      const mentionData = getMentions(mx, roomId, editor);

      const content: IContent = {
        msgtype: msgType,
        body,
      };

      if (replyDraft && replyDraft.userId !== mx.getUserId()) {
        mentionData.users.add(replyDraft.userId);
      }

      const mMentions = getMentionContent(Array.from(mentionData.users), mentionData.room);
      content['m.mentions'] = mMentions;

      if (replyDraft || !customHtmlEqualsPlainText(formattedBody, body)) {
        content.format = 'org.matrix.custom.html';
        content.formatted_body = formattedBody;
      }
      // Thread/reply relations for the native owner travel as IPC fields, not
      // via a JS-built m.relates_to blob. Legacy web keeps getReplyRelation.
      const relation = getReplyRelation();
      if (relation) {
        content['m.relates_to'] = relation;
      }
      const threadRoot =
        replyDraft?.relation?.rel_type === RelationType.Thread &&
        typeof replyDraft.relation.event_id === 'string'
          ? replyDraft.relation.event_id
          : undefined;
      try {
        setSendingMessage(true);
        setSendError(undefined);
        const nativeOwner = await sendPlainTextWithNativeOwner({
          roomId,
          body,
          msgType,
          formattedBody: content.formatted_body,
          mentionUserIds: Array.from(mentionData.users),
          mentionRoom: mentionData.room,
          replyTo: replyDraft?.eventId as string | undefined,
          threadRoot,
        });
        if (nativeOwner === 'legacy') {
          await mx.sendMessage(roomId, content as any);
        }
        resetEditor(editor);
        resetEditorHistory(editor);
        clearRoomDraft(window.localStorage, mx.getSafeUserId(), roomId);
        clearReplyDraft();
        sendTypingStatus(false);
      } catch (err) {
        const reason =
          err instanceof Error && err.message
            ? err.message
            : t('composer.send_failed', 'Could not send message.');
        setSendError(
          t('composer.send_failed_with_reason', {
            reason,
            defaultValue: 'Could not send message: {{reason}}',
          })
        );
      } finally {
        setSendingMessage(false);
      }
    }, [
      mx,
      roomId,
      editor,
      replyDraft,
      sendTypingStatus,
      clearReplyDraft,
      isMarkdown,
      commands,
      getReplyRelation,
      sendingMessage,
      t,
    ]);

    const handlePollAnswerChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
      const index = Number(evt.currentTarget.getAttribute('data-answer-index'));
      const { value } = evt.currentTarget;
      setPollAnswers((current) =>
        current.map((answer, answerIndex) => (answerIndex === index ? value : answer))
      );
    };

    const handlePollMaxSelectionsChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
      const parsed = Number.parseInt(evt.currentTarget.value, 10);
      const safeValue = Number.isFinite(parsed) ? parsed : DEFAULT_POLL_SELECTIONS;
      const maxSelectable = Math.max(
        1,
        Math.min(MAX_POLL_SELECTIONS, pollAnswers.length, safeValue)
      );
      setPollMaxSelections(maxSelectable);
    };

    useEffect(() => {
      setPollMaxSelections((current) =>
        Math.max(1, Math.min(current, pollAnswers.length, MAX_POLL_SELECTIONS))
      );
    }, [pollAnswers]);

    const handleSendPoll = async () => {
      setPollError(undefined);
      const poll = normalizePollParts(pollQuestion, pollAnswers, pollMaxSelections);
      if (!poll) {
        setPollError(
          t('modernization.poll.invalid', 'Add a question and at least two answer options.')
        );
        return;
      }
      try {
        const threadRoot =
          replyDraft?.relation?.rel_type === RelationType.Thread &&
          typeof replyDraft.relation.event_id === 'string'
            ? replyDraft.relation.event_id
            : undefined;
        const owner = await sendPollWithNativeDesktopOwner({
          roomId,
          question: poll.question,
          answers: poll.answers.map((answer) => answer.text),
          maxSelections: poll.maxSelections,
          threadRoot,
          replyTo: replyDraft?.eventId as string | undefined,
        });
        if (owner === 'legacy') {
          setPollError(
            t(
              'modernization.poll.native_required',
              'Native Matrix session is required to send polls on desktop.'
            )
          );
          return;
        }
        setPollQuestion('');
        setPollAnswers(['', '']);
        setPollMaxSelections(DEFAULT_POLL_SELECTIONS);
        setPollAnchor(undefined);
      } catch {
        setPollError(t('modernization.poll.send_failed', 'Could not send poll.'));
      }
    };

    const handleKeyDown: KeyboardEventHandler = useCallback(
      (evt) => {
        if (
          (isKeyHotkey('mod+enter', evt) || (!enterForNewline && isKeyHotkey('enter', evt))) &&
          !isComposing(evt)
        ) {
          evt.preventDefault();
          submit();
        }
        if (isKeyHotkey('escape', evt)) {
          evt.preventDefault();
          if (autocompleteQuery) {
            setAutocompleteQuery(undefined);
            return;
          }
          clearReplyDraft();
        }
      },
      [submit, clearReplyDraft, enterForNewline, autocompleteQuery, isComposing]
    );

    const handleKeyUp: KeyboardEventHandler = useCallback(
      (evt) => {
        if (isKeyHotkey('escape', evt)) {
          evt.preventDefault();
          return;
        }

        if (!hideActivity) {
          sendTypingStatus(!isEmptyEditor(editor));
        }

        const prevWordRange = getPrevWorldRange(editor);
        const query = prevWordRange
          ? getAutocompleteQuery<AutocompletePrefix>(editor, prevWordRange, AUTOCOMPLETE_PREFIXES)
          : undefined;
        setAutocompleteQuery(query);
      },
      [editor, sendTypingStatus, hideActivity]
    );

    const handleCloseAutocomplete = useCallback(() => {
      setAutocompleteQuery(undefined);
      ReactEditor.focus(editor);
    }, [editor]);

    const handleEmoticonSelect = (key: string, shortcode: string) => {
      editor.insertNode(createEmoticonElement(key, shortcode));
      moveCursor(editor);
    };

    const handleStickerSelect = async (mxc: string, shortcode: string, label: string) => {
      const replyTo = typeof replyDraft?.eventId === 'string' ? replyDraft.eventId : undefined;
      const threadRoot =
        replyDraft?.relation?.rel_type === RelationType.Thread &&
        typeof replyDraft.relation.event_id === 'string'
          ? replyDraft.relation.event_id
          : undefined;

      // Prefer pack media info when resolvable; native send still works without it.
      let info: { w?: number; h?: number; mimetype?: string; size?: number } | undefined;
      const stickerUrl = resolveOptionalMatrixMediaUrl(mx, mxc, { useAuthentication });
      if (stickerUrl) {
        try {
          info = await getImageInfo(
            await loadImageElement(stickerUrl),
            await getImageUrlBlob(stickerUrl)
          );
        } catch {
          info = undefined;
        }
      }

      const nativeOwner = await sendComposerStickerWithNativeOwner({
        roomId,
        body: label || shortcode || 'sticker',
        mxc,
        info: info
          ? {
              width: info.w,
              height: info.h,
              mimetype: info.mimetype,
              size: info.size,
            }
          : undefined,
        replyTo,
        threadRoot,
      });
      if (nativeOwner === 'native') {
        setReplyDraft(undefined);
        return;
      }

      // Legacy web path — only when no native Matrix session is live.
      if (!stickerUrl || !info) return;
      mx.sendEvent(
        roomId,
        'm.sticker' as any,
        addReplyRelationToContent({
          body: label,
          url: mxc,
          info,
        }) as any
      );
      clearReplyDraft();
    };

    const handleGifSelect = async (gif: GifResult) => {
      setGifSending(true);
      setGifSendError(undefined);
      const replyTo = typeof replyDraft?.eventId === 'string' ? replyDraft.eventId : undefined;
      const threadRoot =
        replyDraft?.relation?.rel_type === RelationType.Thread &&
        typeof replyDraft.relation.event_id === 'string'
          ? replyDraft.relation.event_id
          : undefined;
      try {
        await sendComposerGifWithNativeOwner(roomId, gif, replyTo, threadRoot);
        setReplyDraft(undefined);
        setGifPickerAnchor(undefined);
      } catch (err) {
        setGifSendError(err instanceof Error ? err.message : 'Failed to send GIF.');
      } finally {
        setGifSending(false);
      }
    };

    return (
      <div ref={ref}>
        {selectedFiles.length > 0 && (
          <UploadBoard
            header={
              <UploadBoardHeader
                open={uploadBoard}
                onToggle={() => setUploadBoard(!uploadBoard)}
                uploadFamilyObserverAtom={uploadFamilyObserverAtom}
                onSend={handleSendUpload}
                imperativeHandlerRef={uploadBoardHandlers}
                onCancel={handleCancelUpload}
              />
            }
          >
            {uploadBoard && (
              <Scroll size="300" hideTrack visibility="Hover">
                <UploadBoardContent>
                  {Array.from(selectedFiles)
                    .reverse()
                    .map((fileItem, index) => (
                      <UploadCardRenderer
                        // eslint-disable-next-line react/no-array-index-key
                        key={index}
                        isEncrypted={!!fileItem.encInfo}
                        nativeComposerSend={nativeComposerSend}
                        fileItem={fileItem}
                        setMetadata={handleFileMetadata}
                        onRemove={handleRemoveUpload}
                      />
                    ))}
                </UploadBoardContent>
              </Scroll>
            )}
          </UploadBoard>
        )}
        <Overlay
          open={dropZoneVisible}
          backdrop={<OverlayBackdrop />}
          style={{ pointerEvents: 'none' }}
        >
          <OverlayCenter>
            <Dialog variant="Primary">
              <Box
                direction="Column"
                justifyContent="Center"
                alignItems="Center"
                gap="500"
                style={{ padding: toRem(60) }}
              >
                <Icon size="600" src={Icons.File} />
                <Text size="H4" align="Center">
                  {`Drop Files in "${room?.name || 'Room'}"`}
                </Text>
                <Text align="Center">Drag and drop files here or click for selection dialog</Text>
              </Box>
            </Dialog>
          </OverlayCenter>
        </Overlay>
        {sendError && (
          <Box className={css.ComposerError} gap="200" alignItems="Center" role="alert">
            <Icon src={Icons.Warning} size="100" />
            <Box grow="Yes">
              <Text size="T200">{sendError}</Text>
            </Box>
            <IconButton
              onClick={() => setSendError(undefined)}
              variant="Critical"
              fill="Soft"
              size="300"
              radii="300"
              aria-label="Dismiss send error"
            >
              <Icon src={Icons.Cross} size="50" />
            </IconButton>
          </Box>
        )}
        {autocompleteQuery?.prefix === AutocompletePrefix.RoomMention && (
          <RoomMentionAutocomplete
            roomId={roomId}
            editor={editor}
            query={autocompleteQuery}
            requestClose={handleCloseAutocomplete}
          />
        )}
        {autocompleteQuery?.prefix === AutocompletePrefix.UserMention && (
          <UserMentionAutocomplete
            room={room}
            editor={editor}
            query={autocompleteQuery}
            requestClose={handleCloseAutocomplete}
          />
        )}
        {autocompleteQuery?.prefix === AutocompletePrefix.Emoticon && (
          <EmoticonAutocomplete
            imagePackRooms={imagePackRooms}
            editor={editor}
            query={autocompleteQuery}
            requestClose={handleCloseAutocomplete}
          />
        )}
        {autocompleteQuery?.prefix === AutocompletePrefix.Command && (
          <CommandAutocomplete
            room={room}
            editor={editor}
            query={autocompleteQuery}
            requestClose={handleCloseAutocomplete}
          />
        )}
        <RoomComposer
          editableName="RoomInput"
          editor={editor}
          placeholder="Send a message..."
          onKeyDown={handleKeyDown}
          onKeyUp={handleKeyUp}
          onPaste={handlePaste}
          replyPreview={
            replyDraft && (
              <div>
                <Box
                  alignItems="Center"
                  gap="300"
                  style={{ padding: `${config.space.S200} ${config.space.S300} 0` }}
                >
                  <IconButton
                    onClick={() => clearReplyDraft()}
                    variant="SurfaceVariant"
                    size="300"
                    radii="300"
                    aria-label={t('composer.reply.cancel_aria_label', 'Cancel reply')}
                  >
                    <Icon src={Icons.Cross} size="50" />
                  </IconButton>
                  <Box direction="Row" gap="200" alignItems="Center">
                    {replyDraft.relation?.rel_type === RelationType.Thread && <ThreadIndicator />}
                    <ReplyLayout
                      userColor={replyUsernameColor}
                      username={
                        <Text size="T300" truncate>
                          <b>
                            {getMemberDisplayName(room, replyDraft.userId) ??
                              getMxIdLocalPart(replyDraft.userId) ??
                              replyDraft.userId}
                          </b>
                        </Text>
                      }
                    >
                      <Text size="T300" truncate>
                        {trimReplyFromBody(replyDraft.body)}
                      </Text>
                    </ReplyLayout>
                  </Box>
                </Box>
              </div>
            )
          }
          leadingAction={
            <PopOut
              anchor={composerToolsAnchor}
              position="Top"
              align="Start"
              offset={12}
              content={
                <Menu style={{ width: toRem(196) }}>
                  <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
                    <MenuItem
                      size="300"
                      radii="300"
                      after={<Icon src={Icons.File} size="100" />}
                      onClick={() => {
                        setComposerToolsAnchor(undefined);
                        pickFile('*');
                      }}
                    >
                      <Text size="T300">Attach file</Text>
                    </MenuItem>
                    <MenuItem
                      size="300"
                      radii="300"
                      after={<Icon src={Icons.Sticker} size="100" />}
                      onClick={() => {
                        setComposerToolsAnchor(undefined);
                        setEmojiBoardTab(EmojiBoardTab.Sticker);
                      }}
                    >
                      <Text size="T300">Sticker</Text>
                    </MenuItem>
                    {(gifPickerAvailable || gifOnboardingVisible) && (
                      <MenuItem
                        size="300"
                        radii="300"
                        after={<Icon src={Icons.Photo} size="100" />}
                        onClick={() => {
                          const anchor = composerToolsBtnRef.current?.getBoundingClientRect();
                          setComposerToolsAnchor(undefined);
                          setGifPickerAnchor(anchor);
                        }}
                      >
                        <Text size="T300">GIF</Text>
                      </MenuItem>
                    )}
                    <MenuItem
                      size="300"
                      radii="300"
                      after={<Icon src={Icons.Message} size="100" />}
                      onClick={() => {
                        const anchor = composerToolsBtnRef.current?.getBoundingClientRect();
                        setComposerToolsAnchor(undefined);
                        setPollAnchor(anchor);
                      }}
                    >
                      <Text size="T300">Poll</Text>
                    </MenuItem>
                  </Box>
                </Menu>
              }
            >
              <IconButton
                ref={composerToolsBtnRef}
                onClick={(event) =>
                  setComposerToolsAnchor(
                    composerToolsAnchor ? undefined : event.currentTarget.getBoundingClientRect()
                  )
                }
                variant="SurfaceVariant"
                size="300"
                radii="300"
                aria-label="More message actions"
                aria-expanded={!!composerToolsAnchor}
              >
                <Icon src={Icons.PlusCircle} />
              </IconButton>
            </PopOut>
          }
          floatingActions={
            <>
              <IconButton
                variant="SurfaceVariant"
                size="300"
                radii="300"
                onClick={() => setToolbar(!toolbar)}
                aria-label={
                  toolbar
                    ? t('composer.formatting.hide_aria_label', 'Hide formatting toolbar')
                    : t('composer.formatting.show_aria_label', 'Show formatting toolbar')
                }
                aria-pressed={toolbar}
                aria-expanded={toolbar}
              >
                <Icon src={toolbar ? Icons.AlphabetUnderline : Icons.Alphabet} />
              </IconButton>
              <PopOut
                offset={16}
                alignOffset={-44}
                position="Top"
                align="End"
                anchor={
                  emojiBoardTab === undefined
                    ? undefined
                    : emojiBtnRef.current?.getBoundingClientRect() ?? undefined
                }
                content={
                  <EmojiBoard
                    tab={emojiBoardTab}
                    onTabChange={setEmojiBoardTab}
                    imagePackRooms={imagePackRooms}
                    returnFocusOnDeactivate={false}
                    onEmojiSelect={handleEmoticonSelect}
                    onCustomEmojiSelect={handleEmoticonSelect}
                    onStickerSelect={handleStickerSelect}
                    requestClose={() => {
                      setEmojiBoardTab((tab) => {
                        if (tab && !mobileOrTablet()) ReactEditor.focus(editor);
                        return undefined;
                      });
                    }}
                  />
                }
              >
                <IconButton
                  ref={emojiBtnRef}
                  aria-pressed={!!emojiBoardTab}
                  aria-label={t('composer.emoji_picker_aria_label', 'Emoji picker')}
                  onClick={() => setEmojiBoardTab(EmojiBoardTab.Emoji)}
                  variant="SurfaceVariant"
                  size="300"
                  radii="300"
                >
                  <Icon src={Icons.Smile} filled={!!emojiBoardTab} />
                </IconButton>
              </PopOut>
              {(gifPickerAvailable || gifOnboardingVisible) && (
                <PopOut
                  offset={16}
                  alignOffset={-44}
                  position="Top"
                  align="End"
                  anchor={gifPickerAnchor}
                  content={
                    gifPickerAvailable ? (
                      <GifPicker
                        config={clientConfig.gifPicker}
                        disabled={gifSending}
                        error={gifSendError}
                        onSelect={handleGifSelect}
                      />
                    ) : (
                      <Box
                        direction="Column"
                        gap="300"
                        style={{ padding: config.space.S400, width: toRem(280) }}
                      >
                        <Text size="H5">
                          {t('modernization.gif.onboarding.title', 'Enable GIFs?')}
                        </Text>
                        <Text size="T300" priority="300">
                          {t(
                            'modernization.gif.onboarding.description',
                            'GIF search downloads your selected GIF privately, uploads it to Matrix media, then sends an mxc:// attachment.'
                          )}
                        </Text>
                        <Box justifyContent="End" gap="200">
                          <Button
                            size="300"
                            variant="Secondary"
                            fill="None"
                            onClick={() => {
                              setGifOnboardingDismissed(true);
                              setGifPickerAnchor(undefined);
                            }}
                          >
                            <Text size="B300">
                              {t('modernization.gif.onboarding.not_now', 'Not now')}
                            </Text>
                          </Button>
                          <Button
                            size="300"
                            variant="Primary"
                            onClick={() => {
                              setGifSearchEnabled(true);
                              setGifOnboardingDismissed(true);
                              setGifPickerAnchor(
                                composerToolsBtnRef.current?.getBoundingClientRect()
                              );
                            }}
                          >
                            <Text size="B300">
                              {t('modernization.gif.onboarding.enable', 'Enable GIFs')}
                            </Text>
                          </Button>
                        </Box>
                      </Box>
                    )
                  }
                >
                  {null}
                </PopOut>
              )}
              <PopOut
                offset={16}
                alignOffset={-44}
                position="Top"
                align="End"
                anchor={pollAnchor}
                content={
                  <Box
                    direction="Column"
                    gap="300"
                    style={{ padding: config.space.S400, width: toRem(320) }}
                    role="group"
                    aria-label={t('modernization.poll.create_aria_label', 'Create poll')}
                  >
                    <Text size="H5">{t('modernization.poll.create_title', 'Create Poll')}</Text>
                    <Input
                      size="300"
                      radii="300"
                      variant="Background"
                      value={pollQuestion}
                      onChange={(evt) => setPollQuestion(evt.currentTarget.value)}
                      placeholder={t('modernization.poll.question_placeholder', 'Question')}
                      aria-label={t('modernization.poll.question_aria_label', 'Poll question')}
                    />
                    <Input
                      size="300"
                      radii="300"
                      variant="Background"
                      type="number"
                      min={DEFAULT_POLL_SELECTIONS}
                      max={Math.min(MAX_POLL_SELECTIONS, pollAnswers.length)}
                      value={pollMaxSelections}
                      onChange={handlePollMaxSelectionsChange}
                      placeholder={t('modernization.poll.max_aria_label', 'Max selections')}
                      aria-label={t('modernization.poll.max_aria_label', 'Max selections')}
                    />
                    <Text size="T200" priority="300">
                      {t('modernization.poll.max_description', {
                        count: pollMaxSelections,
                        defaultValue: 'Participants can choose up to {{count}} option(s).',
                      })}
                    </Text>
                    {pollAnswers.map((answer, index) => (
                      <Input
                        // eslint-disable-next-line react/no-array-index-key
                        key={index}
                        size="300"
                        radii="300"
                        variant="Background"
                        data-answer-index={index}
                        value={answer}
                        onChange={handlePollAnswerChange}
                        placeholder={t('modernization.poll.answer_placeholder', {
                          count: index + 1,
                          defaultValue: 'Option {{count}}',
                        })}
                        aria-label={t('modernization.poll.answer_aria_label', {
                          count: index + 1,
                          defaultValue: 'Poll option {{count}}',
                        })}
                      />
                    ))}
                    <Box gap="200">
                      <Button
                        size="300"
                        variant="Secondary"
                        fill="None"
                        onClick={() => setPollAnswers((current) => [...current, ''])}
                      >
                        <Text size="B300">{t('modernization.poll.add_option', 'Add option')}</Text>
                      </Button>
                      {pollAnswers.length > 2 && (
                        <Button
                          size="300"
                          variant="Secondary"
                          fill="None"
                          onClick={() => setPollAnswers((current) => current.slice(0, -1))}
                        >
                          <Text size="B300">
                            {t('modernization.poll.remove_option', 'Remove option')}
                          </Text>
                        </Button>
                      )}
                    </Box>
                    {pollError && (
                      <Text size="T300" priority="300">
                        {pollError}
                      </Text>
                    )}
                    <Box justifyContent="End" gap="200">
                      <Button
                        size="300"
                        variant="Secondary"
                        fill="None"
                        onClick={() => setPollAnchor(undefined)}
                      >
                        <Text size="B300">{t('modernization.poll.cancel', 'Cancel')}</Text>
                      </Button>
                      <Button size="300" variant="Primary" onClick={handleSendPoll}>
                        <Text size="B300">{t('modernization.poll.send', 'Send poll')}</Text>
                      </Button>
                    </Box>
                  </Box>
                }
              >
                {null}
              </PopOut>
              <IconButton
                onClick={submit}
                disabled={sendingMessage}
                variant="Primary"
                size="300"
                radii="300"
                aria-label={t('composer.send_aria_label', 'Send message')}
              >
                <Icon src={Icons.Send} />
              </IconButton>
            </>
          }
          onChange={handleEditorChange}
          toolbarVisible={toolbar}
          toolbar={<Toolbar />}
        />
      </div>
    );
  }
);
