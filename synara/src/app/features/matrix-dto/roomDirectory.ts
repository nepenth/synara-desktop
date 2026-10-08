/**
 * V-ROOMS.R-DIRECTORY — strict Synara-owned public-room directory DTOs.
 *
 * This parser deliberately does not mirror Matrix SDK/Ruma response objects.
 * Unknown fields, secret-looking fields, malformed pagination, and unsupported
 * room types are rejected before data reaches Explore.
 */

import type {
  DirectoryProtocolInstance,
  DirectoryRoomHitDto,
  NativeRoomDirectoryPage,
  NativeRoomDirectoryProtocols,
  NativeRoomDirectorySearchResponse,
} from './generated';
import { hasForbiddenWireFields, isObject } from './parseUtil';

export const DIRECTORY_MAX_TEXT_CHARS = 256;
export const DIRECTORY_MAX_TOPIC_CHARS = DIRECTORY_MAX_TEXT_CHARS * 4;
export const DIRECTORY_MAX_ALIAS_CHARS = 255;
export const DIRECTORY_MAX_BATCH_CHARS = 512;
export const DIRECTORY_MAX_HITS = 200;
export const DIRECTORY_MAX_PROTOCOL_INSTANCES = 128;

// Wire shapes are generated from the Rust Core types; this module only adds
// the strict parse that rejects unknown or secret-looking fields.
export type DirectoryRoomHit = DirectoryRoomHitDto;
export type DirectoryRoomType = DirectoryRoomHit['roomType'];
export type DirectoryPage = NativeRoomDirectoryPage;
export type { DirectoryProtocolInstance };
export type DirectoryProtocols = NativeRoomDirectoryProtocols;
export type DirectorySearchResponse = NativeRoomDirectorySearchResponse;
export type DirectorySearchStatus = DirectorySearchResponse['status'];

const PAGE_KEYS = ['sessionGeneration', 'requestId', 'chunk', 'prevBatch', 'nextBatch'];
const HIT_KEYS = [
  'roomId',
  'name',
  'topic',
  'canonicalAlias',
  'avatarUrl',
  'memberCount',
  'worldReadable',
  'guestCanJoin',
  'roomType',
];
const PROTOCOLS_KEYS = ['sessionGeneration', 'instances'];
const PROTOCOL_KEYS = ['protocolId', 'instanceId', 'description'];
const RESPONSE_KEYS = ['sessionGeneration', 'requestId', 'status', 'page'];

const hasExactKeys = (value: Record<string, unknown>, keys: string[]): boolean => {
  const allowed = new Set(keys);
  return Object.keys(value).every((key) => allowed.has(key));
};

const isSafeCounter = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value > 0;

const isBoundedString = (value: unknown, maxChars: number, nonEmpty = false): value is string =>
  typeof value === 'string' &&
  [...value].length <= maxChars &&
  (!nonEmpty || value.trim().length > 0);

const parseOptionalString = (
  value: Record<string, unknown>,
  key: string,
  maxChars: number,
  nonEmpty = false
): string | undefined | null => {
  if (!(key in value) || value[key] === undefined) return undefined;
  return isBoundedString(value[key], maxChars, nonEmpty) ? value[key] : null;
};

const parseBatch = (
  value: Record<string, unknown>,
  key: 'prevBatch' | 'nextBatch'
): string | undefined | null => {
  const batch = parseOptionalString(value, key, DIRECTORY_MAX_BATCH_CHARS, true);
  if (batch === null || (batch !== undefined && batch.includes('access_token'))) return null;
  if (batch !== undefined && batch.includes('refresh_token')) return null;
  return batch;
};

