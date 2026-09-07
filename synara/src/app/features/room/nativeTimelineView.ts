import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { listen as listenTauriEvent } from '@tauri-apps/api/event';
import {
  convertDesktopFileSrc,
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import { parseHermesAgentPayload, type HermesAgentPayload } from '../../utils/hermes';
import type { RoomEncryptionStatus } from '../matrix-dto/room';

const NATIVE_TIMELINE_VIEW_UPDATED_EVENT = 'matrix-timeline-view-updated';
const TIMELINE_VIEW_SCHEMA_VERSION = 1;

export type NativeTimelinePageState = 'available' | 'exhausted' | 'loading' | 'unavailable';

export type NativeTimelinePosition =
  | { kind: 'live_bottom' }
  | { kind: 'unread'; anchor_event_id: string }
  | { kind: 'focused'; target_event_id: string }
  | { kind: 'restored'; anchor_event_id?: string };

export type NativeTimelineRowCapabilities = {
  react: boolean;
  reply: boolean;
  edit: boolean;
  redact: boolean;
  report: boolean;
  pin: boolean;
  forward: boolean;
  vote: boolean;
  declineCall: boolean;
};

type NativeTimelineEventRowBase = {
  itemId: string;
  eventId?: string;
  senderId: string;
  senderName: string;
  senderAvatarUrl?: string;
  originServerTs: number;
  capabilities: NativeTimelineRowCapabilities;
};

export type NativeTimelineReplyPreview = {
  eventId: string;
  senderId?: string;
  senderName: string;
  body: string;
};

export type NativeTimelineThreadSummary = {
  rootEventId: string;
  replyCount: number;
  latestEventId?: string;
};

export type NativeTimelineReaction = {
  key: string;
  count: number;
  own?: boolean;
};

export type NativeTimelineRelationPresentation = {
  /** Parent preview when this row is an in-reply event. */
  reply?: NativeTimelineReplyPreview;
  /** Authoritative root of the thread this row belongs to. */
  threadRoot?: string;
  /** Summary when this row owns a thread. */
  thread?: NativeTimelineThreadSummary;
  reactions?: NativeTimelineReaction[];
};

/** Core-owned action route. Presenters must not infer this from media handles. */
export type NativeTimelineForwardTransport = 'text' | 'media';

type NativeTimelineMessageRow = NativeTimelineEventRowBase &
  NativeTimelineRelationPresentation & {
    kind: 'message';
    body: string;
    formattedBody?: string;
    agentCardJson?: string;
    /** Core-owned approval eligibility; body parsing below is presentation-only. */
    isAgentApproval?: boolean;
    messageType?: string;
    forwardTransport?: NativeTimelineForwardTransport;
    mediaFilename?: string;
    mediaCaption?: string;
    edited: boolean;
    media?: NativeTimelineMediaHandle;
  };

export const parseNativeTimelineAgentCard = (
  agentCardJson: string | undefined
): HermesAgentPayload | undefined => {
  if (!agentCardJson) return undefined;
  try {
    return parseHermesAgentPayload({ 'in.synara.agent': JSON.parse(agentCardJson) });
  } catch {
    return undefined;
  }
};

export type NativeTimelineMediaHandle = {
  handleId: string;
  mimeType?: string;
  width?: number;
  height?: number;
  durationMs?: number;
};

type NativeTimelineStickerRow = {
  kind: 'sticker';
  event: NativeTimelineEventRowBase;
  media: NativeTimelineMediaHandle;
  forwardTransport?: NativeTimelineForwardTransport;
} & NativeTimelineRelationPresentation;

export type NativeTimelinePollAnswer = {
  id: string;
  text: string;
  voteCount: number;
  own?: boolean;
};

type NativeTimelinePollRow = NativeTimelineEventRowBase &
  NativeTimelineRelationPresentation & {
    kind: 'poll';
    question: string;
    closed: boolean;
    /** Maximum simultaneous selections; absent on older snapshots. */
    maxSelections?: number;
    /** Answer options with counts only (no voter IDs). */
    answers?: NativeTimelinePollAnswer[];
  };

type NativeTimelineMembershipRow = NativeTimelineEventRowBase & {
  kind: 'membership';
  targetUserId: string;
  summary: string;
};

type NativeTimelineStateRow = NativeTimelineEventRowBase & {
  kind: 'state';
  stateType: string;
  summary: string;
};

type NativeTimelineCallRow = NativeTimelineEventRowBase & {
  kind: 'call';
  callKind: string;
};

type NativeTimelineOtherRow = {
  kind: 'other';
  itemId: string;
  eventId?: string;
  event?: NativeTimelineEventRowBase;
  eventType?: string;
  forwardTransport?: NativeTimelineForwardTransport;
  summary: string;
};

type NativeTimelineSimpleRow = {
  kind:
    | 'redacted'
    | 'encrypted_unavailable'
    | 'date_separator'
    | 'read_marker'
    | 'unread_marker'
    | 'timeline_start'
    | 'pagination';
  itemId: string;
  eventId?: string;
  summary?: string;
  timestampMs?: number;
  direction?: string;
  state?: NativeTimelinePageState;
};

/** The SDK-neutral renderer contract. It intentionally has no Matrix SDK types. */
export type NativeTimelineViewRow =
  | NativeTimelineMessageRow
  | NativeTimelineStickerRow
  | NativeTimelinePollRow
  | NativeTimelineMembershipRow
  | NativeTimelineStateRow
  | NativeTimelineCallRow
  | NativeTimelineOtherRow
  | NativeTimelineSimpleRow;

export type NativeTimelineViewSnapshot = {
  schemaVersion: number;
  sessionGeneration: number;
  roomId: string;
  revision: number;
  position: NativeTimelinePosition;
  pagination: {
    backward: NativeTimelinePageState;
    forward: NativeTimelinePageState;
  };
  readState: {
    visibleTailEventId?: string;
    receiptTailEventId?: string;
    ownReadEventId?: string;
    unreadAnchorEventId?: string;
    isMarkedUnread: boolean;
  };
  /** Authoritative `m.room.pinned_events` ids; absent/empty means nothing pinned. */
  pinnedEventIds?: string[];
  rows: NativeTimelineViewRow[];
  capabilities: {
    markRead: boolean;
    markUnread: boolean;
    paginateBackward: boolean;
    paginateForward: boolean;
  };
};

export type NativeTimelineViewDeltaOp =
  | { op: 'append'; rows: NativeTimelineViewRow[] }
  | { op: 'clear' }
  | { op: 'push_front'; row: NativeTimelineViewRow }
  | { op: 'push_back'; row: NativeTimelineViewRow }
  | { op: 'pop_front' | 'pop_back' }
  | { op: 'insert' | 'set'; index: number; row: NativeTimelineViewRow }
  | { op: 'remove'; index: number }
  | { op: 'truncate'; len: number }
  | { op: 'reset'; rows: NativeTimelineViewRow[] };

export type NativeTimelineViewDeltaBatch = {
  schemaVersion: number;
  sessionGeneration: number;
  streamId: string;
  roomId: string;
  revision: number;
  ops: NativeTimelineViewDeltaOp[];
  readState?: NativeTimelineViewSnapshot['readState'];
  pagination?: NativeTimelineViewSnapshot['pagination'];
  /** Full replacement of room pin ids when the native owner observes a change. */
  pinnedEventIds?: string[];
};

export type NativeTimelineOpenReadback = {
  schemaVersion: number;
  streamId: string;
  /** The placement selected by the native owner for this open. */
  position: NativeTimelinePosition;
  snapshot: NativeTimelineViewSnapshot;
};

export type NativeTimelineViewState =
  | { status: 'unavailable' | 'loading'; snapshot?: undefined; error?: undefined }
  | {
      status: 'ready';
      snapshot: NativeTimelineViewSnapshot;
      /** Read back from `matrix_timeline_open`, never inferred from JS room state. */
      selectedPosition: NativeTimelinePosition;
      error?: undefined;
    }
  | { status: 'error'; snapshot?: undefined; error: Error };

export type NativeTimelineViewController = {
  state: NativeTimelineViewState;
  paginate: (direction: 'backwards' | 'forwards') => Promise<void>;
  setReadState: (request: {
    action: 'mark_read' | 'mark_unread';
    intent: 'automatic_visibility' | 'explicit_user';
    observedLiveTailEventId?: string;
  }) => Promise<void>;
  /**
   * Re-anchor a non-live view to the live bottom without reopening. Fails
   * closed when the observed tail is no longer the SDK live tail; on success
   * the controller adopts the flipped position so live-tail gates observe the
   * transition and automatic receipts can proceed.
   */
  followLive: (request: { observedLiveTailEventId: string }) => Promise<void>;
  /** True only when this controller adopted the returned live provider. */
  jumpLatest: () => Promise<boolean>;
  /** Keep the current provider until a focused snapshot contains this read target. */
  restoreLastRead: (eventId: string) => Promise<boolean>;
};

type NativeTimelineReadStateReadback = {
  receiptSent?: boolean;
  acknowledgedEventId?: string;
  snapshot: NativeTimelineViewSnapshot;
};

const isValidIndex = (index: number, length: number, allowEnd = false): boolean =>
  Number.isInteger(index) && index >= 0 && index < length + (allowEnd ? 1 : 0);

/**
 * Pagination and read-state readbacks can lag a live delta. Callers first
 * confirm stream ownership; equal-or-older snapshots then keep the newer view.
 */
export const isNativeTimelineReadbackStale = (
  current: NativeTimelineViewSnapshot | undefined,
  next: NativeTimelineViewSnapshot
): boolean =>
  Boolean(
    current &&
      next.schemaVersion === TIMELINE_VIEW_SCHEMA_VERSION &&
      next.sessionGeneration === current.sessionGeneration &&
      next.roomId === current.roomId &&
      next.revision <= current.revision
  );

/** Follow changes placement even when the SDK has emitted no new row revision. */
export const canAcceptNativeTimelineFollowReadback = (
  current: NativeTimelineViewSnapshot | undefined,
  next: NativeTimelineViewSnapshot
): boolean =>
  Boolean(
    current &&
      next.schemaVersion === TIMELINE_VIEW_SCHEMA_VERSION &&
      next.sessionGeneration === current.sessionGeneration &&
      next.roomId === current.roomId &&
      next.revision >= current.revision &&
      next.position.kind === 'live_bottom'
  );

/**
 * Applies one exact native stream update. Invalid operations and revision gaps
 * are rejected rather than guessed at or repaired with a JS timeline fetch.
 * Metadata-only batches (readState / pagination / pinnedEventIds without row
 * ops) are accepted when the native owner emits live frontier, pagination, or
 * pin-list signals.
 */
export const applyNativeTimelineViewDelta = (
  snapshot: NativeTimelineViewSnapshot,
  batch: NativeTimelineViewDeltaBatch
): NativeTimelineViewSnapshot | undefined => {
  if (
    batch.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
    batch.sessionGeneration !== snapshot.sessionGeneration ||
    batch.roomId !== snapshot.roomId ||
    batch.revision !== snapshot.revision + 1
  ) {
    return undefined;
  }

  const hasMetadata = Boolean(
    batch.readState || batch.pagination || batch.pinnedEventIds !== undefined
  );
  if (batch.ops.length === 0 && !hasMetadata) {
    return undefined;
  }

  const rows = [...snapshot.rows];
  for (const op of batch.ops) {
    switch (op.op) {
      case 'append':
        rows.push(...op.rows);
        break;
      case 'clear':
        rows.length = 0;
        break;
      case 'push_front':
        rows.unshift(op.row);
        break;
      case 'push_back':
        rows.push(op.row);
        break;
      case 'pop_front':
        if (rows.length === 0) return undefined;
        rows.shift();
        break;
      case 'pop_back':
        if (rows.length === 0) return undefined;
        rows.pop();
        break;
      case 'insert':
        if (!isValidIndex(op.index, rows.length, true)) return undefined;
        rows.splice(op.index, 0, op.row);
        break;
      case 'set':
        if (!isValidIndex(op.index, rows.length)) return undefined;
        rows[op.index] = op.row;
        break;
      case 'remove':
        if (!isValidIndex(op.index, rows.length)) return undefined;
        rows.splice(op.index, 1);
        break;
      case 'truncate':
        if (!Number.isInteger(op.len) || op.len < 0 || op.len > rows.length) return undefined;
        rows.length = op.len;
        break;
      case 'reset':
        rows.splice(0, rows.length, ...op.rows);
        break;
      default:
        return undefined;
    }
  }

  return {
    ...snapshot,
    revision: batch.revision,
    rows,
    ...(batch.readState ? { readState: batch.readState } : {}),
    ...(batch.pagination ? { pagination: batch.pagination } : {}),
    ...(batch.pinnedEventIds !== undefined ? { pinnedEventIds: batch.pinnedEventIds } : {}),
  };
};

/** Whether the room pin list currently includes this remote event id. */
export const isNativeTimelineEventPinned = (
  pinnedEventIds: readonly string[] | undefined,
  eventId: string | undefined
): boolean => Boolean(eventId && pinnedEventIds?.includes(eventId));

/**
 * Attach Matrix HTML only when it is non-empty and distinct from plain text.
 * Mirrors the Rust `should_attach_formatted_body` action helper.
 */
export const shouldAttachFormattedBody = (body: string, formattedBody?: string | null): boolean => {
  const html = formattedBody?.trim();
  if (!html) return false;
  return html !== body.trim();
};

/**
 * Preserve Matrix HTML only when the user deliberately edited it, or when the
 * plain-text fallback is unchanged. Reusing the original HTML after changing
 * only the fallback would send two different messages in one event.
 */
export const editedFormattedBodyForSubmit = (
  initialBody: string,
  nextBody: string,
  formattedBody: string,
  formattedBodyWasEdited: boolean
): string | undefined => {
  if (!formattedBodyWasEdited && nextBody.trim() !== initialBody.trim()) return undefined;
  return shouldAttachFormattedBody(nextBody, formattedBody) ? formattedBody.trim() : undefined;
};

/** Prefer the latest remote thread event when focusing from a thread summary. */
export const nativeThreadFocusEventId = (
  thread: NativeTimelineThreadSummary | undefined
): string | undefined => thread?.latestEventId ?? thread?.rootEventId;

export type NativeForwardTargetRoom = {
  roomId: string;
  name?: string;
  encryptionStatus: RoomEncryptionStatus;
  isSpace?: boolean;
};

/**
 * Filter joined rooms for the native forward shell (exclude source/spaces).
 * Product multi-room pickers may add more constraints; this is the DTO-safe core.
 */
export const filterNativeForwardTargets = (
  rooms: readonly NativeForwardTargetRoom[],
  sourceRoomId: string,
  query = ''
): NativeForwardTargetRoom[] => {
  const needle = query.trim().toLowerCase();
  return rooms.filter((room) => {
    if (!room.roomId || room.roomId === sourceRoomId || room.isSpace) return false;
    if (!needle) return true;
    const name = room.name?.toLowerCase() ?? '';
    return name.includes(needle) || room.roomId.toLowerCase().includes(needle);
  });
};

export type NativeForwardEncryptionDecision = 'unavailable' | 'confirm_downgrade' | 'proceed';

/**
 * Fail-closed forward policy over Core's authoritative room-encryption state.
 * Missing, malformed, and SDK Unknown/error projections never become cleartext.
 */
export const nativeForwardEncryptionDecision = (
  source: RoomEncryptionStatus | undefined,
  target: RoomEncryptionStatus | undefined
): NativeForwardEncryptionDecision => {
  if (!source || !target || source === 'unknown' || target === 'unknown') return 'unavailable';
  if (source === 'encrypted' && target === 'not_encrypted') return 'confirm_downgrade';
  return 'proceed';
};

export type NativeTimelineOpenInput = {
  roomId: string;
  position:
    | {
        kind: 'normal';
        restoredAnchorEventId?: string;
        atBottom?: boolean;
        liveTailEventId?: string;
        updatedAtMs?: number;
      }
    | { kind: 'live_bottom' }
    | { kind: 'unread' }
    | { kind: 'focused'; eventId: string };
};

export type NativeTimelineCommandError = {
  code?: string;
  message?: string;
  diagnosticId?: string;
};

/**
 * Convert a native `matrix_timeline_*` rejection into a real `Error` without
 * papering over the native diagnostic. Tauri v2 rejects a
 * `MatrixAuthCommandError` as its serialized `{ code, message, diagnosticId }`
 * object (never an `Error` instance); only non-structured rejections fall back
 * to the generic literal.
 */
export const nativeTimelineCommandError = (error: unknown): Error => {
  if (error instanceof Error) return error;
  if (error && typeof error === 'object') {
    const e = error as NativeTimelineCommandError;
    const message =
      typeof e.message === 'string' && e.message.trim().length > 0
        ? e.message
        : 'The native Matrix timeline is unavailable.';
    const diagnosticId =
      typeof e.diagnosticId === 'string' && e.diagnosticId.trim().length > 0
        ? e.diagnosticId
        : undefined;
    const result = new Error(message);
    if (diagnosticId) {
      (result as Error & { diagnosticId?: string }).diagnosticId = diagnosticId;
    }
    return result;
  }
  if (typeof error === 'string' && error.trim().length > 0) {
    return new Error(error.trim());
  }
  return new Error('Native timeline open failed.');
};

const toNativeTimelineOpenRequest = (input: NativeTimelineOpenInput) => {
  const { position } = input;
  return {
    roomId: input.roomId,
    position:
      position.kind === 'focused'
        ? { kind: 'focused' as const, event_id: position.eventId }
        : position.kind === 'normal'
        ? {
            kind: 'normal' as const,
            restored_anchor_event_id: position.restoredAnchorEventId,
            at_bottom: Boolean(position.atBottom),
            live_tail_event_id: position.liveTailEventId,
            updated_at_ms: position.updatedAtMs,
          }
        : position,
  };
};

/**
 * Opens one native timeline view after registering for its native delta event.
 * Mounted via NativeTimelinePresenter (V-TIMELINE.C1); JS RoomTimeline deleted
 * in V-TIMELINE.C2.
 */
export type NativeTimelineFollowLiveInvoke = (
  command: string,
  args?: Record<string, unknown>
) => Promise<DesktopInvokeResult<NativeTimelineViewSnapshot>>;

const defaultFollowLiveInvoke: NativeTimelineFollowLiveInvoke = (command, args) =>
  invokeDesktopWithAvailability<NativeTimelineViewSnapshot>(command, args);

/**
 * Core-verified follow-live transition for one opened stream. Returns the
 * flipped snapshot on success; Core fails closed when the observed tail is no
 * longer live. There is no presenter fallback: a stale tail keeps the stream
 * and the visible Jump to latest path.
 */
export async function requestNativeTimelineFollowLive(
  input: { streamId: string; observedLiveTailEventId: string },
  invoke: NativeTimelineFollowLiveInvoke = defaultFollowLiveInvoke
): Promise<NativeTimelineViewSnapshot> {
  const result = await invoke('matrix_timeline_follow_live', {
    streamId: input.streamId,
    observedLiveTailEventId: input.observedLiveTailEventId,
  });
  if (!result.available || !result.value) {
    throw new Error('Native timeline follow-live is unavailable.');
  }
  return result.value;
}

// Core subscribes before returning the open snapshot. Keep the candidate's
// intervening deltas until its stream ID is known, with a bounded lifetime and
// size. An incomplete candidate is rejected without replacing the current view.
const createNativeTimelineOpenBuffer = (roomId: string, sessionGeneration?: number) => {
  let batches: NativeTimelineViewDeltaBatch[] = [];
  let retainedItems = 0;
  let invalid = false;
  const cancel = () => {
    batches = [];
    invalid = true;
  };
  return {
    cancel,
    add(batch: NativeTimelineViewDeltaBatch) {
      if (
        invalid ||
        batch.roomId !== roomId ||
        (sessionGeneration !== undefined && batch.sessionGeneration !== sessionGeneration)
      )
        return;
      retainedItems += batch.ops.reduce(
        (count, op) => count + 1 + ('rows' in op ? op.rows.length : 'row' in op ? 1 : 0),
        batch.pinnedEventIds?.length ?? 0
      );
      if (batches.length >= 64 || retainedItems > 2048) {
        cancel();
        return;
      }
      batches.push(batch);
    },
    reconcile(opened: NativeTimelineOpenReadback): NativeTimelineViewSnapshot | undefined {
      if (
        invalid ||
        opened.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        opened.snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        opened.snapshot.roomId !== roomId ||
        (sessionGeneration !== undefined && opened.snapshot.sessionGeneration !== sessionGeneration)
      )
        return undefined;
      let snapshot = opened.snapshot;
      for (const batch of batches) {
        if (batch.streamId !== opened.streamId || batch.revision <= snapshot.revision) continue;
        const next = applyNativeTimelineViewDelta(snapshot, batch);
        if (!next) return undefined;
        snapshot = next;
      }
      return snapshot;
    },
  };
};

export const useNativeTimelineView = (
  input: NativeTimelineOpenInput | undefined
): NativeTimelineViewController => {
  const roomId = input?.roomId;
  const positionKind = input?.position.kind;
  const focusedEventId = input?.position.kind === 'focused' ? input.position.eventId : undefined;
  const normalPosition = input?.position.kind === 'normal' ? input.position : undefined;
  const nativeRequest = useMemo(() => {
    if (!roomId || !positionKind) return undefined;
    return toNativeTimelineOpenRequest({
      roomId,
      position:
        positionKind === 'focused' && focusedEventId
          ? { kind: 'focused', eventId: focusedEventId }
          : positionKind === 'normal'
          ? {
              kind: 'normal',
              restoredAnchorEventId: normalPosition?.restoredAnchorEventId,
              atBottom: normalPosition?.atBottom,
              liveTailEventId: normalPosition?.liveTailEventId,
              updatedAtMs: normalPosition?.updatedAtMs,
            }
          : { kind: positionKind },
    } as NativeTimelineOpenInput);
  }, [
    focusedEventId,
    normalPosition?.atBottom,
    normalPosition?.liveTailEventId,
    normalPosition?.restoredAnchorEventId,
    normalPosition?.updatedAtMs,
    positionKind,
    roomId,
  ]);
  const [state, setState] = useState<NativeTimelineViewState>({ status: 'unavailable' });
  const streamIdRef = useRef<string | undefined>(undefined);
  const snapshotRef = useRef<NativeTimelineViewSnapshot | undefined>(undefined);
  const selectedPositionRef = useRef<NativeTimelinePosition | undefined>(undefined);
  const pendingOpenRef = useRef<ReturnType<typeof createNativeTimelineOpenBuffer> | undefined>(
    undefined
  );
  const beginOpen = useCallback((room: string, generation?: number) => {
    pendingOpenRef.current?.cancel();
    const buffer = createNativeTimelineOpenBuffer(room, generation);
    pendingOpenRef.current = buffer;
    return buffer;
  }, []);
  const finishOpen = useCallback((buffer: ReturnType<typeof createNativeTimelineOpenBuffer>) => {
    buffer.cancel();
    if (pendingOpenRef.current === buffer) pendingOpenRef.current = undefined;
  }, []);
  const navigationRevisionRef = useRef(0);
  const navigationRequestRef = useRef(nativeRequest);
  navigationRequestRef.current = nativeRequest;

  const acceptSnapshot = useCallback((next: NativeTimelineViewSnapshot): boolean => {
    const current = snapshotRef.current;
    if (
      !current ||
      next.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
      next.sessionGeneration !== current.sessionGeneration ||
      next.roomId !== current.roomId ||
      next.revision < current.revision
    ) {
      return false;
    }
    snapshotRef.current = next;
    setState({
      status: 'ready',
      snapshot: next,
      selectedPosition: selectedPositionRef.current ?? next.position,
    });
    return true;
  }, []);

  const paginate = useCallback(
    async (direction: 'backwards' | 'forwards') => {
      const streamId = streamIdRef.current;
      const snapshot = snapshotRef.current;
      const permitted =
        direction === 'backwards'
          ? snapshot?.capabilities.paginateBackward
          : snapshot?.capabilities.paginateForward;
      if (!streamId || !snapshot || !permitted) {
        throw new Error('Native timeline pagination is unavailable.');
      }
      const revision = navigationRevisionRef.current;
      const navigationRequest = navigationRequestRef.current;
      const superseded = () =>
        streamIdRef.current !== streamId ||
        navigationRevisionRef.current !== revision ||
        navigationRequestRef.current !== navigationRequest;
      let result: DesktopInvokeResult<NativeTimelineViewSnapshot>;
      try {
        result = await invokeDesktopWithAvailability<NativeTimelineViewSnapshot>(
          'matrix_timeline_paginate',
          { request: { streamId, direction } }
        );
      } catch (error) {
        if (superseded()) return;
        throw error;
      }
      if (superseded()) return;
      if (!result.available || !result.value) {
        throw new Error('Native timeline pagination is unavailable.');
      }
      if (isNativeTimelineReadbackStale(snapshotRef.current, result.value)) return;
      if (!acceptSnapshot(result.value)) {
        setState({
          status: 'error',
          error: new Error('Native timeline pagination lost synchronization.'),
        });
        throw new Error('Native timeline pagination lost synchronization.');
      }
    },
    [acceptSnapshot]
  );

  const setReadState = useCallback(
    async (request: {
      action: 'mark_read' | 'mark_unread';
      intent: 'automatic_visibility' | 'explicit_user';
      observedLiveTailEventId?: string;
    }) => {
      const streamId = streamIdRef.current;
      const snapshot = snapshotRef.current;
      const permitted =
        request.action === 'mark_read'
          ? snapshot?.capabilities.markRead
          : snapshot?.capabilities.markUnread;
      if (!streamId || !snapshot || !permitted) {
        throw new Error('Native timeline read action is unavailable.');
      }
      const revision = navigationRevisionRef.current;
      const navigationRequest = navigationRequestRef.current;
      const superseded = () =>
        streamIdRef.current !== streamId ||
        navigationRevisionRef.current !== revision ||
        navigationRequestRef.current !== navigationRequest;
      let result: DesktopInvokeResult<NativeTimelineReadStateReadback>;
      try {
        result = await invokeDesktopWithAvailability<NativeTimelineReadStateReadback>(
          'matrix_timeline_set_read_state',
          { request: { streamId, ...request } }
        );
      } catch (error) {
        if (superseded()) return;
        throw error;
      }
      if (superseded()) return;
      if (!result.available || !result.value) {
        throw new Error('Native timeline read action is unavailable.');
      }
      const next = result.value.snapshot;
      if (isNativeTimelineReadbackStale(snapshotRef.current, next)) {
        return;
      }
      if (acceptSnapshot(next)) {
        return;
      }
      if (isNativeTimelineReadbackStale(snapshotRef.current, next)) {
        return;
      }
      setState({
        status: 'error',
        error: new Error('Native timeline read state lost synchronization.'),
      });
      throw new Error('Native timeline read state lost synchronization.');
    },
    [acceptSnapshot]
  );

  const followLive = useCallback(
    async (request: { observedLiveTailEventId: string }) => {
      const streamId = streamIdRef.current;
      const snapshot = snapshotRef.current;
      if (!streamId || !snapshot) {
        throw new Error('Native timeline follow-live is unavailable.');
      }
      const revision = navigationRevisionRef.current;
      const navigationRequest = navigationRequestRef.current;
      const superseded = () =>
        streamIdRef.current !== streamId ||
        navigationRevisionRef.current !== revision ||
        navigationRequestRef.current !== navigationRequest;
      let next: NativeTimelineViewSnapshot;
      try {
        next = await requestNativeTimelineFollowLive(
          { streamId, observedLiveTailEventId: request.observedLiveTailEventId },
          invokeDesktopWithAvailability
        );
      } catch (error) {
        if (superseded()) return;
        throw error;
      }
      if (
        superseded() ||
        snapshotRef.current?.sessionGeneration !== snapshot.sessionGeneration ||
        !canAcceptNativeTimelineFollowReadback(snapshotRef.current, next)
      ) {
        return;
      }
      // Adopt the flipped position so live-tail gates observe the transition.
      selectedPositionRef.current = next.position;
      if (acceptSnapshot(next)) {
        return;
      }
      // A superseded follow is not a desync: the stream keeps flowing and the
      // next painted tail retries through the same path.
    },
    [acceptSnapshot]
  );

  const jumpLatest = useCallback(async () => {
    const streamId = streamIdRef.current;
    const current = snapshotRef.current;
    if (!streamId || !current) {
      throw new Error('Native timeline jump-to-latest is unavailable.');
    }
    const buffer = beginOpen(current.roomId, current.sessionGeneration);
    const revision = ++navigationRevisionRef.current;
    const navigationRequest = navigationRequestRef.current;
    const superseded = () =>
      streamIdRef.current !== streamId ||
      navigationRevisionRef.current !== revision ||
      navigationRequestRef.current !== navigationRequest;
    let result: DesktopInvokeResult<NativeTimelineOpenReadback>;
    try {
      result = await invokeDesktopWithAvailability<NativeTimelineOpenReadback>(
        'matrix_timeline_jump_latest',
        { request: { streamId } }
      );
    } catch (error) {
      finishOpen(buffer);
      if (superseded()) return false;
      throw error;
    }
    if (superseded()) {
      finishOpen(buffer);
      if (
        result.available &&
        result.value?.streamId &&
        result.value.streamId !== streamIdRef.current
      ) {
        void invokeDesktopWithAvailability('matrix_timeline_close', {
          request: { streamId: result.value.streamId },
        }).catch(() => undefined);
      }
      return false;
    }
    const snapshot = result.available && result.value ? buffer.reconcile(result.value) : undefined;
    finishOpen(buffer);
    if (!result.available || !result.value || !snapshot) {
      if (
        result.available &&
        result.value?.streamId &&
        result.value.streamId !== streamIdRef.current
      ) {
        void invokeDesktopWithAvailability('matrix_timeline_close', {
          request: { streamId: result.value.streamId },
        }).catch(() => undefined);
      }
      setState({
        status: 'error',
        error: new Error('Native timeline jump-to-latest lost synchronization.'),
      });
      throw new Error('Native timeline jump-to-latest lost synchronization.');
    }
    streamIdRef.current = result.value.streamId;
    selectedPositionRef.current = result.value.position;
    snapshotRef.current = snapshot;
    setState({
      status: 'ready',
      snapshot,
      selectedPosition: result.value.position,
    });
    return true;
  }, [beginOpen, finishOpen]);

  const restoreLastRead = useCallback(
    async (eventId: string) => {
      const streamId = streamIdRef.current;
      const current = snapshotRef.current;
      if (!streamId || !current) throw new Error('Last-read navigation is unavailable.');
      const buffer = beginOpen(current.roomId, current.sessionGeneration);
      const revision = ++navigationRevisionRef.current;
      const navigationRequest = navigationRequestRef.current;
      const superseded = () =>
        navigationRevisionRef.current !== revision ||
        navigationRequestRef.current !== navigationRequest ||
        streamIdRef.current !== streamId;
      let result: DesktopInvokeResult<NativeTimelineOpenReadback>;
      try {
        result = await invokeDesktopWithAvailability<NativeTimelineOpenReadback>(
          'matrix_timeline_open',
          {
            request: toNativeTimelineOpenRequest({
              roomId: current.roomId,
              position: { kind: 'focused', eventId },
            }),
          }
        );
      } catch (error) {
        finishOpen(buffer);
        if (superseded()) return false;
        throw error;
      }
      const opened = result.available ? result.value : undefined;
      const snapshot = opened ? buffer.reconcile(opened) : undefined;
      finishOpen(buffer);
      const discard = () => {
        if (opened?.streamId && opened.streamId !== streamIdRef.current) {
          void invokeDesktopWithAvailability('matrix_timeline_close', {
            request: { streamId: opened.streamId },
          }).catch(() => undefined);
        }
      };
      if (superseded()) {
        discard();
        return false;
      }
      if (
        !result.available ||
        !opened ||
        !snapshot ||
        opened.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        snapshot.sessionGeneration !== current.sessionGeneration ||
        snapshot.roomId !== current.roomId ||
        opened.position.kind !== 'focused' ||
        opened.position.target_event_id !== eventId ||
        snapshot.position.kind !== 'focused' ||
        snapshot.position.target_event_id !== eventId ||
        !snapshot.rows.some(
          (row) => (row.kind === 'sticker' ? row.event.eventId : row.eventId) === eventId
        )
      ) {
        discard();
        throw new Error('The last-read message is not available in this context. Try again.');
      }
      // Each Core open owns its own subscription. Retire the previous provider
      // only after the requested target has been confirmed in its replacement.
      streamIdRef.current = opened.streamId;
      snapshotRef.current = snapshot;
      selectedPositionRef.current = opened.position;
      setState({ status: 'ready', snapshot, selectedPosition: opened.position });
      void invokeDesktopWithAvailability('matrix_timeline_close', {
        request: { streamId },
      }).catch(() => undefined);
      return true;
    },
    [beginOpen, finishOpen]
  );

  useEffect(() => {
    navigationRevisionRef.current += 1;
    streamIdRef.current = undefined;
    snapshotRef.current = undefined;
    selectedPositionRef.current = undefined;
    pendingOpenRef.current?.cancel();
    pendingOpenRef.current = undefined;
    if (!nativeRequest || !isSynaraDesktop()) {
      setState({ status: 'unavailable' });
      return undefined;
    }

    let disposed = false;
    let unlisten: (() => void) | undefined;
    let pollTimer: number | undefined;
    const applyBatch = (batch: NativeTimelineViewDeltaBatch) => {
      if (disposed || batch.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION) return;
      if (batch.streamId !== streamIdRef.current || !snapshotRef.current) {
        pendingOpenRef.current?.add(batch);
        return;
      }
      const next = applyNativeTimelineViewDelta(snapshotRef.current, batch);
      if (!next) {
        setState({
          status: 'error',
          error: new Error('Native timeline stream lost synchronization.'),
        });
        return;
      }
      snapshotRef.current = next;
      setState({
        status: 'ready',
        snapshot: next,
        selectedPosition: selectedPositionRef.current ?? next.position,
      });
    };

    const pollSnapshot = async () => {
      const streamId = streamIdRef.current;
      if (!streamId || disposed) return;
      const revision = navigationRevisionRef.current;
      const navigationRequest = navigationRequestRef.current;
      const result = await invokeDesktopWithAvailability<NativeTimelineViewSnapshot>(
        'matrix_timeline_snapshot',
        { streamId }
      ).catch(() => undefined);
      if (
        disposed ||
        streamIdRef.current !== streamId ||
        navigationRevisionRef.current !== revision ||
        navigationRequestRef.current !== navigationRequest ||
        !result?.available ||
        !result.value
      )
        return;
      const snapshot = result.value;
      if (
        snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        snapshot.roomId !== nativeRequest.roomId
      ) {
        return;
      }
      if (!snapshotRef.current) {
        snapshotRef.current = snapshot;
        setState({
          status: 'ready',
          snapshot,
          selectedPosition: selectedPositionRef.current ?? snapshot.position,
        });
        return;
      }
      acceptSnapshot(snapshot);
    };

    const open = async () => {
      const buffer = beginOpen(nativeRequest.roomId);
      setState({ status: 'loading' });
      try {
        try {
          unlisten = await listenTauriEvent<NativeTimelineViewDeltaBatch>(
            NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
            ({ payload }) => applyBatch(payload)
          );
        } catch {
          unlisten = undefined;
        }
        if (disposed) {
          unlisten?.();
          return;
        }
        const result = await invokeDesktopWithAvailability<NativeTimelineOpenReadback>(
          'matrix_timeline_open',
          { request: nativeRequest }
        );
        if (disposed) {
          if (result.available && result.value?.streamId) {
            void invokeDesktopWithAvailability('matrix_timeline_close', {
              request: { streamId: result.value.streamId },
            });
          }
          return;
        }
        if (!result.available || !result.value) {
          setState({ status: 'unavailable' });
          return;
        }
        const readback = result.value;
        if (
          readback.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
          readback.snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
          readback.snapshot.roomId !== nativeRequest.roomId ||
          readback.position.kind !== readback.snapshot.position.kind
        ) {
          void invokeDesktopWithAvailability('matrix_timeline_close', {
            request: { streamId: readback.streamId },
          }).catch(() => undefined);
          setState({ status: 'error', error: new Error('Unsupported native timeline schema.') });
          return;
        }
        const snapshot = buffer.reconcile(readback);
        if (!snapshot) {
          void invokeDesktopWithAvailability('matrix_timeline_close', {
            request: { streamId: readback.streamId },
          }).catch(() => undefined);
          throw new Error('Native timeline stream lost synchronization.');
        }
        finishOpen(buffer);
        streamIdRef.current = readback.streamId;
        snapshotRef.current = snapshot;
        selectedPositionRef.current = readback.position;
        setState({ status: 'ready', snapshot, selectedPosition: readback.position });
        if (!disposed) {
          pollTimer = window.setInterval(
            () => {
              void pollSnapshot();
            },
            unlisten ? 1500 : 750
          );
        }
      } catch (error) {
        if (!disposed) {
          setState({
            status: 'error',
            error: nativeTimelineCommandError(error),
          });
        }
      } finally {
        finishOpen(buffer);
      }
    };

    void open();
    return () => {
      disposed = true;
      pendingOpenRef.current?.cancel();
      pendingOpenRef.current = undefined;
      navigationRevisionRef.current += 1;
      const streamId = streamIdRef.current;
      if (streamId) {
        void invokeDesktopWithAvailability('matrix_timeline_close', {
          request: { streamId },
        });
      }
      if (pollTimer !== undefined) window.clearInterval(pollTimer);
      unlisten?.();
    };
  }, [acceptSnapshot, beginOpen, finishOpen, nativeRequest]);

  return { state, paginate, setReadState, followLive, jumpLatest, restoreLastRead };
};

export const nativeTimelineMediaSrc = (handle: NativeTimelineMediaHandle): string | undefined =>
  convertDesktopFileSrc(handle.handleId, 'synara-media');
