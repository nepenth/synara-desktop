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
import { toggleReactionWithNativeOwner } from './nativeReactionOwner';
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
import { detectAgentApprovalPrompt } from '../../utils/agentApprovals';
import { NativeFormattedBody } from './nativeTimelineFormattedBody';
import {
  nativeLiveReadAttemptKey,
  nativeLiveReadTarget,
  latestNativeReadEventId,
  shouldShowJumpToLatest,
} from './nativeTimelineViewportPolicy';
import { shouldGroupNativeTimelineRows } from './nativeTimelineGrouping';
import * as htmlCss from './nativeTimelineHtml.css';

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

const mediaStyle = (media?: NativeTimelineMediaHandle): React.CSSProperties => {
  const maxWidth = media?.width ? Math.min(media.width, 480) : 480;
  const maxHeight = media?.height ? Math.min(media.height, 480) : 480;
  return { maxWidth, maxHeight, width: 'auto', height: 'auto' };
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
  const closeAfterOneShotAction = () => onRequestClose?.();
  if (capabilities.reply) {
    buttons.push(
      <MenuItem
        key="reply"
        size="300"
        fill="Soft"
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
        fill="Soft"
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
        fill="Soft"
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
        fill="Soft"
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
    buttons.push(
      <MenuItem
        key="redact"
        variant="Critical"
        size="300"
        fill="Soft"
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
    buttons.push(
      <MenuItem
        key="report"
        variant="Critical"
        size="300"
        fill="Soft"
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
        fill="Soft"
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
      fill="Soft"
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
                    <Menu data-native-timeline-action-menu="true">
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
  if (!mediaSrc) {
    return sticker ? <Text size="T300">Sticker media is unavailable.</Text> : null;
  }
  if (sticker) {
    return <img src={mediaSrc} alt="Sticker" style={{ maxWidth: 256, maxHeight: 256 }} />;
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
}: {
  reactions?: NativeTimelineReaction[];
  enabled: boolean;
  onReaction: (key: string) => void;
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
          onClick={() => onReaction(reaction.key)}
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

  switch (row.kind) {
    case 'message': {
      const isEmote = row.messageType === 'emote';
      const agentPayload = parseNativeTimelineAgentCard(row.agentCardJson);
      const approvalPrompt = row.isAgentApproval
        ? detectAgentApprovalPrompt({ body: row.body, formatted_body: row.formattedBody })
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
  const { setReadState } = controller;
  const timelineState = controller.state;
  const readyState = timelineState.status === 'ready' ? timelineState : undefined;
  const activeSessionGeneration = readyState?.snapshot.sessionGeneration;
  const [actionError, setActionError] = useState<string>();
  const [atLiveBottom, setAtLiveBottom] = useState(false);
  const [documentActive, setDocumentActive] = useState(
    () =>
      typeof document !== 'undefined' &&
      document.visibilityState === 'visible' &&
      document.hasFocus()
  );
  const roomList = useNativeRoomListSnapshot();
  const sourceEncryptionStatus = roomList.rooms.find(
    (room) => room.roomId === roomId
  )?.encryptionStatus;
  const scrollRef = useRef<HTMLDivElement>(null);
  const paginationInFlightRef = useRef<'backwards' | 'forwards' | undefined>(undefined);
  const pendingBackwardGrowRef = useRef(false);
  const userInitiatedScrollRef = useRef(false);
  const followingLiveRef = useRef(false);
  const programmaticScrollUntilRef = useRef(0);
  const lastTotalSizeRef = useRef(0);
  const [hideMembershipEvents] = useSetting(settingsAtom, 'hideMembershipEvents');
  const [hideNickAvatarEvents] = useSetting(settingsAtom, 'hideNickAvatarEvents');
  const [hideActivity] = useSetting(settingsAtom, 'hideActivity');
  const [messageSpacing] = useSetting(settingsAtom, 'messageSpacing');
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
  const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
    count: rows.length,
    getScrollElement: useCallback(() => scrollRef.current, []),
    getItemKey: useCallback(
      (index: number) => {
        const row = rows[index];
        return row ? rowKey(row) : index;
      },
      [rows]
    ),
    estimateSize: useCallback(() => 64, []),
    overscan: 8,
  });

  const initialPlacementRef = useRef<string | undefined>(undefined);
  const saveViewport = useCallback(() => {
    const scrollEl = scrollRef.current;
    if (!scrollEl || rows.length === 0) return;
    const atBottom = scrollEl.scrollTop + scrollEl.clientHeight >= scrollEl.scrollHeight - 8;
    if (atBottom) {
      setNativeTimelineViewport(roomId, { atBottom: true });
      return;
    }
    const visible = virtualizer.getVirtualItems().find((item) => item.end > scrollEl.scrollTop);
    const row = visible ? rows[visible.index] : undefined;
    if (!visible || !row) return;
    setNativeTimelineViewport(roomId, {
      atBottom: false,
      anchor: {
        itemId: rowKey(row),
        eventId: rowEventId(row),
        offsetPx: scrollEl.scrollTop - visible.start,
      },
    });
  }, [roomId, rows, virtualizer]);

  const liveTailSubmittedKeyRef = useRef<string | undefined>(undefined);
  const liveTailMarkGenerationRef = useRef(0);
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
    initialPlacementRef.current = undefined;
    lastTotalSizeRef.current = 0;
    pendingBackwardGrowRef.current = false;
    liveTailSubmittedKeyRef.current = undefined;
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
        latestVisibleEventId: latestRemoteTailEventId,
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
      latestRemoteTailEventId &&
      readyState.snapshot.readState.ownReadEventId === latestRemoteTailEventId &&
      !readyState.snapshot.readState.isMarkedUnread
  );
  useEffect(() => {
    if (!liveTailMarkReadKey) {
      if (liveTailAlreadyRead) liveTailSubmittedKeyRef.current = undefined;
      return;
    }
    if (liveTailSubmittedKeyRef.current === liveTailMarkReadKey) return;
    const generation = liveTailMarkGenerationRef.current;
    let cancelled = false;
    let animationFrame = 0;
    let submitted = false;
    const scrollEl = scrollRef.current;
    if (!scrollEl) {
      return;
    }
    const markPaintedTailRead = () => {
      animationFrame = 0;
      if (cancelled || submitted || liveTailMarkGenerationRef.current !== generation) {
        return;
      }
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
    const requestPaintCheck = () => {
      if (!cancelled && !submitted && animationFrame === 0) {
        animationFrame = window.requestAnimationFrame(markPaintedTailRead);
      }
    };
    scrollEl.addEventListener('scroll', requestPaintCheck, { passive: true });
    window.addEventListener('focus', requestPaintCheck);
    document.addEventListener('visibilitychange', requestPaintCheck);
    const resizeObserver =
      typeof ResizeObserver === 'undefined' ? undefined : new ResizeObserver(requestPaintCheck);
    resizeObserver?.observe(scrollEl);
    const observeContentSize = () => {
      Array.from(scrollEl.children).forEach((child) => resizeObserver?.observe(child));
    };
    observeContentSize();
    const mutationObserver =
      typeof MutationObserver === 'undefined'
        ? undefined
        : new MutationObserver(() => {
            observeContentSize();
            requestPaintCheck();
          });
    mutationObserver?.observe(scrollEl, {
      childList: true,
      subtree: true,
    });
    requestPaintCheck();
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(animationFrame);
      scrollEl.removeEventListener('scroll', requestPaintCheck);
      window.removeEventListener('focus', requestPaintCheck);
      document.removeEventListener('visibilitychange', requestPaintCheck);
      resizeObserver?.disconnect();
      mutationObserver?.disconnect();
    };
  }, [liveTailAlreadyRead, liveTailMarkReadKey, liveTailReadTarget, setReadState]);

  useEffect(() => {
    if (!readyState) return undefined;
    const scrollEl = scrollRef.current;
    if (!scrollEl) return undefined;
    const paginateAtEdge = () => {
      if (paginationInFlightRef.current) return;
      if (!userInitiatedScrollRef.current) return;
      const { snapshot } = readyState;
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
      void controller
        .paginate(direction)
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
      saveViewport();
      const distanceFromBottom = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight;
      const atBottom = distanceFromBottom <= 8;
      followingLiveRef.current = readyState.selectedPosition.kind === 'live_bottom' && atBottom;
      setAtLiveBottom((previous) => (previous === atBottom ? previous : atBottom));
      if (performance.now() < programmaticScrollUntilRef.current) return;
      userInitiatedScrollRef.current = true;
      paginateAtEdge();
    };
    scrollEl.addEventListener('scroll', onScroll, { passive: true });
    saveViewport();
    return () => {
      scrollEl.removeEventListener('scroll', onScroll);
      saveViewport();
    };
  }, [controller, readyState, saveViewport]);

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
    const savedViewport = nativeTimelineViewports.get(roomId);
    const totalSize = virtualizer.getTotalSize();
    const previousTotalSize = lastTotalSizeRef.current;
    lastTotalSizeRef.current = totalSize;

    if (initialPlacement) {
      const anchor = selectedAnchor ?? savedViewport?.anchor;
      const anchorIndex = anchor ? findAnchorIndex(rows, anchor) : -1;
      if (selectedPosition.kind === 'live_bottom' || savedViewport?.atBottom) {
        followingLiveRef.current = true;
        setAtLiveBottom(true);
        programmaticScrollUntilRef.current = performance.now() + 250;
        virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      } else if (anchorIndex >= 0) {
        followingLiveRef.current = false;
        setAtLiveBottom(false);
        programmaticScrollUntilRef.current = performance.now() + 250;
        virtualizer.scrollToIndex(anchorIndex, { align: 'start', behavior: 'auto' });
        const offsetPx = selectedAnchor ? 0 : savedViewport?.anchor?.offsetPx ?? 0;
        const animationFrame = window.requestAnimationFrame(() => {
          if (scrollRef.current && offsetPx !== 0) scrollRef.current.scrollTop += offsetPx;
        });
        initialPlacementRef.current = placementKey;
        return () => window.cancelAnimationFrame(animationFrame);
      }
      initialPlacementRef.current = placementKey;
      return undefined;
    }

    if (followingLiveRef.current) {
      programmaticScrollUntilRef.current = performance.now() + 250;
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      return undefined;
    }

    if (pendingBackwardGrowRef.current && previousTotalSize > 0 && totalSize > previousTotalSize) {
      pendingBackwardGrowRef.current = false;
      const scrollEl = scrollRef.current;
      if (scrollEl) scrollEl.scrollTop += totalSize - previousTotalSize;
    }
    return undefined;
  }, [readyState, roomId, rows, virtualizer]);

  const onFocusEvent = useCallback(
    (targetEventId: string) => {
      const index = findAnchorIndex(rows, { itemId: targetEventId, eventId: targetEventId });
      if (index >= 0) {
        virtualizer.scrollToIndex(index, { align: 'center', behavior: 'smooth' });
        return;
      }
      // Not in the current window: reopen focused through the native owner.
      setFocusEventId(targetEventId);
    },
    [rows, virtualizer]
  );

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
  const runAction = (action: () => Promise<void>) => {
    setActionError(undefined);
    void action().catch((error) => {
      setActionError(error instanceof Error ? error.message : 'Native timeline action failed.');
    });
  };

  const jumpToLatest = () => {
    followingLiveRef.current = true;
    programmaticScrollUntilRef.current = performance.now() + 500;
    setAtLiveBottom(true);
    if (rows.length > 0) {
      virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
    }
    runAction(async () => {
      await controller.jumpLatest();
      followingLiveRef.current = true;
      programmaticScrollUntilRef.current = performance.now() + 500;
      if (rows.length > 0) {
        virtualizer.scrollToIndex(rows.length - 1, { align: 'end', behavior: 'auto' });
      }
      // The painted-tail effect performs the conditional acknowledgement once
      // the newly returned live tail is actually visible. Do not resolve a
      // newer SDK tail from this navigation callback.
    });
  };

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
                  data-native-timeline-row-kind={row.kind}
                  data-native-timeline-event-id={rowEventId(row)}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    transform: `translateY(${virtualItem.start}px)`,
                    width: '100%',
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
        {shouldShowJumpToLatest(readyState.selectedPosition.kind, atLiveBottom) && (
          <Box
            style={{ position: 'absolute', right: config.space.S400, bottom: config.space.S300 }}
          >
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
                  aria-label="Jump to latest"
                  onClick={jumpToLatest}
                >
                  <Icon src={Icons.ChevronBottom} size="300" />
                </IconButton>
              )}
            </TooltipProvider>
          </Box>
        )}
      </Box>
    </Box>
  );
}
