import { useCallback, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useMatrixClient } from './useMatrixClient';
import type { MatrixEventReading } from '../utils/room';
import type { EventedRoomReading } from '../utils/roomEvents';

/** Relation projection mirroring MatrixEventReading.getRelation()'s shape. */
type RoomEventRelationReading = {
  rel_type?: string;
  event_id?: string;
  key?: string;
};

/** Room projection with a local event resolver (real js-sdk Room satisfies this). */
type RoomEventSourceReading = EventedRoomReading & {
  findEventById(eventId: string): MatrixEventReading | undefined;
};

/**
 * SDK-neutral reading of a room event with edit resolution.
 * Mirrors RoomPinMenu's PinEventReading so consumers can keep rendering it.
 */
type RoomEventUnsignedReading = {
  redacted_because?: { content: { reason?: string; [key: string]: unknown } };
  [key: string]: unknown;
};

type RoomEventReading = MatrixEventReading & {
  replyEventId?: string;
  getUnsigned(): RoomEventUnsignedReading;
  replacingEvent(): MatrixEventReading | null;
};

/** Raw wire event returned by mx.fetchRoomEvent (room_id is stripped server-side). */
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
 *
 * Preserves js-sdk `makeReplaced` semantics: when `unsigned['m.relations']['m.replace']`
 * is present, `getContent()` returns the replacement's `m.new_content` so consumers
 * (Reply's edited body) see the edited content, and `replacingEvent()` exposes the
 * replacement reading.
 */
const eventFromWire = (raw: unknown, roomId: string): RoomEventReading => {
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

const useFetchEvent = (room: RoomEventSourceReading, eventId: string) => {
  const mx = useMatrixClient();

  const fetchEventCallback = useCallback(async () => {
    const evt = await mx.fetchRoomEvent(room.roomId, eventId);
    if (!evt) {
      throw new Error('Room event not found');
    }
    return eventFromWire(evt, room.roomId);
  }, [mx, room.roomId, eventId]);

  return fetchEventCallback;
};

/**
 *
 * @param room
 * @param eventId
 * @returns `RoomEventReading`, `undefined` means loading, `null` means failure
 */
export const useRoomEvent = (
  room: RoomEventSourceReading,
  eventId: string,
  getLocally?: () => MatrixEventReading | undefined
) => {
  const event = useMemo(() => {
    if (getLocally) return getLocally();
    return room.findEventById(eventId);
  }, [room, eventId, getLocally]);

  const fetchEvent = useFetchEvent(room, eventId);

  const { data, error } = useQuery({
    enabled: event === undefined,
    queryKey: [room.roomId, eventId],
    queryFn: fetchEvent,
    staleTime: Infinity,
    gcTime: 60 * 60 * 1000, // 1hour
  });

  if (event) return event as RoomEventReading;
  if (data) return data;
  if (error) return null;

  return undefined;
};
