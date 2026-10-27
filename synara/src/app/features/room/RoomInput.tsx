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
import { useAtom, useAtomValue, useStore } from 'jotai';
import { isKeyHotkey } from 'is-hotkey';
import type { EventedRoomReading } from '../../utils/roomEvents';
import { MsgType } from '../../utils/messageContent';
import { useTranslation } from 'react-i18next';
import { ReactEditor } from 'slate-react';
import { Descendant, Transforms, Editor } from 'slate';
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
  Spinner,
  Text,
  config,
  toRem,
} from 'folds';
import FocusTrap from '../../components/FocusTrap';

import { requestRoomLatestAfterSend } from './nativeTimelineNavigation';
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
  createMentionElement,
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
  type AutocompleteSelectFirstRef,
} from '../../components/editor';
import { EmojiBoard } from '../../components/emoji-board';
import { TUploadContent, getMxIdLocalPart } from '../../utils/matrix';
import { useTypingStatusUpdater } from '../../hooks/useTypingStatusUpdater';
import { useFilePicker } from '../../hooks/useFilePicker';
import {
  FILE_DROP_REGION_ATTRIBUTE,
  useFileDropZone,
  windowPasteTargetsComposer,
} from '../../hooks/useFileDrop';
import {
  TUploadItem,
  TUploadMetadata,
  roomIdToMsgDraftAtomFamily,
  roomIdToUploadItemsAtomFamily,
  roomUploadAtomFamily,
} from '../../state/room/roomInputDrafts';
import { composerMentionInsertAtom } from '../../state/composerMentionInsert';
import { UploadCardRenderer } from '../../components/upload-card';
import {
  UploadBoard,
  UploadBoardContent,
  UploadBoardHeader,
  UploadSendOptions,
} from '../../components/upload-board';
import { Upload, UploadStatus, createUploadFamilyObserverAtom } from '../../state/upload';
import {
  editableActiveElement,
  getDataTransferFiles,
  shouldProbeNativeClipboardImage,
  snapshotClipboardText,
} from '../../utils/dom';
import { safeFile } from '../../utils/mimeTypes';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom } from '../../state/settings';
import { getMemberDisplayName, trimReplyFromBody } from '../../utils/room';
import { stopPropagation } from '../../utils/keyboard';
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
import { gifPickerEnabled, gifSearchAvailable, gifSendsAsLink } from '../../utils/gifProvider';
import type { GifResult } from '../../utils/gifProvider';
import { GifPicker } from './gif/GifPicker';
import { isTypedGifCommand } from './typedGifCommand';
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
  sendComposerAttachmentWithNativeOwner,
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
import {
  COMPOSER_UNFURL_DEBOUNCE_MS,
  fetchMediaPreviewWithNativeOwner,
  trailingComposerPreviewUrl,
  type NativeMediaPreview,
} from './nativeLinkUnfurl';
import { NativeLinkUnfurlCard } from './nativeLinkUnfurlCard';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { useNativeRoomSummary } from '../../state/room-list/roomList';
import { useNativeThreadRoot } from './nativeThreadViewContext';
import type { AttachmentSendPlan } from './attachmentSendPlan';
import {
  completeAttachmentSendStep,
  hasTrailingAttachmentText,
  makeOrReuseAttachmentSendPlan,
} from './attachmentSendPlan';
import { renderComposerHtml } from '../../components/editor/composerMarkdown';
import { getMyUserId, getSafeMyUserId } from '../../state/nativeIdentity';
import {
  ComposerSendQueue,
  cloneComposerContent,
  mergeRestoredComposerContent,
  restoreComposerContent,
  stashComposerSendError,
  takeComposerSendError,
} from './composerSendQueue';
import {
  COMPOSER_NOT_SIGNED_IN,
  ComposerUserError,
  composerCommandErrorMessage,
  composerErrorMessage,
  composerSendErrorMessage,
} from './composerSendError';
import type { NativeUploadSendState } from '../../components/upload-card/UploadCardRenderer';
import { useMediaConfig } from '../../hooks/useMediaConfig';
import {
  attachmentTooLargeMessage,
  effectiveNativeAttachmentLimit,
  mediaConfigUploadLimit,
} from '../../utils/nativeMediaLimits';

/** Commands that send a message rather than run an action. */
const MESSAGE_COMMANDS: ReadonlySet<string> = new Set([
  Command.Me,
  Command.Notice,
  Command.Shrug,
  Command.TableFlip,
  Command.UnFlip,
]);

/** One queued composer send, with what goes back to the composer if it fails. */
type ComposerSendItem = {
  draftKey: string;
  content: Descendant[];
  /** Set for an action command such as /invite, for its error copy. */
  commandName?: string;
  /** Composer files this send owns. */
  transactionIds: string[];
  /** The Core reply draft this send consumes. */
  replyRevision?: number;
};

