import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Channel } from '@tauri-apps/api/core';
import {
  convertDesktopFileSrc,
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  type DesktopInvokeResult,
} from '../../utils/desktop';
import { parseHermesAgentPayload, type HermesAgentPayload } from '../../utils/hermes';
import { startVisibleOnlyPoll } from '../../utils/visibilityPoll';
import { readReceiptsArePublic } from '../../state/settings';
import type { RoomEncryptionStatus } from '../matrix-dto/room';
import { devicePixelScale, nativeThumbnailContentUri } from '../../matrix/nativeThumbnail';
import type { NativeTimelineEncryptionShield } from './nativeTimelineShield';
import type {
  NativeTimelineReactionSender,
  SynaraAgentApprovalHistoryDecision,
} from '../matrix-dto/generated';
export type { NativeTimelineReactionSender } from '../matrix-dto/generated';

const TIMELINE_VIEW_SCHEMA_VERSION = 1;

export type NativeTimelinePageState = 'available' | 'exhausted' | 'loading' | 'unavailable';

export type NativeTimelinePosition =
  | { kind: 'live_bottom' }
  | { kind: 'unread'; anchor_event_id: string }
  | { kind: 'focused'; target_event_id: string }
  | { kind: 'restored'; anchor_event_id?: string }
  | { kind: 'thread'; root_event_id: string };

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

export type NativeTimelineLocalEchoState = 'sending' | 'sent' | 'failed' | 'cancelled' | 'wedged';

type NativeTimelineEventRowBase = {
  itemId: string;
  eventId?: string;
  senderId: string;
  senderName: string;
  senderAvatarUrl?: string;
  originServerTs: number;
  /** Absent on remote rows and on older snapshots. A server event id stays sent. */
  localEchoState?: NativeTimelineLocalEchoState;
  /** SDK transaction id for discard and retry. Not a server event id. */
  transactionId?: string;
  /** Closed cause of a failed or wedged echo (`too_large`, `forbidden`, …). */
  sendFailure?: string;
  /** Core's verdict on that cause: `false` means the homeserver refuses a retry again. */
  sendFailureRetryable?: boolean;
  /** SDK authenticity shield in an encrypted room. Absent when trusted. */
  encryptionShield?: NativeTimelineEncryptionShield;
  capabilities: NativeTimelineRowCapabilities;
};

export type NativeTimelineUnsentDelivery = {
  status: 'sending' | 'failed' | 'unsent';
  /** Visible, hover-independent name. Sending and failed stay distinct. */
  accessibleName: 'Sending' | 'Failed to send' | 'Not sent';
  wedged: boolean;
  /**
   * A failed row offers Retry. Off when Core says the same request is refused
   * again (too large, forbidden, not joined, …); Discard stays.
   */
  retryable: boolean;
  transactionId?: string;
};

const isNativeTimelineLocalEchoState = (value: unknown): value is NativeTimelineLocalEchoState =>
  value === 'sending' ||
  value === 'sent' ||
  value === 'failed' ||
  value === 'cancelled' ||
  value === 'wedged';

/** Fields the unsent presentation reads. Sticker and other rows nest them on `event`. */
export const nativeTimelineEchoFields = (
  row: NativeTimelineViewRow
): {
  eventId?: string;
  localEchoState?: NativeTimelineLocalEchoState;
  transactionId?: string;
  sendFailureRetryable?: boolean;
  /** Virtual rows (dates, markers) are not local echoes just because they lack an event id. */
  candidate: boolean;
} => {
  if (row.kind === 'sticker') return { ...row.event, candidate: true };
  if (row.kind === 'other') {
    return row.event
      ? { ...row.event, candidate: true }
      : { eventId: row.eventId, candidate: false };
  }
  if (
    row.kind === 'message' ||
    row.kind === 'poll' ||
    row.kind === 'membership' ||
    row.kind === 'state' ||
    row.kind === 'call'
  ) {
    return { ...row, candidate: true };
  }
  return {
    eventId: 'eventId' in row ? row.eventId : undefined,
    candidate: false,
  };
};

