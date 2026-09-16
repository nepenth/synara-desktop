import { observeRoomLatestAfterSend } from './nativeTimelineNavigation';
import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useFocusWithin, useHover } from 'react-aria';
import FocusTrap from 'focus-trap-react';
import { ErrorBoundary } from 'react-error-boundary';
import {
  Avatar,
  Box,
  Button,
  Icon,
  IconButton,
  Icons,
  Menu,
  MenuItem,
  Line,
  Modal,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  PopOut,
  RectCords,
  Scroll,
  Spinner,
  Text,
  Tooltip,
  TooltipProvider,
  config,
} from 'folds';
import { EmojiBoard } from '../../components/emoji-board';
import { AgentApprovalCard } from '../../components/agent-approval/AgentApprovalCard';
import { setNativeComposerReplyDraft } from './nativeComposerDraft';
import { createLaterItemFromIds, upsertLaterWithNativeOwner } from './nativeLaterOwner';
import {
  nativeReactionViewFromEventReadback,
  nativeReactionsForViewer,
  toggleReactionWithNativeOwner,
  type NativeReactionReadback,
} from './nativeReactionOwner';
import { ReactionViewer } from './reaction-viewer';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { observeNativeTimelineBottom } from './nativeTimelineVisibility';
import {
  editTextWithNativeTimelineAction,
  forwardMediaWithNativeTimelineAction,
  forwardTextWithNativeTimelineAction,
  pinWithNativeTimelineAction,
  pollVoteWithNativeTimelineAction,
  redactWithNativeTimelineAction,
  reportWithNativeTimelineAction,
  unpinWithNativeTimelineAction,
} from './nativeTimelineAction';
import {
  callDeclineWithNativeTimelineOwner,
  isNativeTimelineForwardMedia,
  isNativeTimelineForwardTransport,
  NativePollFlightCoordinator,
  NativeReactionFlightCoordinator,
  nativePollSubmission,
  selectNativeTimelinePinAction,
  toggleNativePollSelection,
} from './nativeTimelineActions';
import {
  editedFormattedBodyForSubmit,
  filterNativeForwardTargets,
  isNativeTimelineEventPinned,
  nativeForwardEncryptionDecision,
  nativeThreadFocusEventId,
  nativeTimelineMediaSrc,
  parseNativeTimelineAgentCard,
  type NativeTimelineMediaHandle,
  type NativeTimelinePollAnswer,
  type NativeTimelineReaction,
  type NativeTimelineReplyPreview,
  type NativeTimelineRowCapabilities,
  type NativeTimelineThreadSummary,
  type NativeTimelineViewRow,
  useNativeTimelineView,
} from './nativeTimelineView';
import type { RoomEncryptionStatus } from '../matrix-dto/room';
import { useNativeRoomListSnapshot } from '../../state/room-list/roomList';
import { Time } from '../../components/message';
import { UserAvatar } from '../../components/user-avatar';
import { useSetting } from '../../state/hooks/settings';
import { settingsAtom, type MessageSpacing } from '../../state/settings';
import { getMxIdLocalPart } from '../../utils/matrix';
import { nameInitials } from '../../utils/common';
import { invokeDesktopWithAvailability, isSynaraDesktop } from '../../utils/desktop';
import { stopPropagation } from '../../utils/keyboard';
import { formatCoreAgentApprovalPrompt } from '../../utils/agentApprovals';
import { NativeFormattedBody } from './nativeTimelineFormattedBody';
import { copyRichTextToClipboard } from '../../utils/dom';
import { prepareNativeFormattedBody } from './nativeTimelineRichText';
import {
  estimateNativeTimelineRowSize,
  NATIVE_TIMELINE_DEFAULT_ROW_ESTIMATE_PX,
  NATIVE_TIMELINE_MEDIA_MAX_PX,
  NATIVE_TIMELINE_STICKER_MAX_PX,
  nativeFollowLiveAttemptKey,
  nativeFollowLiveTarget,
  nativeLiveReadAttemptKey,
  nativeLiveReadTarget,
  nativeTimelineMeasuredSize,
  nativeTimelineMeasuredSizeIdentity,
  nativeTimelineMeasuredSizeKey,
  nativeVisibleReadFrontier,
  latestNativeReadEventId,
  rememberNativeTimelineMeasuredSize,
  reservedNativeTimelineMediaSize,
  shouldShowJumpToLastRead,
  shouldShowJumpToLatest,
  type NativeTimelineRowSizeHint,
} from './nativeTimelineViewportPolicy';
import { shouldGroupNativeTimelineRows } from './nativeTimelineGrouping';
import * as htmlCss from './nativeTimelineHtml.css';
import * as depthCss from '../../styles/Depth.css';

const HermesAgentCard = React.lazy(() =>
  import('../../components/hermes/HermesAgentCard').then((module) => ({
    default: module.HermesAgentCard,
  }))
);

type NativeTimelinePresenterProps = {
  roomId: string;
  eventId?: string;
};

type NativeTimelineViewport = {
  atBottom: boolean;
  anchor?: {
    itemId: string;
    eventId?: string;
    offsetPx: number;
    visualTopPx?: number;
  };
};

const NATIVE_TIMELINE_VIEWPORT_LIMIT = 100;
const nativeTimelineViewports = new Map<string, NativeTimelineViewport>();

const setNativeTimelineViewport = (roomId: string, viewport: NativeTimelineViewport) => {
  nativeTimelineViewports.delete(roomId);
  nativeTimelineViewports.set(roomId, viewport);
  if (nativeTimelineViewports.size > NATIVE_TIMELINE_VIEWPORT_LIMIT) {
    const oldestRoomId = nativeTimelineViewports.keys().next().value;
    if (oldestRoomId) nativeTimelineViewports.delete(oldestRoomId);
  }
};

const rowKey = (row: NativeTimelineViewRow): string => {
  if (row.kind === 'sticker') return row.event.itemId;
  return row.itemId;
};

const rowEventId = (row: NativeTimelineViewRow): string | undefined => {
  if (row.kind === 'sticker') return row.event.eventId;
  if ('eventId' in row) return row.eventId;
  return undefined;
};

const findAnchorIndex = (
  rows: NativeTimelineViewRow[],
  anchor: Pick<NonNullable<NativeTimelineViewport['anchor']>, 'itemId' | 'eventId'>
): number =>
  rows.findIndex(
    (row) => rowKey(row) === anchor.itemId || (anchor.eventId && rowEventId(row) === anchor.eventId)
  );

const parkedNodeVisualTop = (scrollEl: HTMLElement, eventId: string): number | undefined => {
  const node = scrollEl.querySelector(`[data-native-timeline-event-id="${CSS.escape(eventId)}"]`);
  if (!(node instanceof HTMLElement)) return undefined;
  return node.getBoundingClientRect().top - scrollEl.getBoundingClientRect().top;
};

const unreadAnchorIsMissing = (
  selectedPosition: { kind: string; anchor_event_id?: string } | undefined,
  rows: NativeTimelineViewRow[]
): boolean =>
  selectedPosition?.kind === 'unread' &&
  Boolean(selectedPosition.anchor_event_id) &&
  findAnchorIndex(rows, {
    itemId: selectedPosition.anchor_event_id ?? '',
    eventId: selectedPosition.anchor_event_id,
  }) < 0;

const rowCapabilities = (row: NativeTimelineViewRow): NativeTimelineRowCapabilities | undefined => {
  if (row.kind === 'sticker') return row.event.capabilities;
  if (row.kind === 'other') return row.event?.capabilities;
  if ('capabilities' in row) return row.capabilities;
  return undefined;
};

const rowOriginServerTs = (row: NativeTimelineViewRow): number | undefined => {
  if (row.kind === 'sticker') return row.event.originServerTs;
  if (row.kind === 'other') return row.event?.originServerTs;
  if ('originServerTs' in row && typeof row.originServerTs === 'number') return row.originServerTs;
  return undefined;
};

const rowSenderId = (row: NativeTimelineViewRow | undefined): string | undefined => {
  if (!row) return undefined;
  if (row.kind === 'sticker') return row.event.senderId;
  if (row.kind === 'other') return row.event?.senderId;
  if ('senderId' in row) return row.senderId;
  return undefined;
};

const rowSenderName = (row: NativeTimelineViewRow): string => {
  if (row.kind === 'sticker') return row.event.senderName;
  if (row.kind === 'other') return row.event?.senderName ?? rowSenderId(row) ?? '';
  if ('senderName' in row) return row.senderName;
  return rowSenderId(row) ?? '';
};

const rowSenderAvatarUrl = (row: NativeTimelineViewRow): string | undefined => {
  if (row.kind === 'sticker') return row.event.senderAvatarUrl;
  if (row.kind === 'other') return row.event?.senderAvatarUrl;
  if ('senderAvatarUrl' in row) return row.senderAvatarUrl;
  return undefined;
};

const displayNameForRow = (row: NativeTimelineViewRow): string => {
  const name = rowSenderName(row).trim();
  const senderId = rowSenderId(row) ?? '';
  if (name && name !== senderId) return name;
  return getMxIdLocalPart(senderId) ?? name ?? senderId;
};

const isGroupedWithPrevious = (
  previous: NativeTimelineViewRow | undefined,
  row: NativeTimelineViewRow | undefined
): boolean =>
  shouldGroupNativeTimelineRows(
    previous
      ? { senderId: rowSenderId(previous), originServerTs: rowOriginServerTs(previous) }
      : undefined,
    row ? { senderId: rowSenderId(row), originServerTs: rowOriginServerTs(row) } : undefined
  );

const mediaStyle = (
  media?: NativeTimelineMediaHandle,
  maxBox = NATIVE_TIMELINE_MEDIA_MAX_PX
): React.CSSProperties => {
  const maxWidth = media?.width ? Math.min(media.width, maxBox) : maxBox;
  const maxHeight = media?.height ? Math.min(media.height, maxBox) : maxBox;
  const reserved = reservedNativeTimelineMediaSize(
    media?.width,
    media?.height,
    maxWidth,
    maxHeight
  );
  if (reserved && media?.width && media?.height) {
    return {
      width: reserved.width,
      height: reserved.height,
      maxWidth,
      maxHeight,
      aspectRatio: `${media.width} / ${media.height}`,
      objectFit: 'contain',
      display: 'block',
    };
  }
  return { maxWidth, maxHeight, width: 'auto', height: 'auto' };
};

const rowReactionCount = (row: NativeTimelineViewRow): number => {
  if (row.kind === 'sticker') return row.reactions?.length ?? 0;
  if ('reactions' in row) return row.reactions?.length ?? 0;
  return 0;
};

const nativeTimelineRowSizeHint = (
  row: NativeTimelineViewRow,
  grouped: boolean
): NativeTimelineRowSizeHint => ({
  kind: row.kind,
  grouped,
  bodyLineCount: row.kind === 'message' ? Math.max(1, row.body.split('\n').length) : undefined,
  bodyLength: row.kind === 'message' ? row.body.length : undefined,
  hasFormattedCode:
    row.kind === 'message' && Boolean(row.formattedBody && row.formattedBody.includes('<pre')),
  messageType: row.kind === 'message' ? row.messageType : undefined,
  mediaWidth:
    row.kind === 'sticker'
      ? row.media.width
      : row.kind === 'message'
      ? row.media?.width
      : undefined,
  mediaHeight:
    row.kind === 'sticker'
      ? row.media.height
      : row.kind === 'message'
      ? row.media?.height
      : undefined,
  reactionCount: rowReactionCount(row),
});

const measuredSizeKeyForRow = (
  roomId: string,
  rows: readonly NativeTimelineViewRow[],
  index: number
): string | undefined => {
  const row = rows[index];
  if (!row) return undefined;
  return nativeTimelineMeasuredSizeKey(
    roomId,
    rowKey(row),
    nativeTimelineMeasuredSizeIdentity(
      nativeTimelineRowSizeHint(row, isGroupedWithPrevious(rows[index - 1], row))
    )
  );
};

const hasMessageSurface = (kind: NativeTimelineViewRow['kind']): boolean =>
  kind === 'message' ||
  kind === 'sticker' ||
  kind === 'poll' ||
  kind === 'call' ||
  kind === 'redacted' ||
  kind === 'encrypted_unavailable';

type NativeTimelineRowProps = {
  row: NativeTimelineViewRow;
  grouped: boolean;
  groupsNext: boolean;
  roomId: string;
  sessionGeneration: number;
  messageSpacing: MessageSpacing;
  pinnedEventIds?: string[];
  sourceEncryptionStatus?: RoomEncryptionStatus;
  onActionError: (message: string) => void;
  onFocusEvent: (eventId: string) => void;
  onViewReactions: (request: {
    eventId: string;
    initialKey?: string;
    reactions: NativeReactionReadback[];
  }) => void;
};