interface RoomInputProps {
  editor: Editor;
  roomId: string;
  room: EventedRoomReading;
  /**
   * The side-pane thread composer sends into this root. It keeps its own
   * draft and uploads; after a send only the view showing this thread moves.
   * Without it the composer follows the room's open thread view.
   */
  threadRootOverride?: string;
}
export const RoomInput = forwardRef<HTMLDivElement, RoomInputProps>(
  ({ editor, roomId, room, threadRootOverride }, ref) => {
    const draftKey = threadRootOverride ? `${roomId}#thread:${threadRootOverride}` : roomId;
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

    const [msgDraft, setMsgDraft] = useAtom(roomIdToMsgDraftAtomFamily(draftKey));
    const [mentionInsert, setMentionInsert] = useAtom(composerMentionInsertAtom);
    useEffect(() => {
      if (!mentionInsert || mentionInsert.roomId !== roomId) return;
      const mentionEl = createMentionElement(
        mentionInsert.userId,
        mentionInsert.name.startsWith('@') ? mentionInsert.name : `@${mentionInsert.name}`,
        getMyUserId() === mentionInsert.userId
      );
      Transforms.insertNodes(editor, mentionEl);
      moveCursor(editor, true);
      ReactEditor.focus(editor);
      setMentionInsert(undefined);
    }, [mentionInsert, roomId, editor, setMentionInsert]);
    const roomThreadRootEventId = useNativeThreadRoot(roomId);
    const threadRootEventId = threadRootOverride ?? roomThreadRootEventId;
    const coreReplyDraft = useNativeComposerReplyDraft(roomId, threadRootEventId);
    // A queued send owns the reply it was written as. The preview hides until
    // the send clears it (or fails and gives it back), so the next message is
    // not sent as a second reply.
    const [claimedReplyRevisions, setClaimedReplyRevisions] = useState<ReadonlySet<number>>(
      () => new Set()
    );
    const replyDraft =
      coreReplyDraft && !claimedReplyRevisions.has(coreReplyDraft.draftRevision)
        ? coreReplyDraft
        : undefined;
    const clearReplyDraft = useCallback(
      async (expectedDraftRevision: number) => {
        const result = await clearNativeComposerReplyDraft({
          roomId,
          expectedDraftRevision,
          threadRootEventId,
        });
        if (result === 'unavailable') {
          throw new Error('Native reply draft clear is unavailable.');
        }
      },
      [roomId, threadRootEventId]
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
    const [selectedFiles, setSelectedFiles] = useAtom(roomIdToUploadItemsAtomFamily(draftKey));
    const [nativeComposerSend, setNativeComposerSend] = useState(false);
    const uploadFamilyObserverAtom = createUploadFamilyObserverAtom(
      roomUploadAtomFamily,
      selectedFiles.map((f) => f.file)
    );
    const attachmentSendPlan = useRef<AttachmentSendPlan | undefined>(undefined);
    // Files owned by a queued send, and the one Core is uploading now.
    const [claimedTransactionIds, setClaimedTransactionIds] = useState<ReadonlySet<string>>(
      () => new Set()
    );
    const [uploadingTransactionId, setUploadingTransactionId] = useState<string>();
    const claimedTransactionIdsRef = useRef<ReadonlySet<string>>(claimedTransactionIds);
    const claimedReplyRevisionsRef = useRef<ReadonlySet<number>>(claimedReplyRevisions);
    const store = useStore();
    const mediaConfig = useMediaConfig();
    const attachmentLimit = effectiveNativeAttachmentLimit(mediaConfigUploadLimit(mediaConfig));
    const autocompleteSelectFirst: AutocompleteSelectFirstRef = useRef(undefined);

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
    useEffect(() => {
      if (!composerToolsAnchor) return undefined;
      const onPointerDown = (event: PointerEvent) => {
        const target = event.target;
        if (!(target instanceof Node)) {
          setComposerToolsAnchor(undefined);
          return;
        }
        if (composerToolsBtnRef.current?.contains(target)) return;
        const menu = document.querySelector('[data-composer-tools-menu="true"]');
        if (menu?.contains(target)) return;
        setComposerToolsAnchor(undefined);
      };
      document.addEventListener('pointerdown', onPointerDown, true);
      return () => document.removeEventListener('pointerdown', onPointerDown, true);
    }, [composerToolsAnchor]);
    const [emojiBoardOpen, setEmojiBoardOpen] = useState(false);
    const [autocompleteQuery, setAutocompleteQuery] =
      useState<AutocompleteQuery<AutocompletePrefix>>();
    const [gifPickerAnchor, setGifPickerAnchor] = useState<RectCords>();
    const [gifSending, setGifSending] = useState(false);
    const [gifSendError, setGifSendError] = useState<string>();
    // Search the picker opens with, from `/gif <query>`; bumping the key
    // remounts the picker so a new command starts a fresh search.
    const [gifPickerQuery, setGifPickerQuery] = useState({ query: '', key: 0 });
    const [sendError, setSendError] = useState<string>();
    const [composerPreviewUrl, setComposerPreviewUrl] = useState<string>();
    const [composerPreview, setComposerPreview] = useState<NativeMediaPreview | null>(null);
    const [composerPreviewDismissed, setComposerPreviewDismissed] = useState<string>();
    // This room only, so other rooms' activity does not re-render the editor.
    const composerEncryptionStatus = useNativeRoomSummary(roomId)?.encryptionStatus;
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
        const sendRelation = nativeComposerSendRelation(replyDraft, threadRootEventId);
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
        if (owner === 'legacy') throw new ComposerUserError(COMPOSER_NOT_SIGNED_IN);
      },
      [clearReplyDraftAfterSend, replyDraft, roomId, t, threadRootEventId]
    );
    const commands = useCommands(
      room as unknown as Parameters<typeof useCommands>[0],
      sendSlashPoll
    );

    const handleFiles = useCallback(
      async (files: File[]) => {
        setUploadBoard(true);
        const safeFiles = files.map(safeFile);
        const fileItems: TUploadItem[] = [];
        // Native Rust encrypts in e2e rooms. JS encrypt is retired.
        const nativeReady = await nativeComposerAttachmentReady().catch(() => false);
        setNativeComposerSend(nativeReady);
        if (!nativeReady) {
          // Callers (drop, paste, picker) do not await; report here.
          setSendError(
            'Synara is not signed in to your Matrix account, so files can not be attached. Sign in again, then attach them.'
          );
          return;
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
      // A failed probe falls back to pasting the clipboard's text.
      const file = await readPlatformClipboardImage().catch(() => undefined);
      if (file) {
        void handleFiles([file]);
        return true;
      }
      return false;
    }, [handleFiles]);
    const handlePaste: ClipboardEventHandler = useCallback(
      (evt) => {
        const files = getDataTransferFiles(evt.clipboardData);
        if (files) {
          evt.preventDefault();
          void handleFiles(files);
          return;
        }

        if (isDesktopPlatform() && shouldProbeNativeClipboardImage(evt.clipboardData)) {
          evt.preventDefault();
          // The clipboard is unreadable once this handler returns; keep its
          // text for the fallback that runs after the native image probe.
          const clipboardText = snapshotClipboardText(evt.clipboardData);
          void handleNativeClipboardImage().then((handled) => {
            if (!handled) {
              insertClipboardData(editor, clipboardText);
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
    const dropAnchorRef = useRef<HTMLDivElement | null>(null);
    const setRootRef = useCallback(
      (node: HTMLDivElement | null) => {
        dropAnchorRef.current = node;
        if (typeof ref === 'function') ref(node);
        else if (ref) ref.current = node;
      },
      [ref]
    );
    useEffect(() => {
      const handleWindowPaste = (evt: ClipboardEvent) => {
        if (editableActiveElement()) return;
        const portalContainer = document.getElementById('portalContainer');
        if (portalContainer && portalContainer.children.length > 0) return;
        if (!evt.clipboardData) return;
        // With the thread pane open both composers hear this; only one takes it.
        const ownRegion = dropAnchorRef.current?.closest(`[${FILE_DROP_REGION_ATTRIBUTE}]`);
        if (!windowPasteTargetsComposer(ownRegion, document.activeElement, !threadRootOverride)) {
          return;
        }

        const files = getDataTransferFiles(evt.clipboardData);
        if (!files) {
          if (!isDesktopPlatform() || !shouldProbeNativeClipboardImage(evt.clipboardData)) return;
          evt.preventDefault();
          void handleNativeClipboardImage();
          return;
        }

        evt.preventDefault();
        void handleFiles(files);
      };

      window.addEventListener('paste', handleWindowPaste);
      return () => {
        window.removeEventListener('paste', handleWindowPaste);
      };
    }, [handleFiles, handleNativeClipboardImage, threadRootOverride]);
    const handleDroppedTooLarge = useCallback(
      (rejected: { name: string; size: number }[]) => {
        setSendError(
          rejected
            .map((file) => attachmentTooLargeMessage(file.name, file.size, attachmentLimit))
            .join(' ')
        );
      },
      [attachmentLimit]
    );
    // The thread pane's composer takes drops on the pane; the room composer
    // takes the rest. Files over the upload limit are never read.
    const dropZoneVisible = useFileDropZone(handleFiles, {
      anchorRef: dropAnchorRef,
      primary: !threadRootOverride,
      maxFileBytes: attachmentLimit,
      onTooLarge: handleDroppedTooLarge,
    });

    const isComposing = useComposingCheck();

    // The draft this composer shows now; undefined once it has closed. A send
    // that fails after its composer closed restores into the stored draft.
    const mountedDraftKey = useRef<string | undefined>(draftKey);
    useEffect(() => {
      mountedDraftKey.current = draftKey;
      const stashedError = takeComposerSendError(draftKey);
      if (stashedError) setSendError(stashedError);
      return () => {
        mountedDraftKey.current = undefined;
      };
    }, [draftKey]);

    useEffect(() => {
      const storedDraft = loadRoomDraft(window.localStorage, getSafeMyUserId(), draftKey);
      const draft = msgDraft.length > 0 ? msgDraft : storedDraft;
      if (draft && draft.length > 0) {
        Transforms.insertFragment(editor, draft);
      }
    }, [roomId, draftKey, editor, msgDraft]);

    useEffect(
      () => () => {
        if (!isEmptyEditor(editor)) {
          const parsedDraft = JSON.parse(JSON.stringify(editor.children));
          setMsgDraft(parsedDraft);
          saveRoomDraft(window.localStorage, getSafeMyUserId(), draftKey, parsedDraft);
        } else {
          setMsgDraft([]);
          clearRoomDraft(window.localStorage, getSafeMyUserId(), draftKey);
        }
        resetEditor(editor);
        resetEditorHistory(editor);
      },
      [roomId, draftKey, editor, setMsgDraft]
    );

    const handleEditorChange = useCallback(
      (value: Parameters<EditorChangeHandler>[0]) => {
        if (isEmptyEditor(editor)) {
          clearRoomDraft(window.localStorage, getSafeMyUserId(), draftKey);
          setComposerPreviewUrl(undefined);
          setComposerPreview(null);
          return;
        }
        saveRoomDraft(window.localStorage, getSafeMyUserId(), draftKey, value);
        const nextUrl = trailingComposerPreviewUrl(toPlainText(editor.children, isMarkdown));
        setComposerPreviewUrl(nextUrl);
        if (!nextUrl) setComposerPreview(null);
      },
      [draftKey, editor, isMarkdown]
    );

    useEffect(() => {
      if (!composerPreviewUrl) {
        setComposerPreviewDismissed(undefined);
        setComposerPreview(null);
        return undefined;
      }
      if (
        composerPreviewUrl === composerPreviewDismissed ||
        composerEncryptionStatus !== 'not_encrypted'
      ) {
        if (composerPreviewUrl !== composerPreviewDismissed) {
          setComposerPreview(null);
        }
        return undefined;
      }
      let cancelled = false;
      const timer = window.setTimeout(() => {
        void fetchMediaPreviewWithNativeOwner({
          roomId,
          url: composerPreviewUrl,
          encryptionStatus: composerEncryptionStatus,
          desktopAvailable: isSynaraDesktop(),
          invoke: invokeDesktopWithAvailability,
        }).then((preview) => {
          if (!cancelled) setComposerPreview(preview);
        });
      }, COMPOSER_UNFURL_DEBOUNCE_MS);
      return () => {
        cancelled = true;
        window.clearTimeout(timer);
      };
    }, [composerEncryptionStatus, composerPreviewDismissed, composerPreviewUrl, roomId]);

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

    // Removing a file a queued send owns, before Core starts on it, keeps it
    // from being sent. The file Core is uploading has no remove control.
    const handleCancelUpload = (uploads: Upload[]) => {
      handleRemoveUpload(uploads.map((upload) => upload.file));
    };

    const claimTransactionIds = useCallback((ids: string[], claimed: boolean) => {
      if (ids.length === 0) return;
      const next = new Set(claimedTransactionIdsRef.current);
      ids.forEach((id) => (claimed ? next.add(id) : next.delete(id)));
      claimedTransactionIdsRef.current = next;
      setClaimedTransactionIds(next);
    }, []);

    const claimReplyRevision = useCallback((revision: number | undefined, claimed: boolean) => {
      if (revision === undefined) return;
      const next = new Set(claimedReplyRevisionsRef.current);
      if (claimed) next.add(revision);
      else next.delete(revision);
      claimedReplyRevisionsRef.current = next;
      setClaimedReplyRevisions(next);
    }, []);

    /** Drop one composer file by transaction id, reading the current list. */
    const removeUploadItem = useCallback(
      (key: string, transactionId: string) => {
        const listAtom = roomIdToUploadItemsAtomFamily(key);
        const item = store.get(listAtom).find((f) => f.transactionId === transactionId);
        if (!item) return;
        store.set(listAtom, { type: 'DELETE', item });
        roomUploadAtomFamily.remove(item.file);
      },
      [store]
    );

    /**
     * Send composer files one by one. A file removed from the board before
     * its turn is skipped; each sent file leaves the board as it is sent, so
     * a failure keeps exactly the unsent ones.
     */
    const sendAttachments = useCallback(
      async (
        key: string,
        files: TUploadItem[],
        options: UploadSendOptions,
        relation: { replyTo?: string; threadRoot?: string }
      ) => {
        for (const fileItem of files) {
          const { transactionId } = fileItem;
          const stillListed = store
            .get(roomIdToUploadItemsAtomFamily(key))
            .some((f) => f.transactionId === transactionId);
          if (!stillListed) {
            claimTransactionIds([transactionId], false);
            // eslint-disable-next-line no-continue
            continue;
          }
          const source = fileItem.originalFile;
          setUploadingTransactionId(transactionId);
          try {
            // One file at a time: only the file being sent is held as bytes.
            // eslint-disable-next-line no-await-in-loop
            const bytes = await fileToNativeAttachmentBytes(source);
            // eslint-disable-next-line no-await-in-loop
            const owner = await sendComposerAttachmentWithNativeOwner({
              roomId,
              transactionId,
              file: {
                filename: source.name || 'attachment',
                mimeType: source.type || 'application/octet-stream',
                bytes,
              },
              caption: files.length === 1 ? options.caption : undefined,
              formattedCaption: files.length === 1 ? options.formattedCaption : undefined,
              mentionUserIds: files.length === 1 ? options.mentionUserIds : undefined,
              mentionRoom: files.length === 1 ? options.mentionRoom : undefined,
              replyTo: relation.replyTo,
              threadRoot: relation.threadRoot,
            });
            if (owner !== 'native') throw new ComposerUserError(COMPOSER_NOT_SIGNED_IN);
          } finally {
            setUploadingTransactionId(undefined);
          }
          if (attachmentSendPlan.current) {
            attachmentSendPlan.current = completeAttachmentSendStep(
              attachmentSendPlan.current,
              transactionId
            );
          }
          removeUploadItem(key, transactionId);
          claimTransactionIds([transactionId], false);
        }
      },
      [claimTransactionIds, removeUploadItem, roomId, store]
    );

    /**
     * A queued send failed: it and every send queued behind it go back into
     * the composer they came from, oldest first, above anything typed since.
     */
    const handleSendFailure = (
      error: unknown,
      failed: ComposerSendItem,
      skipped: ComposerSendItem[]
    ) => {
      const items = [failed, ...skipped];
      items.forEach((item) => {
        claimTransactionIds(item.transactionIds, false);
        claimReplyRevision(item.replyRevision, false);
      });
      const restored = items.flatMap((item) => item.content);
      const message = failed.commandName
        ? composerCommandErrorMessage(failed.commandName, error)
        : composerSendErrorMessage(error);
      if (mountedDraftKey.current === failed.draftKey) {
        restoreComposerContent(editor, restored);
        setSendError(message);
        return;
      }
      // The composer closed (room or thread pane left): keep the text as its
      // draft and show the error when it opens again.
      const draftAtom = roomIdToMsgDraftAtomFamily(failed.draftKey);
      const inMemory = store.get(draftAtom);
      const existing =
        inMemory.length > 0
          ? inMemory
          : loadRoomDraft(window.localStorage, getSafeMyUserId(), failed.draftKey);
      const merged = mergeRestoredComposerContent(restored, existing);
      store.set(draftAtom, merged);
      saveRoomDraft(window.localStorage, getSafeMyUserId(), failed.draftKey, merged);
      stashComposerSendError(failed.draftKey, message);
    };
    const sendFailureHandler = useRef(handleSendFailure);
    sendFailureHandler.current = handleSendFailure;
    const [waitingSends, setWaitingSends] = useState(0);
    const [sendQueue] = useState(
      () =>
        new ComposerSendQueue<ComposerSendItem>(
          (error, failed, skipped) => sendFailureHandler.current(error, failed, skipped),
          setWaitingSends
        )
    );

    /**
     * Files ready to go with the next send, or user copy for why they are
     * not. Files a queued send already owns are left to that send.
     */
    const takeSendableFiles = useCallback((): TUploadItem[] | string => {
      const files = store
        .get(roomIdToUploadItemsAtomFamily(draftKey))
        .filter((f) => !claimedTransactionIdsRef.current.has(f.transactionId));
      for (const fileItem of files) {
        const upload = store.get(roomUploadAtomFamily(fileItem.file));
        if (upload.status === UploadStatus.Error) return upload.error.message;
        if (fileItem.originalFile.size > attachmentLimit) {
          return attachmentTooLargeMessage(
            fileItem.originalFile.name,
            fileItem.originalFile.size,
            attachmentLimit
          );
        }
        if (upload.status !== UploadStatus.Success) {
          return 'Attachments are still being prepared. Try again in a moment.';
        }
      }
      return files;
    }, [attachmentLimit, draftKey, store]);

    /**
     * Queue what the composer holds and clear it at once, so the user can
     * write the next message while Core waits for the server. A failed send
     * puts its text back (see `handleSendFailure`); nothing typed is lost,
     * and a second press can never send the same content twice.
     */
    const submit = useCallback(
      (options: { filesOnly?: boolean } = {}) => {
        const { filesOnly = false } = options;
        const content = filesOnly ? [] : cloneComposerContent(editor.children);
        let plainText = filesOnly ? '' : toPlainText(content, isMarkdown).trim();
        // `/gif` typed out, not picked from the command list, still opens the picker.
        const commandName = filesOnly
          ? undefined
          : (getBeginCommand(editor) ?? (isTypedGifCommand(plainText) ? Command.Gif : undefined));
        let msgType: 'm.text' | 'm.emote' | 'm.notice' = MsgType.Text;
        if (commandName) plainText = trimCommand(commandName, plainText);

        if (commandName === Command.Gif) {
          if (!gifPickerAvailable && !gifOnboardingVisible) {
            setSendError(
              gifProviderAvailable
                ? t(
                    'modernization.gif.command_disabled',
                    'Turn on GIF Search in Settings > General to use /gif.'
                  )
                : t(
                    'modernization.gif.command_unconfigured',
                    'GIF search is not set up for this app.'
                  )
            );
            return;
          }
          resetEditor(editor);
          resetEditorHistory(editor);
          sendTypingStatus(false);
          setSendError(undefined);
          setGifSendError(undefined);
          setGifPickerQuery((current) => ({ query: plainText, key: current.key + 1 }));
          setGifPickerAnchor(
            (composerToolsBtnRef.current ?? dropAnchorRef.current)?.getBoundingClientRect()
          );
          return;
        }

        if (commandName && !MESSAGE_COMMANDS.has(commandName)) {
          const commandContent = commands[commandName as Command];
          if (!commandContent) {
            setSendError(`Synara does not know the command /${commandName}.`);
            return;
          }
          resetEditor(editor);
          resetEditorHistory(editor);
          sendTypingStatus(false);
          setSendError(undefined);
          sendQueue.enqueue({ draftKey, content, commandName, transactionIds: [] }, () =>
            commandContent.exe(plainText)
          );
          return;
        }

        const files = takeSendableFiles();
        if (typeof files === 'string') {
          setSendError(files);
          return;
        }

        let prefix = '';
        if (commandName === Command.Me) msgType = MsgType.Emote;
        else if (commandName === Command.Notice) msgType = MsgType.Notice;
        else if (commandName === Command.Shrug) prefix = SHRUG;
        else if (commandName === Command.TableFlip) prefix = TABLEFLIP;
        else if (commandName === Command.UnFlip) prefix = UNFLIP;
        const body = prefix ? `${prefix} ${plainText}` : plainText;
        if (body === '' && files.length === 0) return;
        const mentionData = getMentions(roomId, editor);

        // A reply already claimed by a queued send is not reused.
        const sendReply =
          coreReplyDraft && !claimedReplyRevisionsRef.current.has(coreReplyDraft.draftRevision)
            ? coreReplyDraft
            : undefined;
        if (sendReply && sendReply.senderId !== getMyUserId()) {
          mentionData.users.add(sendReply.senderId);
        }
        const mentionUserIds = Array.from(mentionData.users);
        const sendRelation = nativeComposerSendRelation(sendReply, threadRootEventId);
        const transactionIds = files.map((file) => file.transactionId);
        const sendDraftKey = draftKey;

        claimTransactionIds(transactionIds, true);
        claimReplyRevision(sendRelation.draftRevision, true);
        if (!filesOnly) {
          resetEditor(editor);
          resetEditorHistory(editor);
          sendTypingStatus(false);
        }
        setSendError(undefined);

        const afterSent = async () => {
          requestRoomLatestAfterSend(roomId, sendRelation.threadRoot);
          await clearReplyDraftAfterSend(sendRelation.draftRevision, () => {
            setSendError(
              t(
                'composer.reply_clear_after_send_failed',
                'Message sent, but the reply state could not be cleared.'
              )
            );
          });
          claimReplyRevision(sendRelation.draftRevision, false);
        };

        sendQueue.enqueue(
          {
            draftKey: sendDraftKey,
            content,
            transactionIds,
            replyRevision: sendReply?.draftRevision,
          },
          async () => {
            let customHtml = trimCustomHtml(
              isMarkdown
                ? await renderComposerHtml(content)
                : toMatrixCustomHTML(content, { allowTextFormatting: true })
            );
            if (commandName) customHtml = trimCommand(commandName, customHtml);
            const formattedBody = prefix ? `${prefix} ${customHtml}` : customHtml;
            const htmlDiffers = !customHtmlEqualsPlainText(formattedBody, body);

            if (files.length > 0) {
              const plan = makeOrReuseAttachmentSendPlan(
                attachmentSendPlan.current,
                transactionIds,
                body
              );
              attachmentSendPlan.current = plan;
              const caption = plan.textRole === 'caption' ? body : undefined;
              await sendAttachments(
                sendDraftKey,
                files,
                {
                  caption,
                  formattedCaption: caption && htmlDiffers ? formattedBody : undefined,
                  mentionUserIds: caption ? mentionUserIds : undefined,
                  mentionRoom: caption ? mentionData.room : undefined,
                },
                sendRelation
              );
              if (!hasTrailingAttachmentText(plan)) {
                attachmentSendPlan.current = undefined;
                await afterSent();
                return;
              }
              attachmentSendPlan.current = undefined;
            }

            const nativeOwner = await sendPlainTextWithNativeOwner({
              roomId,
              body,
              msgType,
              formattedBody: sendReply || htmlDiffers ? formattedBody : undefined,
              mentionUserIds,
              mentionRoom: mentionData.room,
              replyTo: sendRelation.replyTo,
              threadRoot: sendRelation.threadRoot,
            });
            // Without a native session there is nothing that can send this; the
            // retired web fallback dropped reply and thread relations.
            if (nativeOwner === 'legacy') throw new ComposerUserError(COMPOSER_NOT_SIGNED_IN);
            await afterSent();
          }
        );
      },
      [
        roomId,
        draftKey,
        editor,
        coreReplyDraft,
        sendTypingStatus,
        clearReplyDraftAfterSend,
        isMarkdown,
        commands,
        sendQueue,
        takeSendableFiles,
        claimTransactionIds,
        claimReplyRevision,
        sendAttachments,
        t,
        threadRootEventId,
        gifPickerAvailable,
        gifOnboardingVisible,
        gifProviderAvailable,
      ]
    );

    const handleSendUpload = () => submit({ filesOnly: true });
    const uploadSendState = (transactionId: string): NativeUploadSendState | undefined => {
      if (uploadingTransactionId === transactionId) return 'uploading';
      return claimedTransactionIds.has(transactionId) ? 'waiting' : undefined;
    };

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
        const sendRelation = nativeComposerSendRelation(replyDraft, threadRootEventId);
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
              'Sign in to your Matrix account in Synara to send polls.'
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
          // Enter picks the open suggestion, as Tab does; Mod+Enter always
          // sends. Short emoticon queries (":D", ":P") stay plain text.
          // A fully typed `/gif` opens the picker rather than becoming a chip.
          const pickSuggestion =
            autocompleteQuery &&
            !isKeyHotkey('mod+enter', evt) &&
            !(
              autocompleteQuery.prefix === AutocompletePrefix.Emoticon &&
              autocompleteQuery.text.length < 2
            ) &&
            !(
              autocompleteQuery.prefix === AutocompletePrefix.Command &&
              autocompleteQuery.text === Command.Gif
            );
          if (pickSuggestion && autocompleteSelectFirst.current?.()) return;
          submit();
          return;
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
      const { draftRevision, replyTo, threadRoot } = nativeComposerSendRelation(
        replyDraft,
        threadRootEventId
      );
      try {
        if (gifSendsAsLink(gif.provider)) {
          const owner = await sendPlainTextWithNativeOwner({
            roomId,
            body: gif.url,
            msgType: MsgType.Text,
            replyTo,
            threadRoot,
          });
          if (owner === 'legacy') throw new ComposerUserError(COMPOSER_NOT_SIGNED_IN);
          requestRoomLatestAfterSend(roomId, threadRoot);
        } else {
          await sendComposerGifWithNativeOwner(roomId, gif, replyTo, threadRoot);
        }
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
        setGifSendError(composerErrorMessage(err, 'Synara could not send the GIF. Try again.'));
      } finally {
        setGifSending(false);
      }
    };

    return (
      <div ref={setRootRef}>
        {selectedFiles.length > 0 && (
          <UploadBoard
            header={
              <UploadBoardHeader
                open={uploadBoard}
                onToggle={() => setUploadBoard(!uploadBoard)}
                uploadFamilyObserverAtom={uploadFamilyObserverAtom}
                onSend={handleSendUpload}
                sending={selectedFiles.some((f) => claimedTransactionIds.has(f.transactionId))}
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
                        sendState={uploadSendState(fileItem.transactionId)}
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
        {waitingSends > 0 && (
          // Queued messages reach the timeline only once the one before is
          // accepted, so say where they are meanwhile.
          <Box
            gap="200"
            alignItems="Center"
            role="status"
            style={{ padding: `0 ${config.space.S300} ${config.space.S200}` }}
          >
            <Spinner variant="Secondary" size="100" />
            <Text size="T200" priority="300">
              {waitingSends === 1
                ? '1 more message waiting to send…'
                : `${waitingSends} more messages waiting to send…`}
            </Text>
          </Box>
        )}
        {autocompleteQuery?.prefix === AutocompletePrefix.RoomMention && (
          <RoomMentionAutocomplete
            roomId={roomId}
            editor={editor}
            query={autocompleteQuery}
            requestClose={handleCloseAutocomplete}
            selectFirstRef={autocompleteSelectFirst}
          />
        )}
        {autocompleteQuery?.prefix === AutocompletePrefix.UserMention && (
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
        {autocompleteQuery?.prefix === AutocompletePrefix.Command && (
          <CommandAutocomplete
            room={room}
            editor={editor}
            query={autocompleteQuery}
            requestClose={handleCloseAutocomplete}
            selectFirstRef={autocompleteSelectFirst}
          />
        )}
        <RoomComposer
          editableName="RoomInput"
          editor={editor}
          placeholder="Send a message..."
          ariaLabel={
            threadRootEventId
              ? t('composer.thread_aria_label', 'Reply in thread')
              : t('composer.room_aria_label', {
                  room: room?.name || roomId,
                  defaultValue: 'Message {{room}}',
                })
          }
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
          linkPreview={
            composerPreview &&
            composerPreview.url === composerPreviewUrl &&
            composerPreview.url !== composerPreviewDismissed ? (
              <div style={{ padding: `0 ${config.space.S300} ${config.space.S200}` }}>
                <NativeLinkUnfurlCard
                  preview={composerPreview}
                  onDismiss={() => {
                    setComposerPreviewDismissed(composerPreview.url);
                    setComposerPreview(null);
                  }}
                />
              </div>
            ) : undefined
          }
          leadingAction={
            <PopOut
              anchor={composerToolsAnchor}
              position="Top"
              align="Start"
              offset={12}
              content={
                <FocusTrap
                  focusTrapOptions={{
                    initialFocus: false,
                    onDeactivate: () => setComposerToolsAnchor(undefined),
                    clickOutsideDeactivates: true,
                    isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
                    isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
                    escapeDeactivates: stopPropagation,
                    returnFocusOnDeactivate: false,
                  }}
                >
                  <Menu
                    className={depthCss.floatingSurface}
                    style={{ width: toRem(196) }}
                    data-composer-tools-menu="true"
                  >
                    <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
                      <MenuItem
                        className={depthCss.quietInteractiveSurface}
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
                          className={depthCss.quietInteractiveSurface}
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
                        className={depthCss.quietInteractiveSurface}
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
                </FocusTrap>
              }
            >
              <IconButton
                className={css.ComposerAction}
                fill="None"
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
                className={css.ComposerAction}
                fill="None"
                variant="Surface"
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
                    : (emojiBtnRef.current?.getBoundingClientRect() ?? undefined)
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
                  className={css.ComposerAction}
                  fill="None"
                  ref={emojiBtnRef}
                  aria-pressed={emojiBoardOpen}
                  aria-label={t('composer.emoji_picker_aria_label', 'Emoji picker')}
                  onClick={() => setEmojiBoardOpen((open) => !open)}
                  variant="Surface"
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
                    <FocusTrap
                      focusTrapOptions={{
                        initialFocus: false,
                        onDeactivate: () => setGifPickerAnchor(undefined),
                        clickOutsideDeactivates: true,
                        escapeDeactivates: stopPropagation,
                        returnFocusOnDeactivate: false,
                      }}
                    >
                      {gifPickerAvailable ? (
                        <GifPicker
                          key={gifPickerQuery.key}
                          initialQuery={gifPickerQuery.query}
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
                            {gifSendsAsLink(clientConfig.gifPicker?.provider)
                              ? t(
                                  'modernization.gif.onboarding.description_link',
                                  'GIF search contacts the GIF provider from this device, so it can see your IP address. GIFs you send are links that Synara plays inline.'
                                )
                              : t(
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
                      )}
                    </FocusTrap>
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
                  <FocusTrap
                    focusTrapOptions={{
                      initialFocus: false,
                      onDeactivate: () => setPollAnchor(undefined),
                      clickOutsideDeactivates: true,
                      escapeDeactivates: stopPropagation,
                      returnFocusOnDeactivate: false,
                    }}
                  >
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
                          <Text size="B300">
                            {t('modernization.poll.add_option', 'Add option')}
                          </Text>
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
                  </FocusTrap>
                }
              >
                {null}
              </PopOut>
              <IconButton
                className={css.ComposerAction}
                fill="None"
                onClick={() => submit()}
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
