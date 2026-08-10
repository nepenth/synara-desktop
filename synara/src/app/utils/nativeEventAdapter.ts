/**
 * SDK-neutral adapter that rebuilds Synara's structural MatrixEventReading from
 * raw room-event wire data (e.g. `mx.fetchRoomEvent` responses).
 *
 * Edit-replacement semantics mirror the js-sdk MatrixEvent.makeReplaced behavior:
 * when the event carries an `m.replace` relation in `unsigned`, `getContent()`
 * returns the edited body (`m.new_content ?? replacement.content`) and
 * `replacingEvent()` exposes the replacement. Shared by useRoomEvent and the
 * global timeline handlers so the native event factory lives in one place.
 */
import type { MatrixEventReading } from './room';

/** Relation projection mirroring MatrixEventReading.getRelation()'s shape. */
export type RoomEventRelationReading = {
  rel_type?: string;
  event_id?: string;
  key?: string;
};

/** Unsigned metadata surface read by consumers (redaction + event payload). */
export type RoomEventUnsignedReading = {
  redacted_because?: { content: { reason?: string; [key: string]: unknown } };
  [key: string]: unknown;
};

/** A room event whose edit-replacement (if any) can be inspected. */
export type RoomEventReading = MatrixEventReading & {
  replyEventId?: string;
  threadRootId?: string;
  getUnsigned(): RoomEventUnsignedReading;
  replacingEvent(): MatrixEventReading | null;
};

/** Raw wire event returned by the server event fetch (room_id is stripped). */
type WireRoomEvent = {
  event_id?: string;
  type?: string;
  sender?: string;
  state_key?: string;
  origin_server_ts?: number;
  content?: { [key: string]: any };
  unsigned?: {
    prev_content?: { [key: string]: any };
    redacted_because?: unknown;
    'm.relations'?: {
      'm.replace'?: WireRoomEvent;
      [key: string]: unknown;
    };
    [key: string]: unknown;
  };
};

type RoomEventContent = { [key: string]: any };

/**
 * Build a RoomEventReading from a raw wire event.
 */
export const eventFromWire = (raw: unknown, roomId: string): RoomEventReading => {
  const evt = (raw ?? {}) as WireRoomEvent;
  const content = evt.content ?? {};
  const unsigned = evt.unsigned ?? {};
  const relatesTo = content['m.relates_to'] as RoomEventRelationReading | undefined;

  const replaceRaw = unsigned['m.relations']?.['m.replace'];
  const replacement = replaceRaw ? eventFromWire(replaceRaw, roomId) : null;
  const effectiveContent =
    replacement?.getContent()?.['m.new_content'] ?? replacement?.getContent() ?? {};

  const relation = relatesTo && relatesTo.rel_type && relatesTo.event_id ? relatesTo : null;

  const unsignedReading = unsigned as unknown as RoomEventUnsignedReading;

  return {
    event: evt as unknown as MatrixEventReading['event'],
    replyEventId: content['m.relates_to']?.['m.in_reply_to']?.event_id,
    threadRootId: relatesTo?.rel_type === 'm.thread' ? relatesTo.event_id : undefined,
    getContent: <T extends RoomEventContent = RoomEventContent>(): T =>
      (replacement ? effectiveContent : content) as T,
    getPrevContent: () => unsigned.prev_content ?? {},
    getSender: () => evt.sender,
    getType: () => evt.type ?? '',
    getStateKey: () => evt.state_key,
    getTs: () => evt.origin_server_ts ?? 0,
    getId: () => evt.event_id,
    getRoomId: () => roomId,
    isRedacted: () => Boolean(unsigned.redacted_because),
    isSending: () => false,
    getRelation: () => relation,
    getUnsigned: () => unsignedReading,
    replacingEvent: () => replacement,
  };
};