// Popout menus can unmount while a Core write is still running. Keep the
// event/action lock outside transient presenter state so reopening the menu
// cannot dispatch the same server mutation twice.
const nativeTimelineActionsInFlight = new Set<string>();
const nativePollFlights = new NativePollFlightCoordinator();
const nativeReactionFlights = new NativeReactionFlightCoordinator();
let nativeTimelineActionSessionGeneration: number | undefined;
const bindNativeTimelineActionSession = (sessionGeneration: number) => {
  if (nativeTimelineActionSessionGeneration === sessionGeneration) return;
  nativeTimelineActionSessionGeneration = sessionGeneration;
  nativeTimelineActionsInFlight.clear();
  nativePollFlights.bindSession(sessionGeneration);
  nativeReactionFlights.bindSession(sessionGeneration);
};
const nativeTimelineActionFlightKey = (
  sessionGeneration: number,
  roomId: string,
  eventId: string,
  action: string
) => `${sessionGeneration}\u0000${roomId}\u0000${eventId}\u0000${action}`;
const canonicalPollSelection = (answerIds: readonly string[]) =>
  [...answerIds].sort().join('\u0000');

const beginNativeTimelineAction = (sessionGeneration: number, key: string): boolean => {
  bindNativeTimelineActionSession(sessionGeneration);
  if (nativeTimelineActionsInFlight.has(key)) return false;
  nativeTimelineActionsInFlight.add(key);
  return true;
};

const isTimelineActionEditableTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.tagName === 'SELECT'
  );
};

const runNativeRowAction = (
  action: () => Promise<unknown>,
  onActionError: (message: string) => void,
  failureLabel: string,
  inFlightKey?: string,
  sessionGeneration?: number
) => {
  if (
    inFlightKey &&
    (sessionGeneration === undefined || !beginNativeTimelineAction(sessionGeneration, inFlightKey))
  ) {
    onActionError('That action is already in progress.');
    return;
  }
  void action()
    .catch((error) => {
      onActionError(error instanceof Error ? error.message : failureLabel);
    })
    .finally(() => {
      if (inFlightKey) nativeTimelineActionsInFlight.delete(inFlightKey);
    });
};

type NativeTimelineRowActionsProps = {
  sessionGeneration: number;
  roomId: string;
  eventId?: string;
  body?: string;
  formattedBody?: string;
  forwardTransport?: 'text' | 'media';
  capabilities?: NativeTimelineRowCapabilities;
  pinned?: boolean;
  sourceEncryptionStatus?: RoomEncryptionStatus;
  onActionError: (message: string) => void;
  /** Close the transient row menu after a completed one-shot action. */
  onRequestClose?: () => void;
  onViewReactions?: () => void;
  hasReactions?: boolean;
};

const NativeTimelineRowActions = ({
  sessionGeneration,
  roomId,
  eventId,
  body,
  formattedBody,
  forwardTransport,
  capabilities,
  pinned,
  sourceEncryptionStatus,
  onActionError,
  onRequestClose,
  onViewReactions,
  hasReactions,
}: NativeTimelineRowActionsProps) => {
  const roomList = useNativeRoomListSnapshot();
  const [editing, setEditing] = useState(false);
  const [reporting, setReporting] = useState(false);
  const [reportReason, setReportReason] = useState('');
  const [editBody, setEditBody] = useState(body ?? '');
  const [editFormattedBody, setEditFormattedBody] = useState(formattedBody ?? '');
  const [editFormattedBodyWasEdited, setEditFormattedBodyWasEdited] = useState(false);
  const [forwarding, setForwarding] = useState(false);
  const [forwardQuery, setForwardQuery] = useState('');
  const [forwardAsQuote, setForwardAsQuote] = useState(false);
  const [forwardConfirm, setForwardConfirm] = useState<{
    roomId: string;
    name?: string;
  } | null>(null);
  const [pendingProductAction, setPendingProductAction] = useState<
    'report' | 'forward' | undefined
  >();
  const pendingProductActionRef = useRef<'report' | 'forward' | undefined>(undefined);
  const actionsMountedRef = useRef(true);

  useEffect(() => {
    actionsMountedRef.current = true;
    return () => {
      actionsMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!editing) {
      setEditBody(body ?? '');
      setEditFormattedBody(formattedBody ?? '');
      setEditFormattedBodyWasEdited(false);
    }
  }, [body, editing, formattedBody]);

  const forwardTargets = useMemo(
    () =>
      filterNativeForwardTargets(
        roomList.rooms.map((room) => ({
          roomId: room.roomId,
          name: room.name,
          encryptionStatus: room.encryptionStatus,
          isSpace: room.isSpace,
        })),
        roomId,
        forwardQuery
      ),
    [forwardQuery, roomId, roomList.rooms]
  );

  if (!eventId || !capabilities) return null;
  const buttons: React.ReactNode[] = [];
  const moderationButtons: React.ReactNode[] = [];
  const closeAfterOneShotAction = () => onRequestClose?.();
  if (body) {
    buttons.push(
      <MenuItem
        key="copy"
        size="300"
        fill="None"
        radii="300"
        onClick={() => {
          copyRichTextToClipboard(
            body,
            formattedBody ? prepareNativeFormattedBody(formattedBody) : undefined
          );
          closeAfterOneShotAction();
        }}
      >
        Copy Message
      </MenuItem>
    );
  }
  if (hasReactions && onViewReactions) {
    buttons.push(
      <MenuItem
        key="view-reactions"
        size="300"
        fill="None"
        radii="300"
        after={<Icon size="100" src={Icons.Smile} />}
        onClick={() => {
          onViewReactions();
          closeAfterOneShotAction();
        }}
      >
        View Reactions
      </MenuItem>
    );
  }
  if (capabilities.reply) {
    buttons.push(
      <MenuItem
        key="reply"
        size="300"
        fill="None"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            async () => {
              const result = await setNativeComposerReplyDraft({ roomId, eventId });
              if (result === 'unavailable') {
                throw new Error('Native reply draft is unavailable.');
              }
            },
            onActionError,
            'Native reply draft failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        Reply
      </MenuItem>
    );
    buttons.push(
      <MenuItem
        key="reply-thread"
        size="300"
        fill="None"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            async () => {
              const result = await setNativeComposerReplyDraft({
                roomId,
                eventId,
                startThread: true,
              });
              if (result === 'unavailable') {
                throw new Error('Native thread reply draft is unavailable.');
              }
            },
            onActionError,
            'Native thread reply draft failed.'
          );
          closeAfterOneShotAction();
        }}
      >
        Reply in thread
      </MenuItem>
    );
  }
  if (capabilities.edit) {
    buttons.push(
      <MenuItem
        key="edit"
        size="300"
        fill="None"
        radii="300"
        onClick={() => {
          setForwarding(false);
          setEditing((open) => !open);
          setEditBody(body ?? '');
          setEditFormattedBody(formattedBody ?? '');
          setEditFormattedBodyWasEdited(false);
        }}
      >
        {editing ? 'Cancel edit' : 'Edit'}
      </MenuItem>
    );
  }
  if (capabilities.forward && isNativeTimelineForwardTransport(forwardTransport)) {
    buttons.push(
      <MenuItem
        key="forward"
        size="300"
        fill="None"
        radii="300"
        disabled={pendingProductAction !== undefined}
        onClick={() => {
          setEditing(false);
          setReporting(false);
          setForwarding((open) => !open);
          setForwardConfirm(null);
        }}
      >
        {forwarding ? 'Cancel forward' : 'Forward'}
      </MenuItem>
    );
  }
  if (capabilities.redact) {
    moderationButtons.push(
      <MenuItem
        key="redact"
        variant="Critical"
        size="300"
        fill="None"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            () => redactWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            'Native redact failed.',
            nativeTimelineActionFlightKey(sessionGeneration, roomId, eventId, 'redact'),
            sessionGeneration
          );
          closeAfterOneShotAction();
        }}
      >
        Redact
      </MenuItem>
    );
  }
  if (capabilities.report) {
    moderationButtons.push(
      <MenuItem
        key="report"
        variant="Critical"
        size="300"
        fill="None"
        radii="300"
        disabled={pendingProductAction !== undefined}
        onClick={() => {
          setEditing(false);
          setForwarding(false);
          setReporting((open) => !open);
        }}
      >
        {reporting ? 'Cancel report' : 'Report'}
      </MenuItem>
    );
  }
  if (capabilities.pin) {
    const pinAction = selectNativeTimelinePinAction(Boolean(pinned));
    buttons.push(
      <MenuItem
        key={pinAction}
        size="300"
        fill="None"
        radii="300"
        onClick={() => {
          runNativeRowAction(
            () =>
              pinAction === 'unpin'
                ? unpinWithNativeTimelineAction({ roomId, eventId })
                : pinWithNativeTimelineAction({ roomId, eventId }),
            onActionError,
            pinAction === 'unpin' ? 'Native unpin failed.' : 'Native pin failed.',
            nativeTimelineActionFlightKey(sessionGeneration, roomId, eventId, pinAction),
            sessionGeneration
          );
          closeAfterOneShotAction();
        }}
      >
        {pinAction === 'unpin' ? 'Unpin' : 'Pin'}
      </MenuItem>
    );
  }
  // Later is a room-event affordance for any remote timeline item with an id.
  buttons.push(
    <MenuItem
      key="later"
      size="300"
      fill="None"
      radii="300"
      onClick={() => {
        runNativeRowAction(
          () => upsertLaterWithNativeOwner(createLaterItemFromIds(roomId, eventId, 'saved')),
          onActionError,
          'Native later save failed.'
        );
        closeAfterOneShotAction();
      }}
    >
      Save for later
    </MenuItem>
  );
  if (buttons.length === 0) return null;

  const submitEdit = () => {
    const nextBody = editBody.trim();
    if (!nextBody) {
      onActionError('Edited body cannot be empty.');
      return;
    }
    const nextFormatted = editedFormattedBodyForSubmit(
      body ?? '',
      nextBody,
      editFormattedBody,
      editFormattedBodyWasEdited
    );
    runNativeRowAction(
      async () => {
        await editTextWithNativeTimelineAction({
          roomId,
          eventId,
          body: nextBody,
          formattedBody: nextFormatted,
        });
        setEditing(false);
        onRequestClose?.();
      },
      onActionError,
      'Native edit failed.',
      nativeTimelineActionFlightKey(sessionGeneration, roomId, eventId, 'edit'),
      sessionGeneration
    );
  };

  const submitReport = async () => {
    if (pendingProductActionRef.current !== undefined) return;
    const actionKey = nativeTimelineActionFlightKey(sessionGeneration, roomId, eventId, 'report');
    if (!beginNativeTimelineAction(sessionGeneration, actionKey)) {
      onActionError('That report is already in progress.');
      return;
    }
    pendingProductActionRef.current = 'report';
    setPendingProductAction('report');
    try {
      await reportWithNativeTimelineAction({
        roomId,
        eventId,
        reason: reportReason.trim() || undefined,
      });
      setReporting(false);
      setReportReason('');
      onRequestClose?.();
    } catch (error) {
      onActionError(error instanceof Error ? error.message : 'Native report failed.');
    } finally {
      nativeTimelineActionsInFlight.delete(actionKey);
      pendingProductActionRef.current = undefined;
      if (actionsMountedRef.current) setPendingProductAction(undefined);
    }
  };

  const forwardCanQuote = forwardTransport === 'text';

  const sendForward = async (targetRoomId: string, confirmedEncryptionDowngrade = false) => {
    if (pendingProductActionRef.current !== undefined) return;
    if (!isNativeTimelineForwardTransport(forwardTransport)) {
      onActionError('Native forward transport is unavailable.');
      return;
    }
    const actionKey = nativeTimelineActionFlightKey(sessionGeneration, roomId, eventId, 'forward');
    if (!beginNativeTimelineAction(sessionGeneration, actionKey)) {
      onActionError('That forward is already in progress.');
      return;
    }
    const useMedia = isNativeTimelineForwardMedia(forwardTransport);
    pendingProductActionRef.current = 'forward';
    setPendingProductAction('forward');
    try {
      if (useMedia) {
        await forwardMediaWithNativeTimelineAction({
          sourceRoomId: roomId,
          eventId,
          targetRoomId,
          confirmedEncryptionDowngrade,
        });
      } else {
        await forwardTextWithNativeTimelineAction({
          sourceRoomId: roomId,
          eventId,
          targetRoomId,
          asQuote: forwardCanQuote && forwardAsQuote,
          confirmedEncryptionDowngrade,
        });
      }
      setForwarding(false);
      setForwardQuery('');
      setForwardAsQuote(false);
      setForwardConfirm(null);
      onRequestClose?.();
    } catch (error) {
      onActionError(error instanceof Error ? error.message : 'Native forward failed.');
    } finally {
      nativeTimelineActionsInFlight.delete(actionKey);
      pendingProductActionRef.current = undefined;
      if (actionsMountedRef.current) setPendingProductAction(undefined);
    }
  };

  const requestForward = (target: {
    roomId: string;
    name?: string;
    encryptionStatus: RoomEncryptionStatus;
  }) => {
    if (pendingProductActionRef.current !== undefined) return;
    const decision = nativeForwardEncryptionDecision(
      sourceEncryptionStatus,
      target.encryptionStatus
    );
    if (decision === 'unavailable') {
      onActionError('Room encryption status is unavailable. Forwarding was not started.');
      return;
    }
    if (decision === 'confirm_downgrade') {
      setForwardConfirm({ roomId: target.roomId, name: target.name });
      return;
    }
    sendForward(target.roomId);
  };

  return (
    <Box direction="Column" gap="100" style={{ minWidth: 240, padding: config.space.S100 }}>
      <Box direction="Column" gap="100">
        {buttons}
      </Box>
      {moderationButtons.length > 0 && (
        <>
          <Line size="300" className={htmlCss.MessageActionDivider} />
          <Box direction="Column" gap="100" role="group" aria-label="Moderation actions">
            {moderationButtons}
          </Box>
        </>
      )}
      {editing && (
        <Box direction="Column" gap="100">
          <textarea
            value={editBody}
            onChange={(event) => setEditBody(event.target.value)}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
            aria-label="Edit message body"
          />
          <textarea
            value={editFormattedBody}
            onChange={(event) => {
              setEditFormattedBodyWasEdited(true);
              setEditFormattedBody(event.target.value);
            }}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
            aria-label="Edit message HTML body"
            placeholder="Optional Matrix HTML (org.matrix.custom.html)"
          />
          <Box gap="100">
            <Button size="300" onClick={submitEdit}>
              Save edit
            </Button>
          </Box>
        </Box>
      )}
      {reporting && (
        <Box direction="Column" gap="100">
          <textarea
            value={reportReason}
            onChange={(event) => setReportReason(event.target.value)}
            maxLength={512}
            rows={3}
            style={{ width: '100%', resize: 'vertical' }}
            aria-label="Optional report reason"
            placeholder="Reason (optional)"
            autoFocus
            disabled={pendingProductAction !== undefined}
          />
          <Text size="T200">This report is sent to your homeserver administrators.</Text>
          <Box gap="100">
            <Button
              size="300"
              variant="Critical"
              onClick={() => void submitReport()}
              disabled={pendingProductAction !== undefined}
            >
              {pendingProductAction === 'report' ? 'Reporting…' : 'Send report'}
            </Button>
          </Box>
        </Box>
      )}
      {forwarding && (
        <Box direction="Column" gap="100">
          {forwardConfirm ? (
            <Box direction="Column" gap="100">
              <Text size="T200">
                Forward from an encrypted room into {forwardConfirm.name || forwardConfirm.roomId}{' '}
                (not encrypted)? The forwarded copy will not be protected by room encryption.
              </Text>
              <Box gap="100">
                <Button
                  size="300"
                  onClick={() => void sendForward(forwardConfirm.roomId, true)}
                  disabled={pendingProductAction !== undefined}
                >
                  {pendingProductAction === 'forward' ? 'Forwarding…' : 'Forward anyway'}
                </Button>
                <Button
                  size="300"
                  fill="Soft"
                  onClick={() => setForwardConfirm(null)}
                  disabled={pendingProductAction !== undefined}
                >
                  Cancel
                </Button>
              </Box>
            </Box>
          ) : (
            <>
              <input
                value={forwardQuery}
                onChange={(event) => setForwardQuery(event.target.value)}
                placeholder="Filter rooms by name or id"
                style={{ width: '100%' }}
                aria-label="Forward target room filter"
                autoFocus
                disabled={pendingProductAction !== undefined}
              />
              {forwardCanQuote && (
                <label style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                  <input
                    type="checkbox"
                    checked={forwardAsQuote}
                    onChange={(event) => setForwardAsQuote(event.target.checked)}
                    disabled={pendingProductAction !== undefined}
                  />
                  <Text size="T200">Forward as quote</Text>
                </label>
              )}
              <Box direction="Column" gap="100" style={{ maxHeight: 180, overflow: 'auto' }}>
                {forwardTargets.length === 0 ? (
                  <Text size="T200">No matching rooms.</Text>
                ) : (
                  forwardTargets.slice(0, 40).map((target) => (
                    <Button
                      key={target.roomId}
                      size="300"
                      fill="Soft"
                      onClick={() => requestForward(target)}
                      disabled={pendingProductAction !== undefined}
                    >
                      {target.name || target.roomId}
                      {target.encryptionStatus === 'encrypted' ? ' · encrypted' : ''}
                      {target.encryptionStatus === 'unknown' ? ' · encryption unavailable' : ''}
                    </Button>
                  ))
                )}
              </Box>
            </>
          )}
        </Box>
      )}
    </Box>
  );
};

