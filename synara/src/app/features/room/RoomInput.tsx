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
import * as depthCss from '../../styles/Depth.css';
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
  shouldPreventDefaultForClipboardInsert,
  insertClipboardData,
} from '../../components/editor';
import { EmojiBoard } from '../../components/emoji-board';
import { TUploadContent, getMxIdLocalPart } from '../../utils/matrix';
import { useTypingStatusUpdater } from '../../hooks/useTypingStatusUpdater';
import { useFilePicker } from '../../hooks/useFilePicker';
import { useFileDropZone } from '../../hooks/useFileDrop';
import {
  TUploadItem,
  TUploadMetadata,
  roomIdToMsgDraftAtomFamily,
  roomIdToUploadItemsAtomFamily,
  roomUploadAtomFamily,
} from '../../state/room/roomInputDrafts';
import { UploadCardRenderer } from '../../components/upload-card';
import {
  UploadBoard,
  UploadBoardContent,
  UploadBoardHeader,
  UploadBoardImperativeHandlers,
  UploadSendOptions,
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
  shouldProbeNativeClipboardImage,
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
import { useImagePackRooms } from '../../hooks/useImagePackRooms';
import { usePowerLevelsContext } from '../../hooks/usePowerLevels';
import colorMXID from '../../../util/colorMXID';
import { useIsDirectRoom } from '../../hooks/useRoom';
import { useAccessiblePowerTagColors, useGetMemberPowerTag } from '../../hooks/useMemberPowerTag';
import { useRoomCreators } from '../../hooks/useRoomCreators';
import { useTheme } from '../../hooks/useTheme';
import { useRoomCreatorsTag } from '../../hooks/useRoomCreatorsTag';
import { usePowerLevelTags } from '../../hooks/usePowerLevelTags';
import { useComposingCheck } from '../../hooks/useComposingCheck';
import { useClientConfig } from '../../hooks/useClientConfig';
import { isDesktopPlatform, readPlatformClipboardImage } from '../../platform';
import { gifPickerEnabled, gifSearchAvailable } from '../../utils/gifProvider';
import type { GifResult } from '../../utils/gifProvider';
import { GifPicker } from './gif/GifPicker';
import { clearRoomDraft, loadRoomDraft, saveRoomDraft } from '../../utils/drafts';
import {
  DEFAULT_POLL_SELECTIONS,
  MAX_POLL_SELECTIONS,
  normalizePollParts,
  type ParsedPoll,
} from '../../utils/polls';
import { RoomComposer } from './RoomComposer';
import * as css from './RoomComposer.css';
import {
  fileToNativeAttachmentBytes,
  nativeComposerAttachmentReady,
  sendComposerAttachmentPlanWithNativeOwner,
} from './nativeSendAttachment';
import { sendComposerGifWithNativeOwner } from './nativeSendGif';
import {
  sendPollCommandWithNativeDesktopOwner,
  sendPollWithNativeDesktopOwner,
} from './nativePoll';
import { sendPlainTextWithNativeOwner } from './nativeSendText';
import {
  clearNativeComposerReplyDraft,
  nativeComposerSendRelation,
  useNativeComposerReplyDraft,
} from './nativeComposerDraft';
import type { AttachmentSendPlan } from './attachmentSendPlan';
import {
  completeAttachmentSendStep,
  hasTrailingAttachmentText,
  makeOrReuseAttachmentSendPlan,
} from './attachmentSendPlan';

interface RoomInputProps {
  editor: Editor;
  roomId: string;
  room: EventedRoomReading;
}
export const RoomInput = forwardRef<HTMLDivElement, RoomInputProps>(
  ({ editor, roomId, room }, ref) => {
    const mx = useMatrixClient();
    const clientConfig = useClientConfig();
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
    const emojiBtnRef = useRef<HTMLButtonElement>(null);
    const composerToolsBtnRef = useRef<HTMLButtonElement>(null);
    const roomToParents = useAtomValue(roomToParentsAtom);
    const powerLevels = usePowerLevelsContext();
    const creators = useRoomCreators(room);

    const [msgDraft, setMsgDraft] = useAtom(roomIdToMsgDraftAtomFamily(roomId));
    const replyDraft = useNativeComposerReplyDraft(roomId);
    const clearReplyDraft = useCallback(
      async (expectedDraftRevision: number) => {
        const result = await clearNativeComposerReplyDraft({ roomId, expectedDraftRevision });
        if (result === 'unavailable') {
          throw new Error('Native reply draft clear is unavailable.');
        }
      },
      [roomId]
    );
    const clearReplyDraftAfterSend = useCallback(
      async (expectedDraftRevision: number | undefined, onFailure: () => void) => {
        if (expectedDraftRevision === undefined) return;
        try {
          await clearReplyDraft(expectedDraftRevision);
        } catch {
          // The send already completed. Preserve the visible Core-backed draft
          // and report cleanup separately so retry cannot duplicate the message.
          onFailure();
        }
      },
      [clearReplyDraft]
    );
    const replyUserID = replyDraft?.senderId;

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
    const attachmentSendPlan = useRef<AttachmentSendPlan | undefined>(undefined);

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
    const [emojiBoardOpen, setEmojiBoardOpen] = useState(false);
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

    const sendSlashPoll = useCallback(
      async (poll: ParsedPoll) => {
        // Snapshot both fields from the one visible Core draft before the
        // asynchronous send begins; no Jotai/local relation may diverge.
        const sendRelation = nativeComposerSendRelation(replyDraft);
        const owner = await sendPollCommandWithNativeDesktopOwner(
          {
            roomId,
            question: poll.question,
            answers: poll.answers.map((answer) => answer.text),
            maxSelections: poll.maxSelections,
            threadRoot: sendRelation.threadRoot,
            replyTo: sendRelation.replyTo,
          },
          () =>
            clearReplyDraftAfterSend(sendRelation.draftRevision, () => {
              setSendError(
                t(
                  'composer.reply_clear_after_send_failed',
                  'Poll sent, but the reply state could not be cleared.'
                )
              );
            })
        );
        if (owner === 'legacy') {
          throw new Error('Native Matrix session is required to send polls on desktop.');
        }
      },
      [clearReplyDraftAfterSend, replyDraft, roomId, t]
    );
    const commands = useCommands(
      mx,
      room as unknown as Parameters<typeof useCommands>[1],
      sendSlashPoll
    );

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
            transactionId: `synara-attachment-${crypto.randomUUID()}`,
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
              insertClipboardData(editor, clipboardData);
            }
          });
          return;
        }

        const insertion = insertClipboardData(editor, evt.clipboardData);
        if (shouldPreventDefaultForClipboardInsert(insertion)) {
          evt.preventDefault();
        }
      },
      [editor, handleFiles, handleNativeClipboardImage]
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
    const dropZoneVisible = useFileDropZone(handleFiles);

    const isComposing = useComposingCheck();

    const getReplyRelation = useCallback(() => {
      const { replyTo, threadRoot } = nativeComposerSendRelation(replyDraft);
      if (!replyTo) return undefined;

      const relation: RelationTypeRelatesTo = {
        'm.in_reply_to': {
          event_id: replyTo,
        },
      };
      if (threadRoot) {
        relation.event_id = threadRoot;
        relation.rel_type = RelationType.Thread;
        relation.is_falling_back = false;
      }

      return relation;
    }, [replyDraft]);

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

    const handleSendUpload = async (uploads: UploadSuccess[], options?: UploadSendOptions) => {
      const { draftRevision, replyTo, threadRoot } = nativeComposerSendRelation(replyDraft);
      const nativeInputs = await Promise.all(
        uploads.map(async (upload) => {
          const fileItem = selectedFiles.find((f) => f.file === upload.file);
          if (!fileItem) throw new Error('Broken upload');
          const source = fileItem.originalFile;
          return {
            roomId,
            transactionId: fileItem.transactionId,
            file: {
              filename: source.name || 'attachment',
              mimeType: source.type || 'application/octet-stream',
              bytes: await fileToNativeAttachmentBytes(source),
            },
            caption: uploads.length === 1 ? options?.caption : undefined,
            formattedCaption: uploads.length === 1 ? options?.formattedCaption : undefined,
            mentionUserIds: uploads.length === 1 ? options?.mentionUserIds : undefined,
            mentionRoom: uploads.length === 1 ? options?.mentionRoom : undefined,
            replyTo,
            threadRoot,
          };
        })
      );
      const owner = await sendComposerAttachmentPlanWithNativeOwner(nativeInputs, (index) => {
        const sentTransactionId = nativeInputs[index].transactionId;
        if (attachmentSendPlan.current) {
          attachmentSendPlan.current = completeAttachmentSendStep(
            attachmentSendPlan.current,
            sentTransactionId
          );
        }
        handleRemoveUpload(uploads[index].file);
      });
      if (owner !== 'native') {
        throw new Error('Native Matrix attachment send is unavailable.');
      }
      if (nativeInputs.length > 0) {
        await clearReplyDraftAfterSend(draftRevision, () => {
          setSendError(
            t(
              'composer.reply_clear_after_send_failed',
              'Message sent, but the reply state could not be cleared.'
            )
          );
        });
      }
    };

    const submit = useCallback(async () => {
      if (sendingMessage) return;

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
        if (commandName === Command.Poll && commandContent) {
          try {
            setSendingMessage(true);
            setSendError(undefined);
            await commandContent.exe(plainText);
            resetEditor(editor);
            resetEditorHistory(editor);
            sendTypingStatus(false);
          } catch (err) {
            const reason =
              err instanceof Error && err.message ? err.message : 'Could not send poll.';
            setSendError(
              t('composer.send_failed_with_reason', {
                reason,
                defaultValue: 'Could not send message: {{reason}}',
              })
            );
          } finally {
            setSendingMessage(false);
          }
          return;
        }
        if (commandContent) {
          commandContent.exe(plainText);
        }
        resetEditor(editor);
        resetEditorHistory(editor);
        sendTypingStatus(false);
        return;
      }

      const attachmentCount = selectedFiles.length;
      if (plainText === '' && attachmentCount === 0) return;

      const body = plainText;
      const formattedBody = customHtml;
      const mentionData = getMentions(mx, roomId, editor);

      const content: IContent = {
        msgtype: msgType,
        body,
      };

      if (replyDraft && replyDraft.senderId !== mx.getUserId()) {
        mentionData.users.add(replyDraft.senderId);
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
      const sendRelation = nativeComposerSendRelation(replyDraft);
      try {
        setSendingMessage(true);
        setSendError(undefined);
        if (attachmentCount > 0) {
          const transactionIds = selectedFiles.map((file) => file.transactionId);
          const plan = makeOrReuseAttachmentSendPlan(
            attachmentSendPlan.current,
            transactionIds,
            body
          );
          attachmentSendPlan.current = plan;
          const attachmentCaption = plan.textRole === 'caption' ? body : undefined;
          const formattedCaption =
            attachmentCaption && !customHtmlEqualsPlainText(formattedBody, body)
              ? formattedBody
              : undefined;
          const uploadSender = uploadBoardHandlers.current;
          if (!uploadSender) {
            throw new Error('Attachments are still preparing.');
          }
          await uploadSender.handleSend({
            caption: attachmentCaption,
            formattedCaption,
            mentionUserIds: attachmentCaption ? Array.from(mentionData.users) : undefined,
            mentionRoom: attachmentCaption ? mentionData.room : undefined,
          });
          if (!hasTrailingAttachmentText(plan)) {
            attachmentSendPlan.current = undefined;
            resetEditor(editor);
            resetEditorHistory(editor);
            clearRoomDraft(window.localStorage, mx.getSafeUserId(), roomId);
            await clearReplyDraftAfterSend(sendRelation.draftRevision, () => {
              setSendError(
                t(
                  'composer.reply_clear_after_send_failed',
                  'Message sent, but the reply state could not be cleared.'
                )
              );
            });
            sendTypingStatus(false);
            return;
          }
          attachmentSendPlan.current = undefined;
        }
        const nativeOwner = await sendPlainTextWithNativeOwner({
          roomId,
          body,
          msgType,
          formattedBody: content.formatted_body,
          mentionUserIds: Array.from(mentionData.users),
          mentionRoom: mentionData.room,
          replyTo: sendRelation.replyTo,
          threadRoot: sendRelation.threadRoot,
        });
        if (nativeOwner === 'legacy') {
          await mx.sendMessage(roomId, content as any);
        }
        resetEditor(editor);
        resetEditorHistory(editor);
        clearRoomDraft(window.localStorage, mx.getSafeUserId(), roomId);
        await clearReplyDraftAfterSend(sendRelation.draftRevision, () => {
          setSendError(
            t(
              'composer.reply_clear_after_send_failed',
              'Message sent, but the reply state could not be cleared.'
            )
          );
        });
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
      clearReplyDraftAfterSend,
      isMarkdown,
      commands,
      getReplyRelation,
      sendingMessage,
      selectedFiles,
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
        const sendRelation = nativeComposerSendRelation(replyDraft);
        const owner = await sendPollWithNativeDesktopOwner({
          roomId,
          question: poll.question,
          answers: poll.answers.map((answer) => answer.text),
          maxSelections: poll.maxSelections,
          threadRoot: sendRelation.threadRoot,
          replyTo: sendRelation.replyTo,
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
        await clearReplyDraftAfterSend(sendRelation.draftRevision, () => {
          setPollError(
            t(
              'composer.reply_clear_after_send_failed',
              'Poll sent, but the reply state could not be cleared.'
            )
          );
        });
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
          if (replyDraft) {
            void clearReplyDraft(replyDraft.draftRevision).catch(() => {
              setSendError(t('composer.reply_clear_failed', 'Could not cancel reply.'));
            });
          }
        }
      },
      [submit, clearReplyDraft, enterForNewline, autocompleteQuery, isComposing, replyDraft, t]
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

    const handleGifSelect = async (gif: GifResult) => {
      setGifSending(true);
      setGifSendError(undefined);
      const { draftRevision, replyTo, threadRoot } = nativeComposerSendRelation(replyDraft);
      try {
        await sendComposerGifWithNativeOwner(roomId, gif, replyTo, threadRoot);
        await clearReplyDraftAfterSend(draftRevision, () => {
          setGifSendError(
            t(
              'composer.reply_clear_after_send_failed',
              'GIF sent, but the reply state could not be cleared.'
            )
          );
        });
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
                    onClick={() => {
                      void clearReplyDraft(replyDraft.draftRevision).catch(() => {
                        setSendError(t('composer.reply_clear_failed', 'Could not cancel reply.'));
                      });
                    }}
                    variant="SurfaceVariant"
                    size="300"
                    radii="300"
                    aria-label={t('composer.reply.cancel_aria_label', 'Cancel reply')}
                  >
                    <Icon src={Icons.Cross} size="50" />
                  </IconButton>
                  <Box direction="Row" gap="200" alignItems="Center">
                    {replyDraft.threadRootEventId && <ThreadIndicator />}
                    <ReplyLayout
                      userColor={replyUsernameColor}
                      username={
                        <Text size="T300" truncate>
                          <b>
                            {getMemberDisplayName(room, replyDraft.senderId) ??
                              getMxIdLocalPart(replyDraft.senderId) ??
                              replyDraft.senderId}
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
                <Menu className={depthCss.floatingSurface} style={{ width: toRem(196) }}>
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
                className={depthCss.quietInteractiveSurface}
                ref={composerToolsBtnRef}
                onClick={(event) =>
                  setComposerToolsAnchor(
                    composerToolsAnchor ? undefined : event.currentTarget.getBoundingClientRect()
                  )
                }
                variant="Surface"
                size="300"
                radii="Pill"
                aria-label="More message actions"
                aria-expanded={!!composerToolsAnchor}
              >
                <Icon src={Icons.Plus} size="100" />
              </IconButton>
            </PopOut>
          }
          floatingActions={
            <>
              <IconButton
                className={depthCss.quietInteractiveSurface}
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
                  !emojiBoardOpen
                    ? undefined
                    : emojiBtnRef.current?.getBoundingClientRect() ?? undefined
                }
                content={
                  <EmojiBoard
                    imagePackRooms={imagePackRooms}
                    returnFocusOnDeactivate={false}
                    onEmojiSelect={handleEmoticonSelect}
                    onCustomEmojiSelect={handleEmoticonSelect}
                    requestClose={() => {
                      if (emojiBoardOpen && !mobileOrTablet()) ReactEditor.focus(editor);
                      setEmojiBoardOpen(false);
                    }}
                  />
                }
              >
                <IconButton
                  className={depthCss.quietInteractiveSurface}
                  ref={emojiBtnRef}
                  aria-pressed={emojiBoardOpen}
                  aria-label={t('composer.emoji_picker_aria_label', 'Emoji picker')}
                  onClick={() => setEmojiBoardOpen((open) => !open)}
                  variant="SurfaceVariant"
                  size="300"
                  radii="300"
                >
                  <Icon src={Icons.Smile} filled={emojiBoardOpen} />
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
                        className={depthCss.floatingSurface}
                        direction="Column"
                        gap="300"
                        style={{
                          padding: config.space.S400,
                          width: toRem(280),
                          borderRadius: config.radii.R400,
                        }}
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
                    className={depthCss.floatingSurface}
                    direction="Column"
                    gap="300"
                    style={{
                      padding: config.space.S400,
                      width: toRem(320),
                      borderRadius: config.radii.R400,
                    }}
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
                className={depthCss.quietInteractiveSurface}
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
