/**
 * R0.3 residual / REV-005: topic-typed snapshot/delta body validation.
 *
 * Bodies remain JSON objects on the wire. This module binds each stream topic
 * to owned DTO containers (via matrix-dto parsers) and rejects secret-like
 * fields and media-like numeric byte arrays at the envelope boundary.
 */

import {
  FORBIDDEN_WIRE_FIELD_NAMES,
  hasForbiddenWireFields,
  isObject,
  parseNotificationCandidate,
  parseReceipt,
  parseRoomMember,
  parseRoomSummary,
  parseSecurityStatus,
  parseTimelineItem,
  parseTypingSnapshot,
} from '../matrix-dto';
import type { StreamTopic } from './stream';

function isForbiddenFieldName(name: string): boolean {
  return (FORBIDDEN_WIRE_FIELD_NAMES as readonly string[]).includes(name);
}

/** Recursively reject forbidden keys and pure numeric arrays (media bytes). */
export function rejectForbiddenAndMediaBytes(value: unknown): boolean {
  if (Array.isArray(value)) {
    if (value.length > 0 && value.every((v) => typeof v === 'number')) {
      return false;
    }
    return value.every((child) => rejectForbiddenAndMediaBytes(child));
  }
  if (isObject(value)) {
    if (hasForbiddenWireFields(value)) return false;
    for (const [key, child] of Object.entries(value)) {
      if (isForbiddenFieldName(key)) return false;
      if (!rejectForbiddenAndMediaBytes(child)) return false;
    }
    return true;
  }
  return true;
}

function onlyKnownKeys(o: Record<string, unknown>, allowed: readonly string[]): boolean {
  return Object.keys(o).every((k) => allowed.includes(k));
}

function parseObjectArray<T>(value: unknown, parseOne: (v: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const out: T[] = [];
  for (const item of value) {
    const parsed = parseOne(item);
    if (!parsed) return null;
    out.push(parsed);
  }
  return out;
}

function parseAccountDataEvent(
  value: unknown,
): { type: string; content: Record<string, unknown> } | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  if (!onlyKnownKeys(value, ['type', 'content'])) return null;
  if (typeof value.type !== 'string') return null;
  const content = value.content === undefined ? {} : isObject(value.content) ? value.content : null;
  if (content === null) return null;
  if (!rejectForbiddenAndMediaBytes(content)) return null;
  return { type: value.type, content };
}

function parsePresenceEntry(value: unknown): {
  userId: string;
  presence: string;
  statusMsg?: string;
  lastActiveTs?: number;
} | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  if (!onlyKnownKeys(value, ['userId', 'presence', 'statusMsg', 'lastActiveTs'])) return null;
  if (typeof value.userId !== 'string' || typeof value.presence !== 'string') return null;
  if (value.statusMsg !== undefined && typeof value.statusMsg !== 'string') return null;
  if (
    value.lastActiveTs !== undefined &&
    (typeof value.lastActiveTs !== 'number' || !Number.isFinite(value.lastActiveTs))
  ) {
    return null;
  }
  return {
    userId: value.userId,
    presence: value.presence,
    statusMsg: typeof value.statusMsg === 'string' ? value.statusMsg : undefined,
    lastActiveTs: typeof value.lastActiveTs === 'number' ? value.lastActiveTs : undefined,
  };
}

function parseSendQueueItem(value: unknown): {
  localId: string;
  roomId: string;
  state: string;
} | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  if (!onlyKnownKeys(value, ['localId', 'roomId', 'state'])) return null;
  if (
    typeof value.localId !== 'string' ||
    typeof value.roomId !== 'string' ||
    typeof value.state !== 'string'
  ) {
    return null;
  }
  return { localId: value.localId, roomId: value.roomId, state: value.state };
}

/**
 * Validate snapshot/delta body for a stream topic.
 * Returns true when the body is a typed, privacy-safe projection for the topic.
 */
export function validateStreamTopicBody(topic: StreamTopic, body: unknown): boolean {
  if (!isObject(body)) return false;
  if (!rejectForbiddenAndMediaBytes(body)) return false;

  switch (topic) {
    case 'room_list': {
      if (!onlyKnownKeys(body, ['rooms'])) return false;
      if (body.rooms === undefined) return true;
      return parseObjectArray(body.rooms, parseRoomSummary) !== null;
    }
    case 'timeline': {
      if (!onlyKnownKeys(body, ['items'])) return false;
      if (body.items === undefined) return true;
      return parseObjectArray(body.items, parseTimelineItem) !== null;
    }
    case 'members': {
      if (!onlyKnownKeys(body, ['members'])) return false;
      if (body.members === undefined) return true;
      return parseObjectArray(body.members, parseRoomMember) !== null;
    }
    case 'typing': {
      if (!onlyKnownKeys(body, ['rooms'])) return false;
      if (body.rooms === undefined) return true;
      return parseObjectArray(body.rooms, parseTypingSnapshot) !== null;
    }
    case 'receipts': {
      if (!onlyKnownKeys(body, ['receipts'])) return false;
      if (body.receipts === undefined) return true;
      return parseObjectArray(body.receipts, parseReceipt) !== null;
    }
    case 'account_data': {
      if (!onlyKnownKeys(body, ['events'])) return false;
      if (body.events === undefined) return true;
      return parseObjectArray(body.events, parseAccountDataEvent) !== null;
    }
    case 'presence': {
      if (!onlyKnownKeys(body, ['entries'])) return false;
      if (body.entries === undefined) return true;
      return parseObjectArray(body.entries, parsePresenceEntry) !== null;
    }
    case 'notification_candidates': {
      if (!onlyKnownKeys(body, ['candidates'])) return false;
      if (body.candidates === undefined) return true;
      return parseObjectArray(body.candidates, parseNotificationCandidate) !== null;
    }
    case 'crypto_status': {
      if (!onlyKnownKeys(body, ['status'])) return false;
      if (body.status === undefined) return true;
      return parseSecurityStatus(body.status) !== null;
    }
    case 'send_queue': {
      if (!onlyKnownKeys(body, ['items'])) return false;
      if (body.items === undefined) return true;
      return parseObjectArray(body.items, parseSendQueueItem) !== null;
    }
    default: {
      // Exhaustiveness: unknown topics already rejected by isStreamTopic.
      const _exhaustive: never = topic;
      return _exhaustive;
    }
  }
}