type NativeTimelineRowActionSurfaceProps = {
  children: React.ReactNode;
  actionProps: Omit<NativeTimelineRowActionsProps, 'onRequestClose'>;
  onReaction: (key: string) => void;
};

/**
 * The native presenter owns the action UI as well as the data/actions behind it.
 * Keep the legacy Message component out of this path: it requires the retired
 * Matrix event graph, while these controls consume only native DTO capabilities
 * and native command owners.
 */
const NativeTimelineRowActionSurface = ({
  children,
  actionProps,
  onReaction,
}: NativeTimelineRowActionSurfaceProps) => {
  const { eventId, capabilities } = actionProps;
  const [hovered, setHovered] = useState(false);
  const [focusWithin, setFocusWithin] = useState(false);
  const [emojiBoardAnchor, setEmojiBoardAnchor] = useState<RectCords>();
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const { hoverProps } = useHover({ onHoverChange: setHovered });
  const { focusWithinProps } = useFocusWithin({ onFocusWithinChange: setFocusWithin });
  const hasActionMenu = Boolean(eventId && capabilities);
  const menuOpen = Boolean(menuAnchor);
  const emojiBoardOpen = Boolean(emojiBoardAnchor);
  const actionsActive = hovered || focusWithin || menuOpen || emojiBoardOpen;
  const showActionRail = hasActionMenu && actionsActive;

  const closeMenu = () => setMenuAnchor(undefined);
  const openMenu = (event: React.MouseEvent<HTMLButtonElement>) => {
    setEmojiBoardAnchor(undefined);
    setMenuAnchor(event.currentTarget.getBoundingClientRect());
  };
  const openEmojiBoard = (event: React.MouseEvent<HTMLButtonElement>) => {
    setMenuAnchor(undefined);
    const target = event.currentTarget.parentElement?.parentElement ?? event.currentTarget;
    setEmojiBoardAnchor(target.getBoundingClientRect());
  };

  return (
    <div
      {...(hasActionMenu ? hoverProps : {})}
      {...(hasActionMenu ? focusWithinProps : {})}
      tabIndex={hasActionMenu ? 0 : undefined}
      role={hasActionMenu ? 'group' : undefined}
      aria-label={hasActionMenu ? 'Message actions' : undefined}
      className={htmlCss.MessageActionSurface}
    >
      {showActionRail && (
        <div data-native-timeline-action-rail="true" className={htmlCss.MessageActionRail}>
          <Menu variant="SurfaceVariant" style={{ padding: config.space.S100 }}>
            <Box gap="100">
              {capabilities?.react && (
                <PopOut
                  anchor={emojiBoardAnchor}
                  position="Bottom"
                  align="End"
                  content={
                    <EmojiBoard
                      imagePackRooms={[actionProps.roomId]}
                      returnFocusOnDeactivate={false}
                      addToRecentEmoji={false}
                      onEmojiSelect={(unicode) => {
                        onReaction(unicode);
                        setEmojiBoardAnchor(undefined);
                      }}
                      onCustomEmojiSelect={(mxc) => {
                        onReaction(mxc);
                        setEmojiBoardAnchor(undefined);
                      }}
                      requestClose={() => setEmojiBoardAnchor(undefined)}
                    />
                  }
                >
                  <IconButton
                    variant={actionsActive ? 'Surface' : 'SurfaceVariant'}
                    size="300"
                    radii="300"
                    title="Add reaction"
                    aria-label="Add reaction"
                    aria-pressed={emojiBoardOpen}
                    onClick={openEmojiBoard}
                  >
                    <Icon src={Icons.SmilePlus} size="100" />
                  </IconButton>
                </PopOut>
              )}
              <PopOut
                anchor={menuAnchor}
                position="Bottom"
                align="End"
                content={
                  <FocusTrap
                    focusTrapOptions={{
                      initialFocus: false,
                      onDeactivate: closeMenu,
                      clickOutsideDeactivates: true,
                      isKeyForward: (event: KeyboardEvent) =>
                        event.key === 'ArrowDown' && !isTimelineActionEditableTarget(event.target),
                      isKeyBackward: (event: KeyboardEvent) =>
                        event.key === 'ArrowUp' && !isTimelineActionEditableTarget(event.target),
                      escapeDeactivates: stopPropagation,
                    }}
                  >
                    <Menu
                      className={htmlCss.MessageActionMenu}
                      data-native-timeline-action-menu="true"
                    >
                      <NativeTimelineRowActions {...actionProps} onRequestClose={closeMenu} />
                    </Menu>
                  </FocusTrap>
                }
              >
                <IconButton
                  variant={actionsActive ? 'Surface' : 'SurfaceVariant'}
                  size="300"
                  radii="300"
                  title="More message actions"
                  aria-label="More message actions"
                  aria-haspopup="menu"
                  aria-expanded={menuOpen}
                  onClick={openMenu}
                >
                  <Icon src={Icons.VerticalDots} size="100" />
                </IconButton>
              </PopOut>
            </Box>
          </Menu>
        </div>
      )}
      {children}
    </div>
  );
};

