import { useCallback, useEffect, useRef, useState } from 'react';
import {
  invokeDesktopWithAvailability,
  isSynaraDesktop,
  listen,
  type DesktopUnlisten,
} from '../../utils/desktop';

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
};

type NativeTimelineEventRowBase = {
  itemId: string;
  eventId?: string;
  senderId: string;
  senderName: string;
  originServerTs: number;
  capabilities: NativeTimelineRowCapabilities;
};

type NativeTimelineMessageRow = NativeTimelineEventRowBase & {
  kind: 'message';
  body: string;
  formattedBody?: string;
  messageType?: string;
  edited: boolean;
  reactions?: Array<{ key: string; count: number; own?: boolean }>;
};

type NativeTimelineMediaHandle = {
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
};

type NativeTimelinePollRow = NativeTimelineEventRowBase & {
  kind: 'poll';
  question: string;
  closed: boolean;
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

type NativeTimelineSimpleRow = {
  kind:
    | 'redacted'
    | 'encrypted_unavailable'
    | 'other'
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
    ownReadEventId?: string;
    unreadAnchorEventId?: string;
    isMarkedUnread: boolean;
  };
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
};

type NativeTimelineOpenReadback = {
  schemaVersion: number;
  streamId: string;
  position: unknown;
  snapshot: NativeTimelineViewSnapshot;
};

export type NativeTimelineViewState =
  | { status: 'unavailable' | 'loading'; snapshot?: undefined; error?: undefined }
  | { status: 'ready'; snapshot: NativeTimelineViewSnapshot; error?: undefined }
  | { status: 'error'; snapshot?: undefined; error: Error };

export type NativeTimelineViewController = {
  state: NativeTimelineViewState;
  paginate: (direction: 'backwards' | 'forwards') => Promise<void>;
  setReadState: (action: 'mark_read' | 'mark_unread') => Promise<void>;
};

type NativeTimelineReadStateReadback = {
  snapshot: NativeTimelineViewSnapshot;
};

const isValidIndex = (index: number, length: number, allowEnd = false): boolean =>
  Number.isInteger(index) && index >= 0 && index < length + (allowEnd ? 1 : 0);

/**
 * Applies one exact native stream update. Invalid operations and revision gaps
 * are rejected rather than guessed at or repaired with a JS timeline fetch.
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

  return { ...snapshot, revision: batch.revision, rows };
};

export type NativeTimelineOpenInput = {
  roomId: string;
  position: { kind: 'live_bottom' } | { kind: 'unread' } | { kind: 'focused'; eventId: string };
};

const toNativeTimelineOpenRequest = (input: NativeTimelineOpenInput) => ({
  roomId: input.roomId,
  position:
    input.position.kind === 'focused'
      ? { kind: 'focused', event_id: input.position.eventId }
      : input.position,
});

/**
 * Opens one native timeline view after registering for its native delta event.
 * This hook is deliberately not an activation switch: until the full presenter
 * and action/media routes exist, RoomTimeline remains the active owner.
 */
export const useNativeTimelineView = (
  input: NativeTimelineOpenInput | undefined
): NativeTimelineViewController => {
  const [state, setState] = useState<NativeTimelineViewState>({ status: 'unavailable' });
  const streamIdRef = useRef<string | undefined>(undefined);
  const snapshotRef = useRef<NativeTimelineViewSnapshot | undefined>(undefined);
  const earlyBatchesRef = useRef<NativeTimelineViewDeltaBatch[]>([]);

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
    setState({ status: 'ready', snapshot: next });
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
    const result = await invokeDesktopWithAvailability<NativeTimelineViewSnapshot>(
      'matrix_timeline_paginate',
      { request: { streamId, direction } }
    );
    if (!result.available || !result.value || !acceptSnapshot(result.value)) {
      setState({ status: 'error', error: new Error('Native timeline pagination lost synchronization.') });
      throw new Error('Native timeline pagination lost synchronization.');
    }
  }, [acceptSnapshot]);

  const setReadState = useCallback(async (action: 'mark_read' | 'mark_unread') => {
    const streamId = streamIdRef.current;
    const snapshot = snapshotRef.current;
    const permitted = action === 'mark_read' ? snapshot?.capabilities.markRead : snapshot?.capabilities.markUnread;
    if (!streamId || !snapshot || !permitted) {
      throw new Error('Native timeline read action is unavailable.');
    }
    const result = await invokeDesktopWithAvailability<NativeTimelineReadStateReadback>(
      'matrix_timeline_set_read_state',
      { request: { streamId, action } }
    );
    if (!result.available || !result.value || !acceptSnapshot(result.value.snapshot)) {
      setState({ status: 'error', error: new Error('Native timeline read state lost synchronization.') });
      throw new Error('Native timeline read state lost synchronization.');
    }
  }, [acceptSnapshot]);

  useEffect(() => {
    streamIdRef.current = undefined;
    snapshotRef.current = undefined;
    earlyBatchesRef.current = [];
    if (!input || !isSynaraDesktop()) {
      setState({ status: 'unavailable' });
      return undefined;
    }

    let disposed = false;
    let unlisten: DesktopUnlisten | undefined;
    const applyBatch = (batch: NativeTimelineViewDeltaBatch) => {
      if (disposed || batch.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION) return;
      if (!streamIdRef.current) {
        earlyBatchesRef.current.push(batch);
        return;
      }
      if (batch.streamId !== streamIdRef.current || !snapshotRef.current) return;
      const next = applyNativeTimelineViewDelta(snapshotRef.current, batch);
      if (!next) {
        setState({ status: 'error', error: new Error('Native timeline stream lost synchronization.') });
        return;
      }
      snapshotRef.current = next;
      setState({ status: 'ready', snapshot: next });
    };

    const open = async () => {
      setState({ status: 'loading' });
      try {
        unlisten = await listen<NativeTimelineViewDeltaBatch>(
          NATIVE_TIMELINE_VIEW_UPDATED_EVENT,
          ({ payload }) => applyBatch(payload)
        );
        if (disposed || !unlisten) {
          if (!disposed) setState({ status: 'unavailable' });
          return;
        }
        const result = await invokeDesktopWithAvailability<NativeTimelineOpenReadback>(
          'matrix_timeline_open',
          { request: toNativeTimelineOpenRequest(input) }
        );
        if (disposed) return;
        if (!result.available || !result.value) {
          setState({ status: 'unavailable' });
          return;
        }
        const readback = result.value;
        if (
          readback.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
          readback.snapshot.schemaVersion !== TIMELINE_VIEW_SCHEMA_VERSION ||
          readback.snapshot.roomId !== input.roomId
        ) {
          setState({ status: 'error', error: new Error('Unsupported native timeline schema.') });
          return;
        }
        streamIdRef.current = readback.streamId;
        let snapshot = readback.snapshot;
        for (const batch of earlyBatchesRef.current) {
          if (batch.streamId !== readback.streamId) continue;
          const next = applyNativeTimelineViewDelta(snapshot, batch);
          if (!next) {
            setState({ status: 'error', error: new Error('Native timeline stream lost synchronization.') });
            return;
          }
          snapshot = next;
        }
        earlyBatchesRef.current = [];
        snapshotRef.current = snapshot;
        setState({ status: 'ready', snapshot });
      } catch (error) {
        if (!disposed) {
          setState({
            status: 'error',
            error: error instanceof Error ? error : new Error('Native timeline open failed.'),
          });
        }
      }
    };

    void open();
    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, [input?.position.kind, input?.position.kind === 'focused' ? input.position.eventId : undefined, input?.roomId]);

  return { state, paginate, setReadState };
};
