/**
 * Timeline item DTOs — product virtualization rows (exhaustive tagged union).
 */

import type { EventId, RoomId, TimelineItemId, UserId } from './ids';
import { parseRelationRef, type RelationRef } from './relation';
import {
  hasForbiddenWireFields,
  isObject,
  optBoolean,
  optString,
  reqNumber,
  reqString,
} from './parseUtil';

export const LOCAL_ECHO_STATES = ['sending', 'sent', 'failed', 'cancelled'] as const;
export type LocalEchoState = typeof LOCAL_ECHO_STATES[number];
const LOCAL_ECHO_SET = new Set<string>(LOCAL_ECHO_STATES);

export function isLocalEchoState(value: unknown): value is LocalEchoState {
  return typeof value === 'string' && LOCAL_ECHO_SET.has(value);
}

export const TIMELINE_ITEM_KINDS = [
  'message',
  'state',
  'membership',
  'reaction_summary',
  'redacted',
  'encrypted_unavailable',
  'date_separator',
  'read_marker',
  'other',
] as const;

export type TimelineItemKind = typeof TIMELINE_ITEM_KINDS[number];

export type TimelineMessageItem = {
  kind: 'message';
  itemId: TimelineItemId;
  eventId: EventId;
  roomId: RoomId;
  sender: UserId;
  originServerTs: number;
  body: string;
  msgtype?: string;
  relatesTo?: RelationRef;
  localEchoState?: LocalEchoState;
  isEdited?: boolean;
  isRedacted?: boolean;
  threadRootId?: EventId;
};

export type TimelineStateItem = {
  kind: 'state';
  itemId: TimelineItemId;
  eventId: EventId;
  roomId: RoomId;
  sender: UserId;
  originServerTs: number;
  stateKey: string;
  stateType: string;
  summary?: string;
};

export type TimelineMembershipItem = {
  kind: 'membership';
  itemId: TimelineItemId;
  eventId: EventId;
  roomId: RoomId;
  sender: UserId;
  originServerTs: number;
  targetUserId: UserId;
  summary: string;
};

export type TimelineReactionSummaryItem = {
  kind: 'reaction_summary';
  itemId: TimelineItemId;
  eventId: EventId;
  roomId: RoomId;
  key: string;
  count: number;
  me?: boolean;
};

export type TimelineRedactedItem = {
  kind: 'redacted';
  itemId: TimelineItemId;
  eventId: EventId;
  roomId: RoomId;
  redactedBy?: EventId;
};

export type TimelineEncryptedUnavailableItem = {
  kind: 'encrypted_unavailable';
  itemId: TimelineItemId;
  eventId: EventId;
  roomId: RoomId;
  reason?: string;
};

export type TimelineDateSeparatorItem = {
  kind: 'date_separator';
  itemId: TimelineItemId;
  dayKey: string;
};

export type TimelineReadMarkerItem = {
  kind: 'read_marker';
  itemId: TimelineItemId;
};

export type TimelineOtherItem = {
  kind: 'other';
  itemId: TimelineItemId;
  eventId?: EventId;
  type?: string;
  summary?: string;
};

export type TimelineItem =
  | TimelineMessageItem
  | TimelineStateItem
  | TimelineMembershipItem
  | TimelineReactionSummaryItem
  | TimelineRedactedItem
  | TimelineEncryptedUnavailableItem
  | TimelineDateSeparatorItem
  | TimelineReadMarkerItem
  | TimelineOtherItem;