/** The SDK shield of a message-like row. Membership, state and call rows carry none. */
export const nativeTimelineRowShield = (row: NativeTimelineViewRow): unknown => {
  if (row.kind === 'message' || row.kind === 'poll') return row.encryptionShield;
  if (row.kind === 'sticker') return row.event.encryptionShield;
  return undefined;
};

/**
 * Classify a timeline row for the unsent presentation.
 * A missing status with a server event id stays sent. A missing status without
 * one is unsent when the row can be a local echo. Explicit sending / failed /
 * wedged wins over a stale id.
 */
export const nativeTimelineUnsentDelivery = (row: {
  eventId?: string;
  localEchoState?: string;
  transactionId?: string;
  sendFailureRetryable?: boolean;
  candidate?: boolean;
}): NativeTimelineUnsentDelivery | undefined => {
  const state = isNativeTimelineLocalEchoState(row.localEchoState) ? row.localEchoState : undefined;
  const transactionId = row.transactionId;
  // An unknown cause (older snapshot) keeps Retry, as before.
  const retryable = row.sendFailureRetryable !== false;
  if (state === 'sending') {
    return {
      status: 'sending',
      accessibleName: 'Sending',
      wedged: false,
      retryable: false,
      transactionId,
    };
  }
  if (state === 'failed') {
    return {
      status: 'failed',
      accessibleName: 'Failed to send',
      wedged: false,
      retryable,
      transactionId,
    };
  }
  if (state === 'wedged') {
    return {
      status: 'failed',
      accessibleName: 'Failed to send',
      wedged: true,
      retryable,
      transactionId,
    };
  }
  if (state === 'sent' || row.eventId || !row.candidate) return undefined;
  return {
    status: 'unsent',
    accessibleName: 'Not sent',
    wedged: false,
    retryable: false,
    transactionId,
  };
};

/**
 * `matrix_local_echo_discard` resolves `true` when the SDK dropped the unsent
 * request and `false` when it no longer held it because it was already sent.
 * Anything else is an invalid readback, not a discard.
 */
export type NativeLocalEchoDiscardOutcome = 'discarded' | 'already_sent';

export const nativeLocalEchoDiscardOutcome = (value: unknown): NativeLocalEchoDiscardOutcome => {
  if (value === true) return 'discarded';
  if (value === false) return 'already_sent';
  throw new Error('invalid discard readback');
};

export async function discardNativeLocalEcho(
  roomId: string,
  transactionId: string
): Promise<NativeLocalEchoDiscardOutcome> {
  const result = await invokeDesktopWithAvailability<boolean>('matrix_local_echo_discard', {
    roomId,
    transactionId,
  });
  if (!result.available) {
    throw new Error('unavailable');
  }
  return nativeLocalEchoDiscardOutcome(result.value);
}

export async function retryNativeLocalEcho(roomId: string, transactionId: string): Promise<void> {
  const result = await invokeDesktopWithAvailability('matrix_local_echo_retry', {
    roomId,
    transactionId,
  });
  if (!result.available) {
    throw new Error('unavailable');
  }
}

export const NATIVE_LOCAL_ECHO_DISCARD_FAILED = 'The unsent message could not be discarded.';
export const NATIVE_LOCAL_ECHO_ALREADY_SENT =
  'This message was already sent and could not be discarded.';
export const NATIVE_LOCAL_ECHO_RETRY_FAILED = 'The unsent message could not be retried.';

export type NativeLocalEchoActionResult =
  { status: 'ignored' } | { status: 'done' } | { status: 'error'; message: string };

const nativeLocalEchoActionKey = (roomId: string, transactionId: string): string =>
  `${roomId}\u0000${transactionId}`;

/**
 * One discard or retry per unsent row at a time. A second press while the
 * first is in flight is ignored, so a double click cannot report a false
 * "could not be discarded" after the first press already removed the echo.
 */
export class NativeLocalEchoActionGuard {
  private readonly pending = new Set<string>();

  isPending(roomId: string, transactionId: string): boolean {
    return this.pending.has(nativeLocalEchoActionKey(roomId, transactionId));
  }