const parseHit = (value: unknown): DirectoryRoomHit | null => {
  if (!isObject(value) || hasForbiddenWireFields(value) || !hasExactKeys(value, HIT_KEYS)) {
    return null;
  }
  const roomId = value.roomId;
  const memberCount = value.memberCount;
  const name = parseOptionalString(value, 'name', DIRECTORY_MAX_TEXT_CHARS);
  const topic = parseOptionalString(value, 'topic', DIRECTORY_MAX_TOPIC_CHARS);
  const canonicalAlias = parseOptionalString(
    value,
    'canonicalAlias',
    DIRECTORY_MAX_ALIAS_CHARS,
    true
  );
  const avatarUrl = parseOptionalString(value, 'avatarUrl', DIRECTORY_MAX_BATCH_CHARS, true);
  if (
    typeof roomId !== 'string' ||
    !isBoundedString(roomId, DIRECTORY_MAX_ALIAS_CHARS, true) ||
    !roomId.startsWith('!') ||
    typeof memberCount !== 'number' ||
    !Number.isSafeInteger(memberCount) ||
    memberCount < 0 ||
    name === null ||
    topic === null ||
    canonicalAlias === null ||
    avatarUrl === null ||
    (canonicalAlias !== undefined && !canonicalAlias.startsWith('#')) ||
    (avatarUrl !== undefined &&
      !/^(mxc|synara-media):\/\//i.test(avatarUrl) &&
      !avatarUrl.startsWith('mxc://')) ||
    typeof value.worldReadable !== 'boolean' ||
    typeof value.guestCanJoin !== 'boolean' ||
    (value.roomType !== 'room' && value.roomType !== 'space')
  ) {
    return null;
  }
  return {
    roomId,
    name,
    topic,
    canonicalAlias,
    avatarUrl,
    memberCount,
    worldReadable: value.worldReadable,
    guestCanJoin: value.guestCanJoin,
    roomType: value.roomType,
  };
};

export function parseDirectoryPage(value: unknown): DirectoryPage | null {
  if (!isObject(value) || hasForbiddenWireFields(value) || !hasExactKeys(value, PAGE_KEYS)) {
    return null;
  }
  if (!isSafeCounter(value.sessionGeneration) || !isSafeCounter(value.requestId)) return null;
  if (!Array.isArray(value.chunk) || value.chunk.length > DIRECTORY_MAX_HITS) return null;
  const chunk = value.chunk.map(parseHit);
  if (chunk.some((hit) => hit === null)) return null;
  const prevBatch = parseBatch(value, 'prevBatch');
  const nextBatch = parseBatch(value, 'nextBatch');
  if (prevBatch === null || nextBatch === null) return null;
  return {
    sessionGeneration: value.sessionGeneration,
    requestId: value.requestId,
    chunk: chunk as DirectoryRoomHit[],
    prevBatch,
    nextBatch,
  };
}

export function parseDirectorySearchResponse(value: unknown): DirectorySearchResponse | null {
  if (!isObject(value) || hasForbiddenWireFields(value) || !hasExactKeys(value, RESPONSE_KEYS)) {
    return null;
  }
  if (
    !isSafeCounter(value.sessionGeneration) ||
    !isSafeCounter(value.requestId) ||
    (value.status !== 'ready' && value.status !== 'stale' && value.status !== 'cancelled')
  ) {
    return null;
  }
  const page = value.page === undefined ? undefined : parseDirectoryPage(value.page);
  if (value.status === 'ready') {
    if (page === undefined || page === null) return null;
  } else if (value.page !== undefined || page !== undefined) return null;
  return {
    sessionGeneration: value.sessionGeneration,
    requestId: value.requestId,
    status: value.status,
    page: page ?? undefined,
  };
}

export function parseDirectoryProtocols(value: unknown): DirectoryProtocols | null {
  if (!isObject(value) || hasForbiddenWireFields(value) || !hasExactKeys(value, PROTOCOLS_KEYS)) {
    return null;
  }
  if (
    !isSafeCounter(value.sessionGeneration) ||
    !Array.isArray(value.instances) ||
    value.instances.length > DIRECTORY_MAX_PROTOCOL_INSTANCES
  ) {
    return null;
  }
  const instances: DirectoryProtocolInstance[] = [];
  for (const instance of value.instances) {
    if (
      !isObject(instance) ||
      hasForbiddenWireFields(instance) ||
      !hasExactKeys(instance, PROTOCOL_KEYS) ||
      !isBoundedString(instance.protocolId, DIRECTORY_MAX_TEXT_CHARS, true) ||
      !isBoundedString(instance.instanceId, DIRECTORY_MAX_TEXT_CHARS, true) ||
      !isBoundedString(instance.description, DIRECTORY_MAX_TEXT_CHARS, true)
    ) {
      return null;
    }
    instances.push({
      protocolId: instance.protocolId,
      instanceId: instance.instanceId,
      description: instance.description,
    });
  }
  return { sessionGeneration: value.sessionGeneration, instances };
}