export function parseTimelineItem(value: unknown): TimelineItem | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const kind = value.kind;
  if (typeof kind !== 'string') return null;
  const itemId = reqString(value, 'itemId');
  if (itemId === null) return null;

  switch (kind) {
    case 'message': {
      const eventId = reqString(value, 'eventId');
      const roomId = reqString(value, 'roomId');
      const sender = reqString(value, 'sender');
      const originServerTs = reqNumber(value, 'originServerTs');
      const body = reqString(value, 'body');
      const msgtype = optString(value, 'msgtype');
      const isEdited = optBoolean(value, 'isEdited');
      const isRedacted = optBoolean(value, 'isRedacted');
      const threadRootId = optString(value, 'threadRootId');
      if (
        eventId === null ||
        roomId === null ||
        sender === null ||
        originServerTs === null ||
        body === null ||
        msgtype === null ||
        isEdited === null ||
        isRedacted === null ||
        threadRootId === null
      ) {
        return null;
      }
      let relatesTo: RelationRef | undefined;
      if (value.relatesTo !== undefined) {
        const r = parseRelationRef(value.relatesTo);
        if (!r) return null;
        relatesTo = r;
      }
      let localEchoState: LocalEchoState | undefined;
      if (value.localEchoState !== undefined) {
        if (!isLocalEchoState(value.localEchoState)) return null;
        localEchoState = value.localEchoState;
      }
      return {
        kind: 'message',
        itemId,
        eventId,
        roomId,
        sender,
        originServerTs,
        body,
        msgtype,
        relatesTo,
        localEchoState,
        isEdited,
        isRedacted,
        threadRootId,
      };
    }
    case 'state': {
      const eventId = reqString(value, 'eventId');
      const roomId = reqString(value, 'roomId');
      const sender = reqString(value, 'sender');
      const originServerTs = reqNumber(value, 'originServerTs');
      const stateKey = reqString(value, 'stateKey');
      const stateType = reqString(value, 'stateType');
      const summary = optString(value, 'summary');
      if (
        eventId === null ||
        roomId === null ||
        sender === null ||
        originServerTs === null ||
        stateKey === null ||
        stateType === null ||
        summary === null
      ) {
        return null;
      }
      return {
        kind: 'state',
        itemId,
        eventId,
        roomId,
        sender,
        originServerTs,
        stateKey,
        stateType,
        summary,
      };
    }
    case 'membership': {
      const eventId = reqString(value, 'eventId');
      const roomId = reqString(value, 'roomId');
      const sender = reqString(value, 'sender');
      const originServerTs = reqNumber(value, 'originServerTs');
      const targetUserId = reqString(value, 'targetUserId');
      const summary = reqString(value, 'summary');
      if (
        eventId === null ||
        roomId === null ||
        sender === null ||
        originServerTs === null ||
        targetUserId === null ||
        summary === null
      ) {
        return null;
      }
      return {
        kind: 'membership',
        itemId,
        eventId,
        roomId,
        sender,
        originServerTs,
        targetUserId,
        summary,
      };
    }
    case 'reaction_summary': {
      const eventId = reqString(value, 'eventId');
      const roomId = reqString(value, 'roomId');
      const key = reqString(value, 'key');
      const count = reqNumber(value, 'count');
      const me = optBoolean(value, 'me');
      if (eventId === null || roomId === null || key === null || count === null || me === null) {
        return null;
      }
      return { kind: 'reaction_summary', itemId, eventId, roomId, key, count, me };
    }
    case 'redacted': {
      const eventId = reqString(value, 'eventId');
      const roomId = reqString(value, 'roomId');
      const redactedBy = optString(value, 'redactedBy');
      if (eventId === null || roomId === null || redactedBy === null) return null;
      return { kind: 'redacted', itemId, eventId, roomId, redactedBy };
    }
    case 'encrypted_unavailable': {
      const eventId = reqString(value, 'eventId');
      const roomId = reqString(value, 'roomId');
      const reason = optString(value, 'reason');
      if (eventId === null || roomId === null || reason === null) return null;
      return { kind: 'encrypted_unavailable', itemId, eventId, roomId, reason };
    }
    case 'date_separator': {
      const dayKey = reqString(value, 'dayKey');
      if (dayKey === null) return null;
      return { kind: 'date_separator', itemId, dayKey };
    }
    case 'read_marker': {
      return { kind: 'read_marker', itemId };
    }
    case 'other': {
      const eventId = optString(value, 'eventId');
      const type = optString(value, 'type');
      const summary = optString(value, 'summary');
      if (eventId === null || type === null || summary === null) return null;
      return { kind: 'other', itemId, eventId, type, summary };
    }
    default:
      return null;
  }
}