const NativeTimelineMedia = ({
  media,
  messageType,
  filename,
  caption,
  formattedCaption,
  sticker,
}: {
  media?: NativeTimelineMediaHandle;
  messageType?: string;
  filename?: string;
  caption?: string;
  formattedCaption?: string;
  sticker?: boolean;
}) => {
  const mediaSrc = media ? nativeTimelineMediaSrc(media) : undefined;
  const reservedBox = sticker ? NATIVE_TIMELINE_STICKER_MAX_PX : NATIVE_TIMELINE_MEDIA_MAX_PX;
  if (!mediaSrc) {
    if (sticker) return <Text size="T300">Sticker media is unavailable.</Text>;
    const reserved = reservedNativeTimelineMediaSize(
      media?.width,
      media?.height,
      media?.width ? Math.min(media.width, reservedBox) : reservedBox,
      media?.height ? Math.min(media.height, reservedBox) : reservedBox
    );
    if (reserved && (messageType === 'image' || messageType === 'video')) {
      return <div style={{ width: reserved.width, height: reserved.height }} aria-hidden />;
    }
    return null;
  }
  if (sticker) {
    return <img src={mediaSrc} alt="Sticker" style={mediaStyle(media, reservedBox)} />;
  }
  const captionView = caption ? (
    formattedCaption ? (
      <NativeFormattedBody html={formattedCaption} fallbackBody={caption} />
    ) : (
      <Text size="T300" style={{ whiteSpace: 'pre-wrap', lineHeight: 1.55 }}>
        {caption}
      </Text>
    )
  ) : null;
  if (messageType === 'image') {
    return (
      <Box direction="Column" gap="100">
        <img src={mediaSrc} alt={caption || filename || 'Image'} style={mediaStyle(media)} />
        {captionView}
      </Box>
    );
  }
  if (messageType === 'audio') {
    return (
      <Box direction="Column" gap="100">
        {/* Matrix media metadata does not provide a captions track. */}
        {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
        <audio
          src={mediaSrc}
          controls
          {...(media?.durationMs ? { 'data-duration-ms': String(media.durationMs) } : {})}
        />
        {captionView}
      </Box>
    );
  }
  if (messageType === 'video') {
    return (
      <Box direction="Column" gap="100">
        {/* Matrix media metadata does not provide a captions track. */}
        {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
        <video
          src={mediaSrc}
          controls
          style={mediaStyle(media)}
          {...(media?.durationMs ? { 'data-duration-ms': String(media.durationMs) } : {})}
        />
        {captionView}
      </Box>
    );
  }
  if (messageType === 'file') {
    return (
      <Box direction="Column" gap="100">
        <a href={mediaSrc} download>
          {filename || 'Download file'}
        </a>
        {captionView}
        {media?.mimeType ? (
          <Text size="T200" className={htmlCss.Metadata}>
            {media.mimeType}
          </Text>
        ) : null}
      </Box>
    );
  }
  return null;
};

const NativeTimelineSenderAvatar = ({ row }: { row: NativeTimelineViewRow }) => {
  const senderId = rowSenderId(row) ?? '';
  const displayName = displayNameForRow(row);
  return (
    <Avatar className={htmlCss.TimelineAvatar} size="300" radii="400">
      <UserAvatar
        userId={senderId}
        src={rowSenderAvatarUrl(row)}
        alt={displayName}
        renderFallback={() => (
          <Text as="span" size="T200" style={{ textTransform: 'uppercase' }}>
            {nameInitials(displayName)}
          </Text>
        )}
      />
    </Avatar>
  );
};

const NativeTimelineReplySurface = ({
  reply,
  onFocusEvent,
}: {
  reply?: NativeTimelineReplyPreview;
  onFocusEvent: (eventId: string) => void;
}) =>
  reply ? (
    <Box
      as="button"
      direction="Column"
      gap="100"
      onClick={() => onFocusEvent(reply.eventId)}
      className={htmlCss.ReplySurface}
      aria-label={`Open message from ${reply.senderName}`}
    >
      <Text size="T300" className={htmlCss.SenderName}>
        Replying to {reply.senderName}
      </Text>
      <Text size="T300" style={{ whiteSpace: 'pre-wrap' }}>
        {reply.body}
      </Text>
    </Box>
  ) : null;

const NativeTimelineThreadSurface = ({
  threadRoot,
  thread,
  onFocusEvent,
}: {
  threadRoot?: string;
  thread?: NativeTimelineThreadSummary;
  onFocusEvent: (eventId: string) => void;
}) => {
  const focusEventId = nativeThreadFocusEventId(thread) ?? threadRoot;
  if (!focusEventId) return null;
  return (
    <Button size="300" fill="Soft" onClick={() => onFocusEvent(focusEventId)}>
      {thread ? (
        <>
          Thread · {thread.replyCount} {thread.replyCount === 1 ? 'reply' : 'replies'}
          {thread.latestEventId ? ' · open latest' : ' · open root'}
        </>
      ) : (
        'Thread reply · open thread'
      )}
    </Button>
  );
};

const NativeTimelineReactionPills = ({
  reactions,
  enabled,
  onReaction,
  onViewReactions,
}: {
  reactions?: NativeTimelineReaction[];
  enabled: boolean;
  onReaction: (key: string) => void;
  onViewReactions?: (key: string) => void;
}) =>
  reactions?.length ? (
    <Box gap="100" wrap="Wrap">
      {reactions.map((reaction) => (
        <Button
          key={reaction.key}
          size="300"
          variant={reaction.own ? 'Primary' : 'Secondary'}
          fill="Soft"
          disabled={!enabled}
          onClick={() => enabled && onReaction(reaction.key)}
          onContextMenu={(event) => {
            if (!onViewReactions) return;
            event.preventDefault();
            event.stopPropagation();
            onViewReactions(reaction.key);
          }}
        >
          {reaction.key} {reaction.count}
        </Button>
      ))}
    </Box>
  ) : null;

const NativeTimelinePollAnswers = ({
  answers,
  maximumSelections,
  canVote,
  closed,
  onVote,
}: {
  answers: NativeTimelinePollAnswer[];
  maximumSelections: number;
  canVote: boolean;
  closed: boolean;
  onVote: (answerIds: string[]) => Promise<boolean>;
}) => {
  const original = useMemo(
    () => new Set(answers.filter((answer) => answer.own).map((answer) => answer.id)),
    [answers]
  );
  const available = useMemo(() => new Set(answers.map((answer) => answer.id)), [answers]);
  const [selection, setSelection] = useState<Set<string>>(original);
  const [submitting, setSubmitting] = useState(false);
  const submittingRef = useRef(false);
  const pendingSelectionRef = useRef<string | undefined>(undefined);
  const pollDispatchSettledRef = useRef(false);
  const pollProjectionObservedRef = useRef(false);
  const pollMountedRef = useRef(true);

  useEffect(() => setSelection(original), [original]);
  useEffect(() => {
    if (
      pendingSelectionRef.current !== undefined &&
      pendingSelectionRef.current === canonicalPollSelection([...original])
    ) {
      pollProjectionObservedRef.current = true;
      if (pollDispatchSettledRef.current) {
        pendingSelectionRef.current = undefined;
        pollDispatchSettledRef.current = false;
        pollProjectionObservedRef.current = false;
        submittingRef.current = false;
        setSubmitting(false);
      }
    }
  }, [original]);
  useEffect(() => {
    pollMountedRef.current = true;
    return () => {
      pollMountedRef.current = false;
    };
  }, []);

  const submit = async (answerIds: string[]) => {
    if (submittingRef.current) return;
    submittingRef.current = true;
    setSubmitting(true);
    pendingSelectionRef.current = canonicalPollSelection(answerIds);
    pollDispatchSettledRef.current = false;
    pollProjectionObservedRef.current = false;
    const accepted = await onVote(answerIds);
    if (accepted) {
      // Action readback proves the send, while the poll projection remains
      // the authority for selected answers. Keep the controls locked until
      // that exact state arrives rather than enabling a stale second vote.
      pollDispatchSettledRef.current = true;
      if (pollProjectionObservedRef.current) {
        pendingSelectionRef.current = undefined;
        pollDispatchSettledRef.current = false;
        pollProjectionObservedRef.current = false;
        submittingRef.current = false;
        if (pollMountedRef.current) setSubmitting(false);
      }
      return;
    }
    pendingSelectionRef.current = undefined;
    pollDispatchSettledRef.current = false;
    pollProjectionObservedRef.current = false;
    submittingRef.current = false;
    if (pollMountedRef.current) setSubmitting(false);
  };
  const submission = nativePollSubmission(
    selection,
    original,
    available,
    maximumSelections,
    canVote,
    closed
  );

  return (
    <Box direction="Column" gap="100">
      {answers.map((answer) => {
        const selected = selection.has(answer.id);
        return (
          <Button
            key={answer.id}
            size="300"
            variant={selected ? 'Primary' : 'Secondary'}
            fill="Soft"
            disabled={!canVote || closed || submitting}
            aria-pressed={selected}
            onClick={() => {
              const next = toggleNativePollSelection(
                selection,
                answer.id,
                available,
                maximumSelections
              );
              setSelection(next);
              if (maximumSelections === 1) {
                const immediate = nativePollSubmission(
                  next,
                  original,
                  available,
                  maximumSelections,
                  canVote,
                  closed
                );
                if (immediate) void submit(immediate);
              }
            }}
          >
            {answer.text} ({answer.voteCount})
          </Button>
        );
      })}
      {maximumSelections > 1 && submission ? (
        <Button
          size="300"
          variant="Primary"
          fill="Solid"
          disabled={submitting}
          onClick={() => void submit(submission)}
        >
          {submission.length === 0 ? 'Clear vote' : 'Submit vote'}
        </Button>
      ) : null}
    </Box>
  );
};

const NativeTimelineRow = ({
  row,
  grouped,
  groupsNext,
  roomId,
  sessionGeneration,
  messageSpacing,
  pinnedEventIds,
  sourceEncryptionStatus,
  onActionError,
  onFocusEvent,
  onViewReactions,
}: NativeTimelineRowProps) => {
  const [groupedTimestampOffset, setGroupedTimestampOffset] = useState(0);
  const [declinePending, setDeclinePending] = useState(false);
  const declinePendingRef = useRef(false);
  const rowMountedRef = useRef(true);
  const wheelResetTimer = useRef<number | undefined>(undefined);
  const nonMousePan = useRef<
    | {
        pointerId: number;
        startX: number;
        startY: number;
        active: boolean;
      }
    | undefined
  >(undefined);
  const [hour24Clock] = useSetting(settingsAtom, 'hour24Clock');
  const [dateFormatString] = useSetting(settingsAtom, 'dateFormatString');
  const surface = hasMessageSurface(row.kind);
  const rowClassName = htmlCss.MessageRow({
    surface,
    grouped: surface && grouped,
    groupsNext: surface && groupsNext,
  });
  const spacingToken =
    groupsNext || messageSpacing === '0'
      ? undefined
      : config.space[`S${messageSpacing}` as 'S100' | 'S200' | 'S300' | 'S400' | 'S500'];
  const rowStyle = spacingToken ? { marginBottom: spacingToken } : undefined;
  const capabilities = rowCapabilities(row);
  // Hermes treats its seeded terminal reaction keys as approval decisions.
  // For a Core-classified approval prompt those reactions must therefore flow
  // only through `decideAgentApprovalWithNativeOwner`; the ordinary emoji rail
  // and aggregate pills would otherwise bypass Core eligibility/expiry checks.
  const approvalOwnsReactionActions = row.kind === 'message' && Boolean(row.isAgentApproval);
  const genericReactionCapabilities =
    approvalOwnsReactionActions && capabilities ? { ...capabilities, react: false } : capabilities;
  const eventId = rowEventId(row);
  const originServerTs = rowOriginServerTs(row);
  const pinned = isNativeTimelineEventPinned(pinnedEventIds, eventId);
  const groupedTimestampRevealWidth = 72;
  const clampGroupedTimestampOffset = (offset: number) =>
    Math.max(-groupedTimestampRevealWidth, Math.min(0, offset));
  const scheduleGroupedTimestampReset = (delay = 600) => {
    if (wheelResetTimer.current !== undefined) window.clearTimeout(wheelResetTimer.current);
    wheelResetTimer.current = window.setTimeout(() => setGroupedTimestampOffset(0), delay);
  };
  const finishNonMousePan = (event: React.PointerEvent<HTMLDivElement>, cancelled = false) => {
    const pan = nonMousePan.current;
    if (!pan || pan.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    nonMousePan.current = undefined;
    if (cancelled || !pan.active) setGroupedTimestampOffset(0);
    else scheduleGroupedTimestampReset(900);
  };
  useEffect(() => {
    rowMountedRef.current = true;
    return () => {
      rowMountedRef.current = false;
      if (wheelResetTimer.current !== undefined) window.clearTimeout(wheelResetTimer.current);
    };
  }, []);
  const groupedTimestampRevealProps = grouped
    ? {
        onPointerDown: (event: React.PointerEvent<HTMLDivElement>) => {
          if (!event.isPrimary || event.pointerType === 'mouse') return;
          nonMousePan.current = {
            pointerId: event.pointerId,
            startX: event.clientX,
            startY: event.clientY,
            active: false,
          };
        },
        onPointerMove: (event: React.PointerEvent<HTMLDivElement>) => {
          const pan = nonMousePan.current;
          if (!pan || pan.pointerId !== event.pointerId) return;
          const deltaX = event.clientX - pan.startX;
          const deltaY = event.clientY - pan.startY;
          if (!pan.active) {
            if (Math.abs(deltaX) < 8) return;
            if (Math.abs(deltaX) <= Math.abs(deltaY) * 1.2) {
              nonMousePan.current = undefined;
              return;
            }
            pan.active = true;
            event.currentTarget.setPointerCapture(event.pointerId);
          }
          event.preventDefault();
          setGroupedTimestampOffset(clampGroupedTimestampOffset(deltaX));
        },
        onPointerUp: (event: React.PointerEvent<HTMLDivElement>) => finishNonMousePan(event),
        onPointerCancel: (event: React.PointerEvent<HTMLDivElement>) =>
          finishNonMousePan(event, true),
        onWheel: (event: React.WheelEvent<HTMLDivElement>) => {
          const deltaScale =
            event.deltaMode === WheelEvent.DOM_DELTA_LINE
              ? 16
              : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
              ? groupedTimestampRevealWidth
              : 1;
          const deltaX = event.deltaX * deltaScale;
          const deltaY = event.deltaY * deltaScale;
          if (Math.abs(deltaX) <= Math.abs(deltaY)) return;
          setGroupedTimestampOffset((current) => clampGroupedTimestampOffset(current - deltaX));
          scheduleGroupedTimestampReset();
        },
      }
    : {};
  const runReaction = (key: string) => {
    if (!eventId || !genericReactionCapabilities?.react) return;
    const reactions =
      row.kind === 'sticker' ? row.reactions ?? [] : 'reactions' in row ? row.reactions ?? [] : [];
    const projected = reactions.find((reaction) => reaction.key === key);
    if (projected !== undefined && projected.own === undefined) {
      onActionError('Reaction ownership is unavailable.');
      return;
    }
    const expectedOwn = !(projected?.own ?? false);
    const actionKey = nativeTimelineActionFlightKey(
      sessionGeneration,
      roomId,
      eventId,
      `reaction:${key}`
    );
    if (!beginNativeTimelineAction(sessionGeneration, actionKey)) {
      onActionError('That reaction is already in progress.');
      return;
    }
    nativeReactionFlights.prepare(actionKey, key, expectedOwn);
    void toggleReactionWithNativeOwner({ roomId, eventId, key, expectedOwn })
      .then(() => {
        if (nativeReactionFlights.settleDispatch(actionKey, true)) {
          nativeTimelineActionsInFlight.delete(actionKey);
        }
      })
      .catch((error) => {
        onActionError(error instanceof Error ? error.message : 'Native reaction failed.');
        nativeReactionFlights.settleDispatch(actionKey, false);
        nativeTimelineActionsInFlight.delete(actionKey);
      });
  };
  const runDecline = async () => {
    if (!eventId || !capabilities?.declineCall || declinePendingRef.current) return;
    const actionKey = nativeTimelineActionFlightKey(
      sessionGeneration,
      roomId,
      eventId,
      'call_decline'
    );
    if (!beginNativeTimelineAction(sessionGeneration, actionKey)) {
      onActionError('That call decline is already in progress.');
      return;
    }
    declinePendingRef.current = true;
    setDeclinePending(true);
    try {
      const result = await callDeclineWithNativeTimelineOwner(
        { roomId, eventId },
        isSynaraDesktop(),
        invokeDesktopWithAvailability
      );
      if (result === 'unavailable') onActionError('Native call decline is unavailable.');
    } catch (error) {
      onActionError(error instanceof Error ? error.message : 'Native call decline failed.');
    } finally {
      nativeTimelineActionsInFlight.delete(actionKey);
      declinePendingRef.current = false;
      if (rowMountedRef.current) setDeclinePending(false);
    }
  };
  const runPollVote = async (answerIds: string[]): Promise<boolean> => {
    if (!eventId || !capabilities?.vote) return false;
    const actionKey = nativeTimelineActionFlightKey(
      sessionGeneration,
      roomId,
      eventId,
      'poll_vote'
    );
    if (!beginNativeTimelineAction(sessionGeneration, actionKey)) {
      onActionError('That poll vote is already in progress.');
      return false;
    }
    nativePollFlights.bindSession(sessionGeneration);
    nativePollFlights.prepare(actionKey, answerIds);
    try {
      await pollVoteWithNativeTimelineAction({ roomId, eventId, answerIds });
      if (nativePollFlights.settleDispatch(actionKey, true)) {
        nativeTimelineActionsInFlight.delete(actionKey);
      }
      return true;
    } catch (error) {
      onActionError(error instanceof Error ? error.message : 'Native poll vote failed.');
      nativePollFlights.settleDispatch(actionKey, false);
      nativeTimelineActionsInFlight.delete(actionKey);
      return false;
    }
  };
  useEffect(() => {
    if (row.kind !== 'poll' || !eventId) return;
    const actionKey = nativeTimelineActionFlightKey(
      sessionGeneration,
      roomId,
      eventId,
      'poll_vote'
    );
    if (
      nativePollFlights.observeProjection(
        actionKey,
        (row.answers ?? []).filter((answer) => answer.own).map((answer) => answer.id)
      )
    ) {
      nativeTimelineActionsInFlight.delete(actionKey);
    }
  }, [eventId, roomId, row, sessionGeneration]);
  useEffect(() => {
    if (!eventId) return;
    const reactions =
      row.kind === 'sticker' ? row.reactions ?? [] : 'reactions' in row ? row.reactions ?? [] : [];
    const actionPrefix = nativeTimelineActionFlightKey(
      sessionGeneration,
      roomId,
      eventId,
      'reaction:'
    );
    for (const completed of nativeReactionFlights.observeEventProjection(actionPrefix, reactions)) {
      nativeTimelineActionsInFlight.delete(completed);
    }
  }, [eventId, roomId, row, sessionGeneration]);
  const rowViewReactions = nativeReactionsForViewer(
    'reactions' in row ? row.reactions : undefined
  );
  const openReactionViewer = (initialKey?: string) => {
    if (!eventId) return;
    onViewReactions({
      eventId,
      initialKey,
      reactions: rowViewReactions,
    });
  };

  switch (row.kind) {
    case 'message': {
      const isEmote = row.messageType === 'emote';
      const agentPayload = parseNativeTimelineAgentCard(row.agentCardJson);
      const approvalPrompt = row.isAgentApproval
        ? formatCoreAgentApprovalPrompt(row.body)
        : undefined;
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            body: row.body,
            formattedBody: row.formattedBody,
            forwardTransport: row.forwardTransport,
            capabilities: genericReactionCapabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <div
            {...groupedTimestampRevealProps}
            className={htmlCss.MessageSwipeSurface}
            title={
              grouped && originServerTs ? new Date(originServerTs).toLocaleString() : undefined
            }
          >
            {grouped && originServerTs ? (
              <div
                className={htmlCss.GroupedTimestampReveal}
                style={{ opacity: Math.min(1, Math.abs(groupedTimestampOffset) / 36) }}
                aria-hidden={groupedTimestampOffset === 0}
              >
                <Time
                  compact
                  ts={originServerTs}
                  hour24Clock={hour24Clock}
                  dateFormatString={dateFormatString}
                />
              </div>
            ) : null}
            <Box
              direction="Column"
              gap="100"
              className={`${rowClassName} ${htmlCss.MessageSwipeContent}`}
              style={{ ...rowStyle, transform: `translateX(${groupedTimestampOffset}px)` }}
            >
              <Box gap="300" alignItems="Start">
                <Box direction="Column" alignItems="Center" style={{ width: 36, flexShrink: 0 }}>
                  {grouped ? null : <NativeTimelineSenderAvatar row={row} />}
                </Box>
                <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0 }}>
                  {grouped ? null : (
                    <Box gap="200" alignItems="Baseline" className={htmlCss.Metadata}>
                      <Text size="T300" className={htmlCss.SenderName}>
                        {displayNameForRow(row)}
                      </Text>
                      {originServerTs ? (
                        <Time
                          ts={originServerTs}
                          hour24Clock={hour24Clock}
                          dateFormatString={dateFormatString}
                        />
                      ) : null}
                      {pinned ? (
                        <Text size="T200" className={htmlCss.Metadata}>
                          Pinned
                        </Text>
                      ) : null}
                    </Box>
                  )}
                  <NativeTimelineReplySurface reply={row.reply} onFocusEvent={onFocusEvent} />
                  {approvalPrompt && eventId ? (
                    <AgentApprovalCard
                      prompt={approvalPrompt}
                      target={{
                        roomId,
                        eventId,
                        canSendReaction: capabilities?.react,
                        coreEligible: true,
                      }}
                    />
                  ) : agentPayload ? (
                    <ErrorBoundary fallback={<Text size="T300">Agent output unavailable</Text>}>
                      <React.Suspense
                        fallback={<Spinner size="200" aria-label="Loading agent output" />}
                      >
                        <HermesAgentCard payload={agentPayload} />
                      </React.Suspense>
                    </ErrorBoundary>
                  ) : row.media ? null : (
                    <div className={htmlCss.MessageBody}>
                      {row.formattedBody ? (
                        <NativeFormattedBody
                          html={row.formattedBody}
                          fallbackBody={row.body}
                          style={{
                            fontStyle: isEmote ? 'italic' : undefined,
                          }}
                        />
                      ) : (
                        <Text
                          size="T300"
                          style={{
                            whiteSpace: 'pre-wrap',
                            fontWeight: 400,
                            lineHeight: 1.55,
                            fontStyle: isEmote ? 'italic' : undefined,
                          }}
                        >
                          {isEmote ? `* ${row.body}` : row.body}
                        </Text>
                      )}
                      {row.edited ? (
                        <Text size="T200" className={htmlCss.Metadata}>
                          Edited
                        </Text>
                      ) : null}
                    </div>
                  )}
                  <NativeTimelineThreadSurface
                    threadRoot={row.threadRoot}
                    thread={row.thread}
                    onFocusEvent={onFocusEvent}
                  />
                  <NativeTimelineMedia
                    media={row.media}
                    messageType={row.messageType}
                    filename={row.mediaFilename}
                    caption={row.mediaCaption}
                    formattedCaption={row.formattedBody}
                  />
                  <NativeTimelineReactionPills
                    reactions={row.reactions}
                    enabled={Boolean(genericReactionCapabilities?.react)}
                    onReaction={runReaction}
                    onViewReactions={openReactionViewer}
                  />
                </Box>
              </Box>
            </Box>
          </div>
        </NativeTimelineRowActionSurface>
      );
    }
    case 'membership':
    case 'state':
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box className={`${rowClassName} ${htmlCss.SystemRow}`}>
            <Text size="T300">{row.summary}</Text>
          </Box>
        </NativeTimelineRowActionSurface>
      );
    case 'poll':
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box direction="Column" gap="100" className={rowClassName}>
            {originServerTs ? (
              <Time
                ts={originServerTs}
                hour24Clock={hour24Clock}
                dateFormatString={dateFormatString}
              />
            ) : null}
            <NativeTimelineReplySurface reply={row.reply} onFocusEvent={onFocusEvent} />
            <Text size="L400">{row.question}</Text>
            <Text size="T300">{row.closed ? 'Poll closed' : 'Poll open'}</Text>
            <NativeTimelinePollAnswers
              answers={row.answers ?? []}
              maximumSelections={Math.max(0, row.maxSelections ?? 1)}
              canVote={Boolean(capabilities?.vote)}
              closed={row.closed}
              onVote={runPollVote}
            />
            <NativeTimelineThreadSurface
              threadRoot={row.threadRoot}
              thread={row.thread}
              onFocusEvent={onFocusEvent}
            />
            <NativeTimelineReactionPills
              reactions={row.reactions}
              enabled={Boolean(genericReactionCapabilities?.react)}
              onReaction={runReaction}
              onViewReactions={openReactionViewer}
            />
          </Box>
        </NativeTimelineRowActionSurface>
      );
    case 'call':
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box direction="Column" gap="100" className={rowClassName}>
            <Text size="T300">{row.callKind}</Text>
            {capabilities?.declineCall && (
              <Button
                size="300"
                fill="Soft"
                disabled={declinePending}
                onClick={() => void runDecline()}
                aria-label="Decline call"
              >
                {declinePending ? 'Declining…' : 'Decline call'}
              </Button>
            )}
          </Box>
        </NativeTimelineRowActionSurface>
      );
    case 'date_separator':
      return row.timestampMs && Number.isFinite(row.timestampMs) ? (
        <Box className={`${rowClassName} ${htmlCss.SystemRow}`}>
          <span className={htmlCss.SystemRule} aria-hidden="true" />
          <Text size="T300">{new Date(row.timestampMs).toLocaleDateString()}</Text>
          <span className={htmlCss.SystemRule} aria-hidden="true" />
        </Box>
      ) : null;
    case 'read_marker':
      return (
        <Box className={`${rowClassName} ${htmlCss.SystemRow}`}>
          <span className={htmlCss.SystemRule} aria-hidden="true" />
          <Text size="T300">Read up to here</Text>
          <span className={htmlCss.SystemRule} aria-hidden="true" />
        </Box>
      );
    case 'unread_marker':
      return (
        <Box className={`${rowClassName} ${htmlCss.SystemRow} ${htmlCss.UnreadSystemRow}`}>
          <span className={htmlCss.SystemRule} aria-hidden="true" />
          <Text size="T300">New messages</Text>
          <span className={htmlCss.SystemRule} aria-hidden="true" />
        </Box>
      );
    case 'timeline_start':
      return (
        <Box className={rowClassName}>
          <Text size="T300">Beginning of timeline</Text>
        </Box>
      );
    case 'redacted':
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box className={rowClassName}>
            <Text size="T300">{row.summary ?? 'Message removed'}</Text>
          </Box>
        </NativeTimelineRowActionSurface>
      );
    case 'encrypted_unavailable':
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box className={rowClassName}>
            <Text size="T300">This encrypted message is not available on this device.</Text>
          </Box>
        </NativeTimelineRowActionSurface>
      );
    case 'other':
      return row.summary ? (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            forwardTransport: row.forwardTransport,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box className={rowClassName}>
            <Text size="T300">{row.summary}</Text>
          </Box>
        </NativeTimelineRowActionSurface>
      ) : null;
    case 'sticker': {
      return (
        <NativeTimelineRowActionSurface
          actionProps={{
            sessionGeneration,
            roomId,
            eventId,
            forwardTransport: row.forwardTransport,
            capabilities,
            pinned,
            sourceEncryptionStatus,
            onActionError,
            hasReactions: rowViewReactions.length > 0,
            onViewReactions:
              rowViewReactions.length > 0 ? () => openReactionViewer() : undefined,
          }}
          onReaction={runReaction}
        >
          <Box direction="Column" gap="100" className={rowClassName}>
            <Box gap="200" alignItems="Baseline" className={htmlCss.Metadata}>
              <Text size="T300" className={htmlCss.SenderName}>
                {row.event.senderName}
              </Text>
              {originServerTs ? (
                <Time
                  ts={originServerTs}
                  hour24Clock={hour24Clock}
                  dateFormatString={dateFormatString}
                />
              ) : null}
              {pinned ? (
                <Text size="T200" className={htmlCss.Metadata}>
                  Pinned
                </Text>
              ) : null}
            </Box>
            <NativeTimelineReplySurface reply={row.reply} onFocusEvent={onFocusEvent} />
            <NativeTimelineMedia media={row.media} sticker />
            <NativeTimelineThreadSurface
              threadRoot={row.threadRoot}
              thread={row.thread}
              onFocusEvent={onFocusEvent}
            />
            <NativeTimelineReactionPills
              reactions={row.reactions}
              enabled={Boolean(genericReactionCapabilities?.react)}
              onReaction={runReaction}
              onViewReactions={openReactionViewer}
            />
          </Box>
        </NativeTimelineRowActionSurface>
      );
    }
    case 'pagination':
      return row.state === 'loading' ? (
        <Box className={rowClassName}>
          <Box justifyContent="Center">
            <Spinner size="200" aria-label="Loading messages" />
          </Box>
        </Box>
      ) : null;
    default:
      return null;
  }
};