  async run(
    roomId: string,
    transactionId: string,
    action: () => Promise<NativeLocalEchoActionResult>
  ): Promise<NativeLocalEchoActionResult> {
    const key = nativeLocalEchoActionKey(roomId, transactionId);
    if (this.pending.has(key)) return { status: 'ignored' };
    this.pending.add(key);
    try {
      return await action();
    } finally {
      this.pending.delete(key);
    }
  }
}

export const runNativeLocalEchoDiscard = (
  guard: NativeLocalEchoActionGuard,
  roomId: string,
  transactionId: string,
  discard: typeof discardNativeLocalEcho = discardNativeLocalEcho
): Promise<NativeLocalEchoActionResult> =>
  guard.run(roomId, transactionId, async () => {
    try {
      const outcome = await discard(roomId, transactionId);
      return outcome === 'discarded'
        ? { status: 'done' }
        : { status: 'error', message: NATIVE_LOCAL_ECHO_ALREADY_SENT };
    } catch {
      return { status: 'error', message: NATIVE_LOCAL_ECHO_DISCARD_FAILED };
    }
  });

export const runNativeLocalEchoRetry = (
  guard: NativeLocalEchoActionGuard,
  roomId: string,
  transactionId: string,
  retry: typeof retryNativeLocalEcho = retryNativeLocalEcho
): Promise<NativeLocalEchoActionResult> =>
  guard.run(roomId, transactionId, async () => {
    try {
      await retry(roomId, transactionId);
      return { status: 'done' };
    } catch {
      return { status: 'error', message: NATIVE_LOCAL_ECHO_RETRY_FAILED };
    }
  });

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
  senders?: NativeTimelineReactionSender[];
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

