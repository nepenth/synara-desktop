/**
 * Message search DTOs.
 */

import type { EventId, RoomId, UserId } from './ids';
import {
  hasForbiddenWireFields,
  isObject,
  optNumber,
  optString,
  reqString,
} from './parseUtil';

export type SearchResultItem = {
  eventId: EventId;
  roomId: RoomId;
  originServerTs?: number;
  sender?: UserId;
  snippet?: string;
};

export type SearchResult = {
  query: string;
  roomId?: RoomId;
  results: SearchResultItem[];
  nextBatch?: string;
  totalCount?: number;
};

function parseItem(value: unknown): SearchResultItem | null {
  if (!isObject(value)) return null;
  const eventId = reqString(value, 'eventId');
  const roomId = reqString(value, 'roomId');
  const originServerTs = optNumber(value, 'originServerTs');
  const sender = optString(value, 'sender');
  const snippet = optString(value, 'snippet');
  if (
    eventId === null ||
    roomId === null ||
    originServerTs === null ||
    sender === null ||
    snippet === null
  ) {
    return null;
  }
  return { eventId, roomId, originServerTs, sender, snippet };
}

export function parseSearchResult(value: unknown): SearchResult | null {
  if (!isObject(value) || hasForbiddenWireFields(value)) return null;
  const query = reqString(value, 'query');
  const roomId = optString(value, 'roomId');
  const nextBatch = optString(value, 'nextBatch');
  const totalCount = optNumber(value, 'totalCount');
  if (
    query === null ||
    roomId === null ||
    nextBatch === null ||
    totalCount === null ||
    !Array.isArray(value.results)
  ) {
    return null;
  }
  const results: SearchResultItem[] = [];
  for (const item of value.results) {
    const parsed = parseItem(item);
    if (!parsed) return null;
    results.push(parsed);
  }
  return { query, roomId, results, nextBatch, totalCount };
}