/**
 * SDK-neutral, virtualized presentation of the native timeline DTO.
 * Active owner mounted by RoomView after V-TIMELINE.C1; JS RoomTimeline deleted
 * in V-TIMELINE.C2 (dual_backend false).
 */
export function NativeTimelinePresenter({ roomId, eventId }: NativeTimelinePresenterProps) {
  const [focusEventId, setFocusEventId] = useState(eventId);
  useEffect(() => {
    setFocusEventId(eventId);
  }, [eventId, roomId]);

  const openingViewport = useMemo(
    () => (focusEventId ? undefined : nativeTimelineViewports.get(roomId)),
    [focusEventId, roomId]
  );
  const input = useMemo(
    () => ({
      roomId,
      position: focusEventId
        ? ({ kind: 'focused', eventId: focusEventId } as const)
        : ({
            kind: 'normal',
            restoredAnchorEventId: openingViewport?.atBottom
              ? undefined
              : openingViewport?.anchor?.eventId,
          } as const),
    }),
    [focusEventId, openingViewport, roomId]
  );
  const controller = useNativeTimelineView(input);
  const { setReadState, followLive, jumpLatest: loadLatest, restoreLastRead } = controller;
  const timelineState = controller.state;
  const readyState = timelineState.status === 'ready' ? timelineState : undefined;
  const activeSessionGeneration = readyState?.snapshot.sessionGeneration;
  const [actionError, setActionError] = useState<string>();
  const [atLiveBottom, setAtLiveBottom] = useState(false);
  const [pendingLastRead, setPendingLastRead] = useState<string>();
  const [latestPlacementRequest, setLatestPlacementRequest] = useState(0);
  const appliedLatestPlacementRef = useRef(0);
  const [lastReadPlacementRequest, setLastReadPlacementRequest] = useState(0);
  const appliedLastReadPlacementRef = useRef(0);
  const mountedRoomRef = useRef(roomId);
  mountedRoomRef.current = roomId;
  const mountedNavigationRef = useRef(input);
  mountedNavigationRef.current = input;
  const [documentActive, setDocumentActive] = useState(
    () =>
      typeof document !== 'undefined' &&
      document.visibilityState === 'visible' &&
      document.hasFocus()
  );
  const mx = useMatrixClient();
  const ownUserId = mx.getUserId() ?? undefined;
  const [reactionViewer, setReactionViewer] = useState<{
    eventId: string;
    initialKey?: string;
    reactions: NativeReactionReadback[];
  } | null>(null);
  const roomList = useNativeRoomListSnapshot();
  const sourceEncryptionStatus = roomList.rooms.find(
    (room) => room.roomId === roomId
  )?.encryptionStatus;
  const scrollRef = useRef<HTMLDivElement>(null);
  const paginationInFlightRef = useRef<'backwards' | 'forwards' | undefined>(undefined);
  const pendingBackwardGrowRef = useRef(false);
  const lastParkedStartRef = useRef(-1);
  const lastClientHeightRef = useRef(0);
  const userInitiatedScrollRef = useRef(false);
  const followingLiveRef = useRef(false);
  const applyingStickRef = useRef(false);
  const parkedPinFrameRef = useRef(0);
  const parkedPinGenerationRef = useRef(0);
  const parkedVisualTopRef = useRef<number | undefined>(undefined);
  const parkedResizeObserverRef = useRef<ResizeObserver | undefined>(undefined);
  const programmaticScrollUntilRef = useRef(0);
  const lastDistanceFromBottomRef = useRef(Number.POSITIVE_INFINITY);
  const stickToLiveTail = useCallback(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || !followingLiveRef.current) return;
    const nextTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
    if (Math.abs(scrollEl.scrollTop - nextTop) <= 0.5) {
      lastDistanceFromBottomRef.current = 0;
      return;
    }
    applyingStickRef.current = true;
    programmaticScrollUntilRef.current = Math.max(
      programmaticScrollUntilRef.current,
      performance.now() + 48
    );
    scrollEl.scrollTop = nextTop;
    applyingStickRef.current = false;
    lastDistanceFromBottomRef.current = 0;
  }, []);
  const smoothScrollActiveRef = useRef(false);
  const lastTotalSizeRef = useRef(0);
  const [hideMembershipEvents] = useSetting(settingsAtom, 'hideMembershipEvents');
  const [hideNickAvatarEvents] = useSetting(settingsAtom, 'hideNickAvatarEvents');
  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');
  const [messageSpacing] = useSetting(settingsAtom, 'messageSpacing');
  const timelineReady = readyState !== undefined;
  useEffect(() => {
    const element = scrollRef.current;
    if (!timelineReady || !element) return undefined;
    return observeNativeTimelineBottom(element, (atBottom) => {
      setAtLiveBottom(atBottom);
      if (atBottom && readyState?.selectedPosition.kind === 'live_bottom') {
        followingLiveRef.current = true;
      }
      const resized =
        lastClientHeightRef.current > 0 && element.clientHeight !== lastClientHeightRef.current;
      lastClientHeightRef.current = element.clientHeight;
      // A follow-live viewport that shrinks would otherwise leave the tail. Do
      // not treat a user scroll (same clientHeight) as a resize.
      if (resized) stickToLiveTail();
    });
  }, [roomId, timelineReady, readyState?.selectedPosition.kind, stickToLiveTail]);
  useEffect(() => {
    if (activeSessionGeneration === undefined) return;
    bindNativeTimelineActionSession(activeSessionGeneration);
  }, [activeSessionGeneration]);
  const rows = useMemo(() => {
    const raw = readyState?.snapshot.rows ?? [];
    return raw.filter((row) => {
      if (hideMembershipEvents && row.kind === 'membership') return false;
      if (hideNickAvatarEvents && row.kind === 'state') return false;
      return true;
    });
  }, [hideMembershipEvents, hideNickAvatarEvents, readyState?.snapshot.rows]);
  const latestRemoteTailEventId = useMemo(
    () => latestNativeReadEventId(readyState?.snapshot.rows.map(rowEventId) ?? []),
    [readyState?.snapshot.rows]
  );
  const receiptTailEventId = nativeVisibleReadFrontier(
    latestRemoteTailEventId,
    readyState?.snapshot.readState
  );
  const rowsRef = useRef(rows);
  rowsRef.current = rows;
  const rowIndexByKey = useMemo(() => {
    const map = new Map<string, number>();
    for (let index = 0; index < rows.length; index += 1) {
      map.set(rowKey(rows[index]), index);
    }
    return map;
  }, [rows]);
  const rowIndexByKeyRef = useRef(rowIndexByKey);
  rowIndexByKeyRef.current = rowIndexByKey;
  const estimateSize = useCallback(
    (index: number) => {
      const current = rowsRef.current;
      const row = current[index];
      if (!row) return NATIVE_TIMELINE_DEFAULT_ROW_ESTIMATE_PX;
      const measuredKey = measuredSizeKeyForRow(roomId, current, index);
      const measured = measuredKey ? nativeTimelineMeasuredSize(measuredKey) : undefined;
      if (measured) return measured;
      return estimateNativeTimelineRowSize(
        nativeTimelineRowSizeHint(row, isGroupedWithPrevious(current[index - 1], row))
      );
    },
    [roomId]
  );
  const measureElement = useCallback(
    (element: HTMLDivElement, entry?: ResizeObserverEntry) => {
      const measured = entry?.borderBoxSize?.[0]?.blockSize
        ? Math.round(entry.borderBoxSize[0].blockSize)
        : Math.round(element.getBoundingClientRect().height);
      const current = rowsRef.current;
      const keyed = element.dataset.nativeTimelineRowKey;
      const fromKey = keyed !== undefined ? rowIndexByKeyRef.current.get(keyed) : undefined;
      // A recycled node can fire ResizeObserver after the keyed row left the
      // window. Falling back to dataset.index would cache this height on the
      // row that now occupies that slot.
      if (keyed !== undefined && fromKey === undefined) return measured;
      const index = fromKey ?? Number(element.dataset.index);
      const row = Number.isInteger(index) ? current[index] : undefined;
      if (row && measured > 0) {
        const measuredKey = measuredSizeKeyForRow(roomId, current, index);
        if (measuredKey) rememberNativeTimelineMeasuredSize(measuredKey, measured);
      }
      return measured;
    },
    [roomId]
  );
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: rows.length,
    getScrollElement: useCallback(() => scrollRef.current, []),
    // TanStack memoizes its measurements on `getItemKey` identity and `count`,
    // so this callback must change whenever `rows` changes: a same-count batch
    // (insert + remove, set) would otherwise keep stale key-to-index mappings.
    getItemKey: useCallback(
      (index: number) => {
        const row = rows[index];
        return row ? rowKey(row) : index;
      },
      [rows]
    ),
    estimateSize,
    measureElement,
    overscan: 8,
  });

  const initialPlacementRef = useRef<string | undefined>(undefined);
  const saveViewport = useCallback(() => {
    // Programmatic prepend/stick/placement must not snapshot in-flight
    // geometry. A rows-change effect used to call this while the parked
    // event was still off the virtual window, which cleared visualTopPx
    // and replaced the user's row with a prepended one.
    if (performance.now() < programmaticScrollUntilRef.current) return;
    const scrollEl = scrollRef.current;
    if (!scrollEl || rows.length === 0) return;
    const atBottom = scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 8;
    if (atBottom) {
      parkedVisualTopRef.current = undefined;
      setNativeTimelineViewport(roomId, { atBottom: true });
      return;
    }
    const visible = virtualizer.getVirtualItems().find((item) => item.end > scrollEl.scrollTop);
    const row = visible ? rows[visible.index] : undefined;
    if (!visible || !row) return;
    lastParkedStartRef.current = visible.start;
    const anchorEventId = rowEventId(row);
    const visualTop = anchorEventId ? parkedNodeVisualTop(scrollEl, anchorEventId) : undefined;
    // Keep the last user-parked visual top when the event has left the
    // virtual window this frame. Assigning undefined here made the next
    // prepend pin to align-start and drop the intra-row offset.
    if (visualTop !== undefined) parkedVisualTopRef.current = visualTop;
    else if (!anchorEventId) parkedVisualTopRef.current = undefined;
    setNativeTimelineViewport(roomId, {
      atBottom: false,
      anchor: {
        itemId: rowKey(row),
        eventId: anchorEventId,
        offsetPx: scrollEl.scrollTop - visible.start,
        visualTopPx: visualTop ?? parkedVisualTopRef.current,
      },
    });
  }, [roomId, rows, virtualizer]);

  const liveTailSubmittedKeyRef = useRef<string | undefined>(undefined);
  const liveTailMarkGenerationRef = useRef(0);
  const followLiveSubmittedKeyRef = useRef<string | undefined>(undefined);
  useEffect(() => {
    const updateDocumentActive = () => {
      setDocumentActive(document.visibilityState === 'visible' && document.hasFocus());
    };
    window.addEventListener('focus', updateDocumentActive);
    window.addEventListener('blur', updateDocumentActive);
    document.addEventListener('visibilitychange', updateDocumentActive);
    updateDocumentActive();
    return () => {
      window.removeEventListener('focus', updateDocumentActive);
      window.removeEventListener('blur', updateDocumentActive);
      document.removeEventListener('visibilitychange', updateDocumentActive);
    };
  }, []);

  useEffect(() => {
    liveTailMarkGenerationRef.current += 1;
    userInitiatedScrollRef.current = false;
    followingLiveRef.current = false;
    programmaticScrollUntilRef.current = 0;
    lastDistanceFromBottomRef.current = Number.POSITIVE_INFINITY;
    smoothScrollActiveRef.current = false;
    initialPlacementRef.current = undefined;
    lastTotalSizeRef.current = 0;
    pendingBackwardGrowRef.current = false;
    lastParkedStartRef.current = -1;
    lastClientHeightRef.current = 0;
    applyingStickRef.current = false;
    parkedPinGenerationRef.current += 1;
    parkedVisualTopRef.current = undefined;
    parkedResizeObserverRef.current?.disconnect();
    parkedResizeObserverRef.current = undefined;
    if (parkedPinFrameRef.current !== 0) {
      window.cancelAnimationFrame(parkedPinFrameRef.current);
      parkedPinFrameRef.current = 0;
    }
    liveTailSubmittedKeyRef.current = undefined;
    followLiveSubmittedKeyRef.current = undefined;
    setPendingLastRead(undefined);
    setAtLiveBottom(false);
  }, [roomId]);

  const liveTailReadTarget = readyState
    ? nativeLiveReadTarget({
        selectedRoomId: roomId,
        snapshotRoomId: readyState.snapshot.roomId,
        documentActive,
        hideActivity,
        atLiveBottom,
        positionKind: readyState.selectedPosition.kind,
        canMarkRead: readyState.snapshot.capabilities.markRead,
        latestVisibleEventId: receiptTailEventId,
        ownReadEventId: readyState.snapshot.readState.ownReadEventId,
        isMarkedUnread: readyState.snapshot.readState.isMarkedUnread,
      })
    : undefined;
  const liveTailMarkReadKey =
    liveTailReadTarget && readyState
      ? nativeLiveReadAttemptKey(
          roomId,
          liveTailReadTarget,
          readyState.snapshot.readState.isMarkedUnread
        )
      : undefined;
  const liveTailAlreadyRead = Boolean(
    readyState &&
      receiptTailEventId &&
      readyState.snapshot.readState.ownReadEventId === receiptTailEventId &&
      !readyState.snapshot.readState.isMarkedUnread
  );
  useEffect(() => {
    if (!liveTailMarkReadKey) {
      if (liveTailAlreadyRead) liveTailSubmittedKeyRef.current = undefined;
      return undefined;
    }
    if (liveTailSubmittedKeyRef.current === liveTailMarkReadKey) return undefined;
    if (!atLiveBottom || !documentActive) return undefined;
    const generation = liveTailMarkGenerationRef.current;
    let cancelled = false;
    let animationFrame = 0;
    let submitted = false;
    const markPaintedTailRead = () => {
      animationFrame = 0;
      if (cancelled || submitted || liveTailMarkGenerationRef.current !== generation) {
        return;
      }
      const scrollEl = scrollRef.current;
      if (!scrollEl) return;
      const paintedAtBottom = Boolean(
        scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight <= 8 &&
          document.visibilityState === 'visible' &&
          document.hasFocus()
      );
      if (!paintedAtBottom) return;
      submitted = true;
      liveTailSubmittedKeyRef.current = liveTailMarkReadKey;
      void setReadState({
        action: 'mark_read',
        intent: 'automatic_visibility',
        observedLiveTailEventId: liveTailReadTarget,
      }).catch(() => {
        if (
          liveTailMarkGenerationRef.current === generation &&
          liveTailSubmittedKeyRef.current === liveTailMarkReadKey
        ) {
          liveTailSubmittedKeyRef.current = undefined;
          submitted = false;
        }
      });
    };
    animationFrame = window.requestAnimationFrame(markPaintedTailRead);
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(animationFrame);
    };
  }, [
    atLiveBottom,
    documentActive,
    liveTailAlreadyRead,
    liveTailMarkReadKey,
    liveTailReadTarget,
    setReadState,
  ]);

  // A room opened at an unread/restored/focused position keeps that position
  // no matter how far the user scrolls: forward pagination never re-anchors
  // the stream, so automatic receipts stay gated off forever. When the painted
  // tail is visually at the bottom, attempt one Core-verified follow-live
  // transition per tail; success flips the snapshot to live_bottom and the
  // receipt effect above fires. A stale tail fails closed (Jump to latest
  // stays visible) and a newer tail retries through a fresh key.
  const followLiveTarget =
    readyState && atLiveBottom
      ? nativeFollowLiveTarget({
          roomId,
          atLiveBottom,
          positionKind: readyState.selectedPosition.kind,
          latestVisibleEventId: receiptTailEventId,
        })
      : undefined;
  const followLiveKey =
    followLiveTarget !== undefined
      ? nativeFollowLiveAttemptKey(roomId, followLiveTarget)
      : undefined;
  useEffect(() => {
    if (!followLiveKey || !followLiveTarget) return undefined;
    if (followLiveSubmittedKeyRef.current === followLiveKey) return undefined;
    if (!atLiveBottom || !documentActive) return undefined;
    let cancelled = false;
    let timer = 0;
    const attemptFollowLive = () => {
      if (cancelled) return;
      const scrollEl = scrollRef.current;
      if (!scrollEl) return;
      const paintedAtBottom = Boolean(
        scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight <= 8 &&
          document.visibilityState === 'visible' &&
          document.hasFocus()
      );
      if (!paintedAtBottom) return;
      followLiveSubmittedKeyRef.current = followLiveKey;
      void followLive({ observedLiveTailEventId: followLiveTarget }).catch(() => {
        // Tail not loaded or superseded: keep the stream and the visible Jump
        // to latest path. A newer tail retries through a fresh key.
        if (followLiveSubmittedKeyRef.current === followLiveKey) {
          followLiveSubmittedKeyRef.current = undefined;
        }
      });
    };
    timer = window.setTimeout(attemptFollowLive, 500);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [atLiveBottom, documentActive, followLiveKey, followLiveTarget, followLive]);

  const scrollHandlersRef = useRef<
    { onScroll: () => void; onUserInput: (event?: Event) => void } | undefined
  >(undefined);
  const paginate = controller.paginate;
  const readyStateRef = useRef(readyState);
  readyStateRef.current = readyState;
  const hasReadyState = readyState !== undefined;
  useEffect(() => {
    if (!hasReadyState) {
      scrollHandlersRef.current = undefined;
      return undefined;
    }
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;
    const paginateAtEdge = () => {
      const current = readyStateRef.current;
      if (!current || paginationInFlightRef.current || !userInitiatedScrollRef.current) return;
      const { snapshot } = current;
      const distanceFromBottom = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
      const direction =
        scrollEl.scrollTop <= 96 &&
        snapshot.capabilities.paginateBackward &&
        snapshot.pagination.backward === 'available'
          ? 'backwards'
          : distanceFromBottom <= 96 &&
            snapshot.capabilities.paginateForward &&
            snapshot.pagination.forward === 'available'
          ? 'forwards'
          : undefined;
      if (!direction) return;

      paginationInFlightRef.current = direction;
      if (direction === 'backwards') pendingBackwardGrowRef.current = true;
      setActionError(undefined);
      void paginate(direction)
        .catch((error) => {
          setActionError(
            error instanceof Error ? error.message : 'Native timeline pagination failed.'
          );
        })
        .finally(() => {
          paginationInFlightRef.current = undefined;
        });
    };
    const onScroll = () => {
      const distanceFromBottom = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
      const atBottom = distanceFromBottom <= 8;
      if (applyingStickRef.current) {
        lastDistanceFromBottomRef.current = distanceFromBottom;
        setAtLiveBottom((previous) => (previous === atBottom ? previous : atBottom));
        return;
      }
      const locked = performance.now() < programmaticScrollUntilRef.current;
      // Programmatic prepend/stick scrolls must not overwrite the parked
      // history anchor; the next prepend would then restore the in-flight
      // geometry instead of the user's row.
      if (!locked) saveViewport();
      setAtLiveBottom((previous) => (previous === atBottom ? previous : atBottom));
      if (locked) {
        // Programmatic sticks land within a few pixels of the bottom. A
        // far jump inside the lock is still a real departure. A small move
        // away from a tail we were already stuck to (scrollIntoView, focus)
        // is also a departure — in-flight scrollToIndex starts far from
        // the tail (lastDistance is Infinity until we have been at bottom).
        if (distanceFromBottom > 96) followingLiveRef.current = false;
        else if (
          followingLiveRef.current &&
          lastDistanceFromBottomRef.current <= 8 &&
          distanceFromBottom > 8
        ) {
          followingLiveRef.current = false;
        }
        lastDistanceFromBottomRef.current = distanceFromBottom;
        return;
      }
      lastDistanceFromBottomRef.current = distanceFromBottom;
      // Only leaving the bottom releases follow-live. A scroll that ends at the
      // bottom (the virtualizer re-measuring rows above the viewport after
      // fonts or media load, or a late programmatic placement) keeps the
      // ownership the placement established; a live position re-acquires it.
      followingLiveRef.current =
        atBottom &&
        (readyStateRef.current?.selectedPosition.kind === 'live_bottom' ||
          followingLiveRef.current);
      userInitiatedScrollRef.current = true;
      paginateAtEdge();
    };
    const onUserInput = (event?: Event) => {
      userInitiatedScrollRef.current = true;
      // End is an explicit request for the live tail. Stick now and keep the
      // programmatic lock so the following scroll event cannot drop ownership
      // when the virtualizer is still a few dozen pixels short of max.
      if (event instanceof KeyboardEvent && event.key === 'End') {
        followingLiveRef.current = true;
        stickToLiveTail();
        return;
      }
      // Home is an explicit history request. Drop follow-live before the
      // browser scrolls so a same-turn stick cannot leave the viewport only
      // a few dozen pixels off the tail.
      if (event instanceof KeyboardEvent && event.key === 'Home') {
        followingLiveRef.current = false;
      }
      programmaticScrollUntilRef.current = 0;
      if (smoothScrollActiveRef.current) {
        smoothScrollActiveRef.current = false;
        const el = scrollRef.current;
        if (el) el.scrollTo({ top: el.scrollTop, behavior: 'auto' });
      }
      // A click is not a departure from the live tail. Actual scrolling below
      // recomputes ownership from geometry, including during drag/scroll input.
    };
    scrollHandlersRef.current = { onScroll, onUserInput };
    saveViewport();
    return () => {
      saveViewport();
    };
  }, [hasReadyState, paginate, saveViewport, stickToLiveTail]);

  // The DOM listeners are bound once per mounted viewport and delegate to the
  // latest handlers above. Re-subscribing on every render raced the
  // virtualizer's own scroll listener, which re-renders synchronously at the
  // start of a gesture: that swapped our listener mid-dispatch, and a listener
  // added during dispatch is skipped for the in-flight event. The first scroll
  // of a gesture was lost, `followingLiveRef` stayed true, and the next
  // snapshot snapped a single wheel step back to the live tail.
  useEffect(() => {
    if (!hasReadyState) return undefined;
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;
    const onScroll = () => scrollHandlersRef.current?.onScroll();
    const onUserInput = (event: Event) => scrollHandlersRef.current?.onUserInput(event);
    scrollEl.addEventListener('wheel', onUserInput, { passive: true });
    scrollEl.addEventListener('pointerdown', onUserInput, { passive: true });
    scrollEl.addEventListener('keydown', onUserInput);
    // Focus-induced scrollIntoView does not fire wheel/pointer/key. Treat it
    // as user input so the 48ms stick lock cannot yank the viewport back.
    scrollEl.addEventListener('focusin', onUserInput);
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      scrollEl.removeEventListener('wheel', onUserInput);
      scrollEl.removeEventListener('pointerdown', onUserInput);
      scrollEl.removeEventListener('keydown', onUserInput);
      scrollEl.removeEventListener('focusin', onUserInput);
    };
  }, [hasReadyState, roomId]);

  const pinParkedHistory = useCallback(
    (index: number, offsetPx: number, parkedEventId?: string) => {
      const scrollEl = scrollRef.current;
      if (!scrollEl || index < 0) return;
      programmaticScrollUntilRef.current = Math.max(
        programmaticScrollUntilRef.current,
        performance.now() + 250
      );
      const applyIndexOffset = () => {
        const item = virtualizer.getVirtualItems().find((row) => row.index === index);
        if (!item) return false;
        const desired = Math.max(0, item.start + offsetPx);
        if (Math.abs(scrollEl.scrollTop - desired) > 0.5) scrollEl.scrollTop = desired;
        lastParkedStartRef.current = item.start;
        return true;
      };
      if (!applyIndexOffset()) {
        // The parked row left the virtual window (typical after a large prepend).
        // scrollToIndex(align start) may not expose the item this turn; the
        // visual-top rAF restores the intra-row offset once it is mounted.
        // Do not add offsetPx to the pre-remap scrollTop — that stacked the
        // saved offset onto stale geometry.
        virtualizer.scrollToIndex(index, { align: 'start', behavior: 'auto' });
        applyIndexOffset();
      }
      if (!parkedEventId) return;
      parkedPinGenerationRef.current += 1;
      const generation = parkedPinGenerationRef.current;
      if (parkedPinFrameRef.current !== 0) {
        window.cancelAnimationFrame(parkedPinFrameRef.current);
        parkedPinFrameRef.current = 0;
      }
      parkedResizeObserverRef.current?.disconnect();
      parkedResizeObserverRef.current = undefined;
      const applyDomPin = () => {
        if (parkedPinGenerationRef.current !== generation || followingLiveRef.current) return;
        const el = scrollRef.current;
        if (!el) return;
        const current = parkedNodeVisualTop(el, parkedEventId);
        if (current === undefined) return;
        const desired = parkedVisualTopRef.current;
        // Only restore a visual top recorded from a user park. Capturing here
        // would freeze pre-offset placement (room reentry applies offsetPx on
        // the next frame) and fight Home/End.
        if (desired !== undefined && Math.abs(current - desired) > 0.5) {
          el.scrollTop += current - desired;
        }
        if (!parkedResizeObserverRef.current) {
          const node = el.querySelector(
            `[data-native-timeline-event-id="${CSS.escape(parkedEventId)}"]`
          );
          if (node instanceof HTMLElement) {
            const observer = new ResizeObserver(() => applyDomPin());
            parkedResizeObserverRef.current = observer;
            observer.observe(node);
          }
        }
      };
      applyDomPin();
      const schedule = (remaining: number) => {
        parkedPinFrameRef.current = window.requestAnimationFrame(() => {
          parkedPinFrameRef.current = 0;
          applyDomPin();
          if (remaining > 0 && parkedPinGenerationRef.current === generation) {
            schedule(remaining - 1);
          }
        });
      };
      // Extra frames cover CI measurement of prepended rows after the item
      // remounts; one frame was enough locally and dropped the intra-row pin.
      schedule(3);
    },
    [virtualizer]
  );

  useLayoutEffect(() => {
    if (!readyState || rows.length === 0) return undefined;
    const { snapshot, selectedPosition } = readyState;
    const selectedAnchor =
      selectedPosition.kind === 'focused'
        ? { eventId: selectedPosition.target_event_id, itemId: selectedPosition.target_event_id }
        : selectedPosition.kind === 'unread'
        ? { eventId: selectedPosition.anchor_event_id, itemId: selectedPosition.anchor_event_id }
        : selectedPosition.kind === 'restored' && selectedPosition.anchor_event_id
        ? {
            eventId: selectedPosition.anchor_event_id,
            itemId: selectedPosition.anchor_event_id,
          }
        : undefined;
    const placementKey = `${roomId}:${snapshot.sessionGeneration}:${selectedPosition.kind}:${
      selectedAnchor?.eventId ?? ''
    }`;
    const initialPlacement = initialPlacementRef.current !== placementKey;
    const savedViewport = initialPlacement
      ? openingViewport ?? nativeTimelineViewports.get(roomId)
      : nativeTimelineViewports.get(roomId);
    const parkedIndex = savedViewport?.anchor ? findAnchorIndex(rows, savedViewport.anchor) : -1;
    const explicitLatest = latestPlacementRequest !== appliedLatestPlacementRef.current;
    appliedLatestPlacementRef.current = latestPlacementRequest;
    const explicitLastRead = lastReadPlacementRequest !== appliedLastReadPlacementRef.current;
    appliedLastReadPlacementRef.current = lastReadPlacementRequest;
    const totalSize = virtualizer.getTotalSize();
    const previousTotalSize = lastTotalSizeRef.current;
    lastTotalSizeRef.current = totalSize;

    if (initialPlacement || explicitLatest || explicitLastRead) {
      const missingLastRead =
        selectedPosition.kind === 'unread' &&
        selectedAnchor &&
        findAnchorIndex(rows, selectedAnchor) < 0;
      // An unavailable marker is an action target, not a viewport. Retain a
      // saved history location when possible; a new room starts at its live tail.
      const anchor = missingLastRead
        ? savedViewport?.atBottom
          ? undefined
          : savedViewport?.anchor
        : selectedAnchor ?? savedViewport?.anchor;
      const anchorIndex = anchor ? findAnchorIndex(rows, anchor) : -1;
      if (missingLastRead) setPendingLastRead(selectedPosition.anchor_event_id);
      // Passive live promotion must preserve an unresolved last-read action.
      if (
        selectedPosition.kind === 'live_bottom' ||
        explicitLatest ||
        (missingLastRead && anchorIndex < 0)
      ) {
        followingLiveRef.current = true;
        programmaticScrollUntilRef.current = performance.now() + 250;
        virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      } else if (anchorIndex >= 0) {
        followingLiveRef.current = false;
        setAtLiveBottom(false);
        programmaticScrollUntilRef.current = performance.now() + 250;
        virtualizer.scrollToIndex(anchorIndex, { align: 'start', behavior: 'auto' });
        const offsetPx =
          selectedAnchor && !missingLastRead ? 0 : savedViewport?.anchor?.offsetPx ?? 0;
        if (selectedAnchor && !missingLastRead) {
          parkedVisualTopRef.current = undefined;
          setPendingLastRead((pending) =>
            selectedAnchor.eventId === pending ? undefined : pending
          );
        } else if (savedViewport?.anchor?.visualTopPx !== undefined) {
          parkedVisualTopRef.current = savedViewport.anchor.visualTopPx;
        }
        pinParkedHistory(anchorIndex, offsetPx, savedViewport?.anchor?.eventId);
        // scrollToIndex does not always expose the parked item this turn, and a
        // later totalSize pass used to cancel the offset rAF — dropping 24px on
        // unread reentry. Pin again on the next frame without cleanup-cancel.
        window.requestAnimationFrame(() => {
          pinParkedHistory(anchorIndex, offsetPx, savedViewport?.anchor?.eventId);
        });
        initialPlacementRef.current = placementKey;
        return undefined;
      }
      initialPlacementRef.current = placementKey;
      return undefined;
    }

    if (followingLiveRef.current) {
      stickToLiveTail();
      return undefined;
    }

    // Pin a parked history row by its saved start+offset. Applies to prepends
    // and to later measurements of rows above the anchor. Missing last-read
    // reentry keeps a saved history location, so it must pin too.
    if (
      !followingLiveRef.current &&
      parkedIndex >= 0 &&
      savedViewport?.anchor &&
      (selectedPosition.kind === 'live_bottom' ||
        selectedPosition.kind === 'restored' ||
        unreadAnchorIsMissing(selectedPosition, rows))
    ) {
      if (savedViewport.anchor.visualTopPx !== undefined) {
        parkedVisualTopRef.current = savedViewport.anchor.visualTopPx;
      }
      pinParkedHistory(parkedIndex, savedViewport.anchor.offsetPx, savedViewport.anchor.eventId);
      pendingBackwardGrowRef.current = false;
    } else if (
      pendingBackwardGrowRef.current &&
      previousTotalSize > 0 &&
      totalSize > previousTotalSize
    ) {
      pendingBackwardGrowRef.current = false;
      const scrollEl = scrollRef.current;
      if (scrollEl) scrollEl.scrollTop += totalSize - previousTotalSize;
    }
    return undefined;
  }, [
    readyState,
    roomId,
    rows,
    virtualizer,
    latestPlacementRequest,
    lastReadPlacementRequest,
    openingViewport,
    pinParkedHistory,
    stickToLiveTail,
  ]);

  // Sticking to the live tail from the rows effect uses estimated heights for
  // rows `measureElement` has not seen yet. When a measurement then grows the
  // total size (a long message, media with dimensions, late fonts), re-stick in
  // the same layout pass so a following viewport never drifts off the bottom.
  // Parked history uses the same trigger: prepended rows measure after the
  // spacer delta, and only a totalSize-keyed pass can restore the saved offset.
  const totalSize = virtualizer.getTotalSize();
  useLayoutEffect(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl) return;
    if (followingLiveRef.current) {
      stickToLiveTail();
      return;
    }
    const selectedKind = readyState?.selectedPosition.kind;
    if (
      selectedKind !== 'live_bottom' &&
      selectedKind !== 'restored' &&
      !unreadAnchorIsMissing(readyState?.selectedPosition, rows)
    ) {
      return;
    }
    const saved = nativeTimelineViewports.get(roomId);
    if (!saved?.anchor) return;
    const index = findAnchorIndex(rows, saved.anchor);
    if (index < 0) return;
    if (saved.anchor.visualTopPx !== undefined) {
      parkedVisualTopRef.current = saved.anchor.visualTopPx;
    }
    pinParkedHistory(index, saved.anchor.offsetPx, saved.anchor.eventId);
  }, [totalSize, roomId, rows, virtualizer, pinParkedHistory, stickToLiveTail, readyState]);

  const onFocusEvent = useCallback(
    (targetEventId: string) => {
      const index = findAnchorIndex(rows, { itemId: targetEventId, eventId: targetEventId });
      if (index >= 0) {
        smoothScrollActiveRef.current = true;
        virtualizer.scrollToIndex(index, { align: 'center', behavior: 'smooth' });
        return;
      }
      // Not in the current window: reopen focused through the native owner.
      setFocusEventId(targetEventId);
    },
    [rows, virtualizer]
  );

  const jumpToLatest = useCallback(() => {
    setActionError(undefined);
    setPendingLastRead(undefined);
    // The new provider's layout effect places the tail. Until its geometry
    // confirms the bottom, preserve the control and do not claim visibility.
    const navigation = input;
    void loadLatest()
      .then((accepted) => {
        // A stale native result resolves without adoption. A newer focused
        // navigation in the same room also supersedes this placement intent.
        if (accepted && mountedNavigationRef.current === navigation)
          setLatestPlacementRequest((request) => request + 1);
      })
      .catch((error) => {
        if (mountedNavigationRef.current === navigation)
          setActionError(
            error instanceof Error ? error.message : 'Could not open latest messages.'
          );
      });
  }, [input, loadLatest]);
  useEffect(() => observeRoomLatestAfterSend(roomId, jumpToLatest), [roomId, jumpToLatest]);

  const jumpToLastRead = useCallback(() => {
    if (!pendingLastRead) return;
    const navigation = input;
    setActionError(undefined);
    void restoreLastRead(pendingLastRead)
      .then((adoptedAnchor) => {
        if (adoptedAnchor !== undefined && mountedNavigationRef.current === navigation) {
          // Core may have resolved the marker to a rendered neighbour; either
          // way the pending frontier is now placed and the action is spent.
          setPendingLastRead(undefined);
          setLastReadPlacementRequest((request) => request + 1);
        }
      })
      .catch((error) => {
        if (mountedNavigationRef.current === navigation) {
          setActionError(
            error instanceof Error ? error.message : 'Could not open last-read messages.'
          );
        }
      });
  }, [input, pendingLastRead, restoreLastRead]);

  if (timelineState.status === 'unavailable') {
    return (
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Text size="T300">The native timeline is unavailable in this window.</Text>
      </Box>
    );
  }
  if (timelineState.status === 'loading') {
    return (
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Text size="T300">Opening native timeline…</Text>
      </Box>
    );
  }
  if (timelineState.status === 'error') {
    return (
      <Box
        grow="Yes"
        alignItems="Center"
        justifyContent="Center"
        style={{ padding: config.space.S400 }}
      >
        <Text size="T300">{timelineState.error.message}</Text>
      </Box>
    );
  }

  if (!readyState) return null;
  const { snapshot } = readyState;
  const showJumpToLastRead = shouldShowJumpToLastRead(pendingLastRead);
  const showJumpToLatest = shouldShowJumpToLatest(readyState.selectedPosition.kind, atLiveBottom);

  return (
    <Box grow="Yes" direction="Column" style={{ minHeight: 0 }}>
      {actionError && <Text size="T300">{actionError}</Text>}
      <Box grow="Yes" style={{ minHeight: 0, position: 'relative' }}>
        <Scroll ref={scrollRef} visibility="Hover" style={{ height: '100%' }}>
          {snapshot.pagination.backward === 'loading' && (
            <Box justifyContent="Center" style={{ padding: config.space.S200 }}>
              <Spinner size="200" aria-label="Loading older messages" />
            </Box>
          )}
          {rows.length === 0 ? (
            <Box
              alignItems="Center"
              justifyContent="Center"
              style={{ minHeight: '100%', padding: config.space.S400 }}
            >
              <Text size="T300">No messages in this view yet.</Text>
            </Box>
          ) : null}
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative', width: '100%' }}>
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const row = rows[virtualItem.index];
              if (!row) return null;
              return (
                <div
                  key={virtualItem.key}
                  ref={virtualizer.measureElement}
                  data-index={virtualItem.index}
                  data-native-timeline-row-key={rowKey(row)}
                  data-native-timeline-row-kind={row.kind}
                  data-native-timeline-event-id={rowEventId(row)}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    transform: `translateY(${virtualItem.start}px)`,
                    width: '100%',
                    overflowAnchor: 'none',
                  }}
                >
                  <NativeTimelineRow
                    row={row}
                    sessionGeneration={snapshot.sessionGeneration}
                    grouped={isGroupedWithPrevious(
                      virtualItem.index > 0 ? rows[virtualItem.index - 1] : undefined,
                      row
                    )}
                    groupsNext={isGroupedWithPrevious(
                      row,
                      virtualItem.index + 1 < rows.length ? rows[virtualItem.index + 1] : undefined
                    )}
                    roomId={roomId}
                    messageSpacing={messageSpacing}
                    pinnedEventIds={snapshot.pinnedEventIds}
                    sourceEncryptionStatus={sourceEncryptionStatus}
                    onActionError={setActionError}
                    onFocusEvent={onFocusEvent}
                    onViewReactions={(request) => {
                      setReactionViewer(request);
                      void nativeReactionViewFromEventReadback({
                        roomId,
                        eventId: request.eventId,
                      }).then((refreshed) => {
                        if (!refreshed) return;
                        setReactionViewer((current) =>
                          current?.eventId === request.eventId
                            ? { ...current, reactions: refreshed }
                            : current
                        );
                      });
                    }}
                  />
                </div>
              );
            })}
          </div>
          {snapshot.pagination.forward === 'loading' && (
            <Box justifyContent="Center" style={{ padding: config.space.S200 }}>
              <Spinner size="200" aria-label="Loading newer messages" />
            </Box>
          )}
        </Scroll>
        {rows.filter((row) => rowEventId(row)).length <= 1 &&
          snapshot.pagination.backward === 'available' && (
            <Box style={{ position: 'absolute', left: config.space.S400, top: config.space.S300 }}>
              <Button
                onClick={() => {
                  void controller
                    .paginate('backwards')
                    .catch((error) => setActionError(String(error)));
                }}
              >
                <Text>Load older messages</Text>
              </Button>
            </Box>
          )}
        {(showJumpToLastRead || showJumpToLatest) && (
          <Box
            direction="Column"
            gap="200"
            alignItems="End"
            style={{ position: 'absolute', right: config.space.S400, bottom: config.space.S300 }}
          >
            {showJumpToLastRead && (
              <Button
                variant="Secondary"
                fill="Soft"
                radii="Pill"
                outlined
                size="300"
                className={depthCss.quietInteractiveSurface}
                before={<Icon src={Icons.MessageUnread} size="100" />}
                onClick={jumpToLastRead}
              >
                <Text size="B300">Jump to Last Read</Text>
              </Button>
            )}
            {showJumpToLatest && (
              <TooltipProvider
                position="Top"
                offset={4}
                tooltip={
                  <Tooltip>
                    <Text>Jump to latest</Text>
                  </Tooltip>
                }
              >
                {(triggerRef) => (
                  <IconButton
                    ref={triggerRef}
                    variant="SurfaceVariant"
                    radii="Pill"
                    outlined
                    size="300"
                    className={depthCss.quietInteractiveSurface}
                    aria-label="Jump to latest"
                    onClick={jumpToLatest}
                  >
                    <Icon src={Icons.ChevronBottom} size="300" />
                  </IconButton>
                )}
              </TooltipProvider>
            )}
          </Box>
        )}
      </Box>
      <Overlay
        open={Boolean(reactionViewer)}
        backdrop={<OverlayBackdrop />}
        onContextMenu={(event: React.MouseEvent) => event.stopPropagation()}
      >
        <OverlayCenter>
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              returnFocusOnDeactivate: false,
              onDeactivate: () => setReactionViewer(null),
              clickOutsideDeactivates: true,
              escapeDeactivates: stopPropagation,
            }}
          >
            <Modal variant="Surface" size="300">
              {reactionViewer ? (
                <ReactionViewer
                  roomId={roomId}
                  targetEventId={reactionViewer.eventId}
                  reactions={reactionViewer.reactions}
                  initialKey={reactionViewer.initialKey}
                  ownUserId={ownUserId}
                  canRedactOwn={Boolean(snapshot.capabilities.canRedactOwn)}
                  canRedactOther={Boolean(snapshot.capabilities.canRedactOther)}
                  requestClose={() => setReactionViewer(null)}
                />
              ) : null}
            </Modal>
          </FocusTrap>
        </OverlayCenter>
      </Overlay>
    </Box>
  );
}