/** Earliest own terminal reaction on an approval prompt; `decidedAt` is epoch ms. */
export type NativeTimelineAgentApprovalDecision = {
  decision: SynaraAgentApprovalHistoryDecision;
  decidedAt?: number;
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
    /** This account's own decision on the approval prompt, projected by Core. */
    agentApprovalDecision?: NativeTimelineAgentApprovalDecision;
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
    canRedactOwn?: boolean;
    canRedactOther?: boolean;
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

/**
 * Where a routed event lives, as Core resolved it for a `focused` open. A
 * `thread` reply is never shown in the room stream: Core returns the live room
 * (at the root when it is in the live window) and the shell opens the thread.
 * `missing` and `unreachable` also return the live room, so the shell says why.
 */
export type NativeTimelineEventRoute =
  | { kind: 'room' }
  | { kind: 'thread'; root_event_id: string }
  | { kind: 'missing' }
  | { kind: 'unreachable' };

export type NativeTimelineOpenReadback = {
  schemaVersion: number;
  streamId: string;
  /** The placement selected by the native owner for this open. */
  position: NativeTimelinePosition;
  snapshot: NativeTimelineViewSnapshot;
  /** Present only for `focused` opens. */
  focusRoute?: NativeTimelineEventRoute;
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
  /**
   * Keep the current provider until a focused snapshot contains the read
   * target. Core may resolve a marker that has no row of its own (reaction,
   * edit, redaction) to its nearest rendered neighbour; the adopted anchor is
   * returned so the caller can retire the pending marker. `undefined` means
   * the request was superseded and nothing was adopted.
   */
  restoreLastRead: (eventId: string) => Promise<string | undefined>;
  /** How the current `focused` open resolved its event, once it is ready. */
  focusRoute?: { eventId: string; route: NativeTimelineEventRoute };
  /**
   * A stream update was missed. The view keeps the rows it has (`resyncing`)
   * while a snapshot is fetched at once; `failed` means that fetch did not
   * come back, so the caller should offer `reload`.
   */
  streamGap: NativeTimelineStreamGap;
  /** Close the current stream and open the same position again. */
  reload: () => void;
};

export type NativeTimelineStreamGap = 'none' | 'resyncing' | 'failed';

/**
 * Stream metadata without rows. Pagination and read-state replies carry only
 * this: the rows (and the page or read state itself) reach the presenter as
 * delta batches, so a reply never ships the window across IPC again.
 */
export type NativeTimelineViewMetadata = Omit<NativeTimelineViewSnapshot, 'rows'>;

type NativeTimelineReadStateReadback = {
  receiptSent?: boolean;
  acknowledgedEventId?: string;
  view: NativeTimelineViewMetadata;
};

/** A safety-net probe that found the stream still at the revision it named. */
type NativeTimelineSnapshotUnchanged = { revision: number };

/** Whether a reply's metadata belongs to the stream the presenter shows. */
const isSameNativeTimelineStream = (
  current: NativeTimelineViewSnapshot | undefined,
  metadata: NativeTimelineViewMetadata
): boolean =>
  Boolean(
    current &&
    metadata.schemaVersion === TIMELINE_VIEW_SCHEMA_VERSION &&
    metadata.sessionGeneration === current.sessionGeneration &&
    metadata.roomId === current.roomId
  );

const isValidIndex = (index: number, length: number, allowEnd = false): boolean =>
  Number.isInteger(index) && index >= 0 && index < length + (allowEnd ? 1 : 0);

/**
 * Pagination and read-state readbacks can lag a live delta. Callers first
 * confirm stream ownership; equal-or-older snapshots then keep the newer view.
 */
/**
 * With the delta channel live, the snapshot poll is only a safety net for a
 * dropped batch. Without deltas it is the update path.
 */
export const NATIVE_TIMELINE_SAFETY_NET_POLL_MS = 10_000;
export const NATIVE_TIMELINE_POLL_WITHOUT_DELTAS_MS = 750;
/**
 * One early safety-net poll after open. A batch dropped around the open
 * readback is the likeliest gap, so check it soon, then fall back to the slow
 * cadence.
 */
export const NATIVE_TIMELINE_FIRST_SAFETY_NET_POLL_MS = 1_500;

/** Delay before the next snapshot poll, given whether deltas are live. */
export const nativeTimelinePollDelay = (deltasLive: boolean, firstPoll: boolean): number => {
  if (!deltasLive) return NATIVE_TIMELINE_POLL_WITHOUT_DELTAS_MS;
  return firstPoll ? NATIVE_TIMELINE_FIRST_SAFETY_NET_POLL_MS : NATIVE_TIMELINE_SAFETY_NET_POLL_MS;
};

/**
 * After a send, a view Core opened at the live bottom already receives the
 * message as a delta, so only the tail needs placing. Any other placement
 * (history, a focused event, last read) still asks Core to jump to latest.
 */
export const shouldPlaceLatestLocally = (position: NativeTimelinePosition | undefined): boolean =>
  position?.kind === 'live_bottom';

/**
 * True when `next` carries nothing new. The revision counts SDK timeline
 * diffs only; position, pagination, and read state can change without one, so
 * an equal revision alone is not proof. Compare the payload too, but only
 * when the revisions already match.
 */
export const isUnchangedNativeTimelineSnapshot = (
  current: NativeTimelineViewSnapshot,
  next: NativeTimelineViewSnapshot
): boolean =>
  next.revision === current.revision && JSON.stringify(next) === JSON.stringify(current);

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

  if (batch.ops.length === 0) {
    return {
      ...snapshot,
      revision: batch.revision,
      rows: snapshot.rows,
      ...(batch.readState ? { readState: batch.readState } : {}),
      ...(batch.pagination ? { pagination: batch.pagination } : {}),
      ...(batch.pinnedEventIds !== undefined ? { pinnedEventIds: batch.pinnedEventIds } : {}),
    };
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

export type NativeTimelineViewDeltaBatchApply = {
  snapshot: NativeTimelineViewSnapshot;
  /** True when a later batch was a gap or invalid op; `snapshot` is the prefix. */
  gap: boolean;
};

/**
 * Apply several stream batches in revision order. Stale/duplicate revisions are
 * skipped. A missing revision or invalid op fails closed rather than guessed,
 * but a successful prefix is kept so a later fill can recover.
 */
export const applyNativeTimelineViewDeltaBatches = (
  snapshot: NativeTimelineViewSnapshot,
  batches: readonly NativeTimelineViewDeltaBatch[]
): NativeTimelineViewDeltaBatchApply => {
  const sorted = [...batches].sort((left, right) => left.revision - right.revision);
  let current = snapshot;
  for (const batch of sorted) {
    if (batch.revision <= current.revision) continue;
    const applied = applyNativeTimelineViewDelta(current, batch);
    if (!applied) return { snapshot: current, gap: true };
    current = applied;
  }
  return { snapshot: current, gap: false };
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
    | { kind: 'focused'; eventId: string }
    | { kind: 'thread'; rootEventId: string };
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
        : position.kind === 'thread'
          ? { kind: 'thread' as const, root_event_id: position.rootEventId }
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

/** One timeline view's delta channel (L20). */
export type NativeTimelineDeltaSubscription = {
  /** Passed to the opens and jumps whose stream this view shows. */
  id: number;
  /** Stops delivery and closes the channel on the Rust side. */
  close: () => void;
};

export type NativeTimelineDeltaChannelDeps = {
  createChannel: (onBatch: (batch: NativeTimelineViewDeltaBatch) => void) => unknown;
  invoke: (
    command: string,
    args?: Record<string, unknown>
  ) => Promise<DesktopInvokeResult<unknown>>;
};

const defaultDeltaChannelDeps: NativeTimelineDeltaChannelDeps = {
  createChannel: (onBatch) => new Channel<NativeTimelineViewDeltaBatch>(onBatch),
  invoke: (command, args) => invokeDesktopWithAvailability(command, args),
};

/**
 * Open a view's delta channel before its first stream. Rust sends the
 * room's batches down it in emit order, and only this view's once its open
 * claims a stream. Resolves undefined when no channel opens; the view then
 * polls its snapshot instead.
 */
export async function subscribeNativeTimelineDeltas(
  roomId: string,
  onBatch: (batch: NativeTimelineViewDeltaBatch) => void,
  deps: NativeTimelineDeltaChannelDeps = defaultDeltaChannelDeps
): Promise<NativeTimelineDeltaSubscription | undefined> {
  let open = true;
  let channel: unknown;
  try {
    channel = deps.createChannel((batch) => {
      if (open) onBatch(batch);
    });
  } catch {
    return undefined;
  }
  const result = await deps
    .invoke('desktop_timeline_subscribe', { roomId, channel })
    .catch(() => undefined);
  const id = result?.available ? result.value : undefined;
  if (typeof id !== 'number') {
    open = false;
    return undefined;
  }
  return {
    id,
    close: () => {
      if (!open) return;
      open = false;
      void deps
        .invoke('desktop_timeline_unsubscribe', { subscriptionId: id })
        .catch(() => undefined);
    },
  };
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
  const threadRootEventId =
    input?.position.kind === 'thread' ? input.position.rootEventId : undefined;
  const normalPosition = input?.position.kind === 'normal' ? input.position : undefined;
  const nativeRequest = useMemo(() => {
    if (!roomId || !positionKind) return undefined;
    return toNativeTimelineOpenRequest({
      roomId,
      position:
        positionKind === 'focused' && focusedEventId
          ? { kind: 'focused', eventId: focusedEventId }
          : positionKind === 'thread' && threadRootEventId
            ? { kind: 'thread', rootEventId: threadRootEventId }
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
    threadRootEventId,
    normalPosition?.atBottom,
    normalPosition?.liveTailEventId,
    normalPosition?.restoredAnchorEventId,
    normalPosition?.updatedAtMs,
    positionKind,
    roomId,
  ]);
  const [state, setState] = useState<NativeTimelineViewState>({ status: 'unavailable' });
  const [focusRoute, setFocusRoute] = useState<NativeTimelineViewController['focusRoute']>();
  const [streamGap, setStreamGap] = useState<NativeTimelineStreamGap>('none');
  const [reloadToken, setReloadToken] = useState(0);
  const reload = useCallback(() => setReloadToken((token) => token + 1), []);
  const streamIdRef = useRef<string | undefined>(undefined);
  /** This view's delta channel; opens and jumps claim their stream for it. */
  const deltaSubscriptionIdRef = useRef<number | undefined>(undefined);
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
    if (isUnchangedNativeTimelineSnapshot(current, next)) {
      // Accepted, but identical: keep row identities so memoized rows do not
      // re-render on every safety-net poll.
      return true;
    }
    snapshotRef.current = next;
    setState({
      status: 'ready',
      snapshot: next,
      selectedPosition: selectedPositionRef.current ?? next.position,
    });
    return true;
  }, []);

  const paginate = useCallback(async (direction: 'backwards' | 'forwards') => {
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
    let result: DesktopInvokeResult<NativeTimelineViewMetadata>;
    try {
      result = await invokeDesktopWithAvailability<NativeTimelineViewMetadata>(
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
    // The page's rows and state arrive as delta batches; the reply only
    // confirms the page ran on this stream.
    if (!isSameNativeTimelineStream(snapshotRef.current, result.value)) {
      setState({
        status: 'error',
        error: new Error('Native timeline pagination lost synchronization.'),
      });
      throw new Error('Native timeline pagination lost synchronization.');
    }
  }, []);

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
          { request: { streamId, ...request, publicReceipt: readReceiptsArePublic() } }
        );
      } catch (error) {
        if (superseded()) return;
        throw error;
      }
      if (superseded()) return;
      if (!result.available || !result.value) {
        throw new Error('Native timeline read action is unavailable.');
      }
      // The read state reaches the presenter as a delta batch; the reply only
      // confirms the action ran on this stream.
      if (isSameNativeTimelineStream(snapshotRef.current, result.value.view)) return;
      setState({
        status: 'error',
        error: new Error('Native timeline read state lost synchronization.'),
      });
      throw new Error('Native timeline read state lost synchronization.');
    },
    []
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
        { request: { streamId }, subscriptionId: deltaSubscriptionIdRef.current }
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
    // Core keeps a stream that already shows the live bottom and answers
    // with that same stream: its deltas kept flowing to this presenter, so
    // the readback must not roll back rows newer than its revision.
    const kept =
      result.value.streamId === streamId &&
      snapshotRef.current !== undefined &&
      snapshotRef.current.revision >= snapshot.revision;
    const adopted = kept && snapshotRef.current ? snapshotRef.current : snapshot;
    streamIdRef.current = result.value.streamId;
    selectedPositionRef.current = result.value.position;
    snapshotRef.current = adopted;
    setState({
      status: 'ready',
      snapshot: adopted,
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
            subscriptionId: deltaSubscriptionIdRef.current,
          }
        );
      } catch (error) {
        finishOpen(buffer);
        if (superseded()) return undefined;
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
        return undefined;
      }
      const openedAnchor =
        opened?.position.kind === 'focused' ? opened.position.target_event_id : undefined;
      if (
        !result.available ||
        !opened ||
        !snapshot ||
        opened.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        snapshot.sessionGeneration !== current.sessionGeneration ||
        snapshot.roomId !== current.roomId ||
        openedAnchor === undefined ||
        snapshot.position.kind !== 'focused' ||
        snapshot.position.target_event_id !== openedAnchor ||
        !snapshot.rows.some(
          (row) => (row.kind === 'sticker' ? row.event.eventId : row.eventId) === openedAnchor
        )
      ) {
        discard();
        throw new Error(
          openedAnchor !== undefined && openedAnchor !== eventId
            ? 'The last-read position could not be placed. Try again.'
            : 'The last-read message is not available in this context. Try again.'
        );
      }
      // Each Core open owns its own subscription. Retire the previous provider
      // only after the adopted anchor has been confirmed in its replacement.
      streamIdRef.current = opened.streamId;
      snapshotRef.current = snapshot;
      selectedPositionRef.current = opened.position;
      setState({ status: 'ready', snapshot, selectedPosition: opened.position });
      void invokeDesktopWithAvailability('matrix_timeline_close', {
        request: { streamId },
      }).catch(() => undefined);
      return openedAnchor;
    },
    [beginOpen, finishOpen]
  );

  useEffect(() => {
    navigationRevisionRef.current += 1;
    streamIdRef.current = undefined;
    deltaSubscriptionIdRef.current = undefined;
    snapshotRef.current = undefined;
    selectedPositionRef.current = undefined;
    setFocusRoute(undefined);
    setStreamGap('none');
    pendingOpenRef.current?.cancel();
    pendingOpenRef.current = undefined;
    if (!nativeRequest || !isSynaraDesktop()) {
      setState({ status: 'unavailable' });
      return undefined;
    }

    let disposed = false;
    let deltas: NativeTimelineDeltaSubscription | undefined;
    let stopPoll: (() => void) | undefined;
    let resyncInFlight = false;
    // Assigned below; a gap found while applying deltas asks for a snapshot.
    let resyncAfterGap: () => void = () => undefined;
    const COALESCE_QUEUE_LIMIT = 128;
    let coalesceQueued: NativeTimelineViewDeltaBatch[] = [];
    let coalesceFrame = 0;
    const flushCoalescedBatches = () => {
      coalesceFrame = 0;
      const queued = coalesceQueued;
      coalesceQueued = [];
      if (disposed || queued.length === 0) return;
      const next = snapshotRef.current;
      const active: NativeTimelineViewDeltaBatch[] = [];
      for (const batch of queued) {
        if (batch.streamId !== streamIdRef.current || !next) {
          pendingOpenRef.current?.add(batch);
          continue;
        }
        active.push(batch);
      }
      if (!next || active.length === 0) return;
      const applied = applyNativeTimelineViewDeltaBatches(next, active);
      if (applied.gap) {
        // Keep showing the rows that did apply, and fetch the full snapshot
        // now rather than at the next safety-net poll. Later batches gap too
        // until it lands; the snapshot's newer revision then supersedes them.
        if (applied.snapshot !== snapshotRef.current) {
          snapshotRef.current = applied.snapshot;
          setState({
            status: 'ready',
            snapshot: applied.snapshot,
            selectedPosition: selectedPositionRef.current ?? applied.snapshot.position,
          });
        }
        resyncAfterGap();
        return;
      }
      if (applied.snapshot === snapshotRef.current) return;
      snapshotRef.current = applied.snapshot;
      setState({
        status: 'ready',
        snapshot: applied.snapshot,
        selectedPosition: selectedPositionRef.current ?? applied.snapshot.position,
      });
    };
    const applyBatch = (batch: NativeTimelineViewDeltaBatch) => {
      if (disposed || batch.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION) return;
      if (batch.streamId !== streamIdRef.current || !snapshotRef.current) {
        pendingOpenRef.current?.add(batch);
        return;
      }
      coalesceQueued.push(batch);
      // rAF pauses while the document is hidden; bound the queue so a busy
      // room cannot accumulate unboundedly before the next visible frame.
      if (coalesceQueued.length >= COALESCE_QUEUE_LIMIT) {
        if (coalesceFrame !== 0) window.cancelAnimationFrame(coalesceFrame);
        flushCoalescedBatches();
        return;
      }
      if (coalesceFrame !== 0) return;
      coalesceFrame = window.requestAnimationFrame(flushCoalescedBatches);
    };

    /**
     * True when the snapshot was adopted (or already current). The safety-net
     * poll names the revision it shows, and Core answers `{ revision }` with
     * no rows while the stream has not moved; a gap resync always asks for
     * the full snapshot.
     */
    const pollSnapshot = async (probe: boolean): Promise<boolean> => {
      const streamId = streamIdRef.current;
      if (!streamId || disposed) return false;
      const revision = navigationRevisionRef.current;
      const navigationRequest = navigationRequestRef.current;
      const knownRevision = probe ? snapshotRef.current?.revision : undefined;
      const result = await invokeDesktopWithAvailability<
        NativeTimelineViewSnapshot | NativeTimelineSnapshotUnchanged
      >(
        'matrix_timeline_snapshot',
        knownRevision === undefined ? { streamId } : { streamId, knownRevision }
      ).catch(() => undefined);
      if (
        disposed ||
        streamIdRef.current !== streamId ||
        navigationRevisionRef.current !== revision ||
        navigationRequestRef.current !== navigationRequest ||
        !result?.available ||
        !result.value
      )
        return false;
      if (!('rows' in result.value)) {
        // Unchanged: still current only if no batch moved the presenter since.
        return result.value.revision === snapshotRef.current?.revision;
      }
      const snapshot = result.value;
      if (
        snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
        snapshot.roomId !== nativeRequest.roomId
      ) {
        return false;
      }
      if (!snapshotRef.current) {
        snapshotRef.current = snapshot;
        setState({
          status: 'ready',
          snapshot,
          selectedPosition: selectedPositionRef.current ?? snapshot.position,
        });
        return true;
      }
      const accepted = acceptSnapshot(snapshot);
      if (accepted) setStreamGap('none');
      return accepted;
    };

    resyncAfterGap = () => {
      setStreamGap((current) => (current === 'failed' ? current : 'resyncing'));
      if (resyncInFlight) return;
      resyncInFlight = true;
      const streamId = streamIdRef.current;
      void pollSnapshot(false).then((adopted) => {
        resyncInFlight = false;
        if (!adopted && !disposed && streamIdRef.current === streamId) setStreamGap('failed');
      });
    };

    const open = async () => {
      const buffer = beginOpen(nativeRequest.roomId);
      setState({ status: 'loading' });
      try {
        deltas = await subscribeNativeTimelineDeltas(nativeRequest.roomId, applyBatch);
        if (disposed) {
          deltas?.close();
          return;
        }
        deltaSubscriptionIdRef.current = deltas?.id;
        const result = await invokeDesktopWithAvailability<NativeTimelineOpenReadback>(
          'matrix_timeline_open',
          { request: nativeRequest, subscriptionId: deltas?.id }
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
        const requested = nativeRequest.position;
        setFocusRoute(
          requested.kind === 'focused' && readback.focusRoute
            ? { eventId: requested.event_id, route: readback.focusRoute }
            : undefined
        );
        if (!disposed) {
          // Paused while the window is hidden; showing it again polls at once (D11).
          const deltasLive = Boolean(deltas);
          stopPoll = startVisibleOnlyPoll(
            () => {
              if (!disposed) void pollSnapshot(true);
            },
            (firstPoll) => nativeTimelinePollDelay(deltasLive, firstPoll)
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
      coalesceQueued = [];
      if (coalesceFrame !== 0) window.cancelAnimationFrame(coalesceFrame);
      pendingOpenRef.current?.cancel();
      pendingOpenRef.current = undefined;
      navigationRevisionRef.current += 1;
      const streamId = streamIdRef.current;
      if (streamId) {
        void invokeDesktopWithAvailability('matrix_timeline_close', {
          request: { streamId },
        });
      }
      stopPoll?.();
      deltas?.close();
    };
    // `reloadToken` reopens the same request on demand (Retry, Reload).
  }, [acceptSnapshot, beginOpen, finishOpen, nativeRequest, reloadToken]);

  return {
    state,
    paginate,
    setReadState,
    followLive,
    jumpLatest,
    restoreLastRead,
    focusRoute,
    streamGap,
    reload,
  };
};

export const nativeTimelineMediaSrc = (handle: NativeTimelineMediaHandle): string | undefined =>
  convertDesktopFileSrc(handle.handleId, 'synara-media');

/**
 * Inline image source sized to the timeline box. A server thumbnail replaces
 * the original only when the original is (or may be) larger than the box at
 * this device pixel ratio. GIFs keep the original so they still animate.
 */
export const nativeTimelineInlineImageContentUri = (
  handle: NativeTimelineMediaHandle,
  boxCssPx: number,
  scale: number = devicePixelScale()
): string => {
  if (handle.mimeType?.toLowerCase() === 'image/gif') return handle.handleId;
  const deviceBox = boxCssPx * scale;
  const fitsBox =
    typeof handle.width === 'number' &&
    typeof handle.height === 'number' &&
    handle.width <= deviceBox &&
    handle.height <= deviceBox;
  if (fitsBox) return handle.handleId;
  return nativeThumbnailContentUri(handle.handleId, boxCssPx, boxCssPx, 'scale', scale);
};

export const nativeTimelineInlineImageSrc = (
  handle: NativeTimelineMediaHandle,
  boxCssPx: number
): string | undefined =>
  convertDesktopFileSrc(nativeTimelineInlineImageContentUri(handle, boxCssPx), 'synara-media');
