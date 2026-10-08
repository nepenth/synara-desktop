import { useEffect, useSyncExternalStore } from 'react';
import { isObject } from '../features/matrix-dto/parseUtil';
import type {
  NativeAccountDataContent,
  NativeAccountDataTypes,
} from '../features/matrix-dto/generated';
import { invokeDesktopWithAvailability } from '../utils/desktop';
import { startRoomListUpdateDrivenPoll } from '../utils/nativeRoomListUpdates';
import { UNAVAILABLE_MESSAGE, type NativeInvoke } from './nativeWire';

/**
 * Global and room account data through Core's raw account-data commands.
 *
 * Reads are cached so synchronous call sites (sidebar layout, recent emoji)
 * can read the last known content. Every type read once is refreshed when Core
 * reports an account-data change. Secret-bearing types are refused by Core.
 */

export type AccountDataObject = Record<string, unknown>;

type Listener = () => void;

let invoke: NativeInvoke = (command, args) => invokeDesktopWithAvailability(command, args);
const cache = new Map<string, AccountDataObject | null>();
const tracked = new Map<string, { eventType: string; roomId?: string }>();
const listeners = new Set<Listener>();
let stopRefresh: (() => void) | undefined;
let refreshEnabled = true;

const cacheKey = (eventType: string, roomId?: string): string =>
  roomId ? `${roomId}\u0000${eventType}` : eventType;

const notify = () => listeners.forEach((listener) => listener());

/** Route commands through a fake invoke and clear the cache (tests). */
export const setNativeAccountDataInvokeForTests = (next: NativeInvoke): void => {
  invoke = next;
  refreshEnabled = false;
  cache.clear();
  tracked.clear();
  stopRefresh?.();
  stopRefresh = undefined;
};

/** Content of a `NativeAccountDataContent` reply, or `undefined` if malformed. */
export const parseAccountDataContent = (
  value: unknown
): { content: AccountDataObject | null } | undefined => {
  if (!isObject(value)) return undefined;
  const reply = value as Partial<NativeAccountDataContent>;
  if (typeof reply.eventType !== 'string') return undefined;
  if (reply.content === null || reply.content === undefined) return { content: null };
  return isObject(reply.content) ? { content: reply.content as AccountDataObject } : undefined;
};

export const parseAccountDataTypes = (value: unknown): string[] | undefined => {
  if (!isObject(value)) return undefined;
  const types = (value as Partial<NativeAccountDataTypes>).types;
  if (!Array.isArray(types) || !types.every((type) => typeof type === 'string')) return undefined;
  return [...types].sort();
};

const ensureRefresh = () => {
  if (stopRefresh || !refreshEnabled) return;
  stopRefresh = startRoomListUpdateDrivenPoll(() => {
    tracked.forEach(({ eventType, roomId }) => {
      void loadNativeAccountData(eventType, roomId).catch(() => undefined);
    });
  });
};

/** Last known content; `undefined` when never loaded, `null` when absent. */
export const getCachedAccountData = (
  eventType: string,
  roomId?: string
): AccountDataObject | null | undefined => cache.get(cacheKey(eventType, roomId));

/** Fetch one account-data object from Core and update the cache. */
export const loadNativeAccountData = async (
  eventType: string,
  roomId?: string
): Promise<AccountDataObject | null> => {
  const key = cacheKey(eventType, roomId);
  tracked.set(key, { eventType, roomId });
  const result = await invoke('matrix_account_data_get', { eventType, roomId });
  if (!result.available) return null;
  const parsed = parseAccountDataContent(result.value);
  if (!parsed) throw new Error('Invalid account data response.');
  const previous = cache.get(key);
  cache.set(key, parsed.content);
  if (JSON.stringify(previous) !== JSON.stringify(parsed.content)) notify();
  ensureRefresh();
  return parsed.content;
};

/** Replace one account-data object. Throws when the desktop shell is absent or Core rejects it. */
export const setNativeAccountData = async (
  eventType: string,
  content: AccountDataObject,
  roomId?: string
): Promise<void> => {
  const result = await invoke('matrix_account_data_set', { eventType, roomId, content });
  if (!result.available) throw new Error(UNAVAILABLE_MESSAGE);
  const parsed = parseAccountDataContent(result.value);
  const key = cacheKey(eventType, roomId);
  tracked.set(key, { eventType, roomId });
  cache.set(key, parsed?.content ?? content);
  notify();
  ensureRefresh();
};

/** Account-data types Core has seen this session (global, or for one room). */
export const listNativeAccountDataTypes = async (roomId?: string): Promise<string[]> => {
  const result = await invoke('matrix_account_data_types', { roomId });
  if (!result.available) return [];
  return parseAccountDataTypes(result.value) ?? [];
};

export const subscribeNativeAccountData = (listener: Listener): (() => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/** Live account data of one type; loads it on first use. */
export const useNativeAccountData = (
  eventType: string,
  roomId?: string,
  enabled = true
): AccountDataObject | null | undefined => {
  useEffect(() => {
    if (!enabled) return;
    void loadNativeAccountData(eventType, roomId).catch(() => undefined);
  }, [eventType, roomId, enabled]);
  return useSyncExternalStore(subscribeNativeAccountData, () =>
    enabled ? getCachedAccountData(eventType, roomId) : undefined
  );
};

/**
 * Every account-data object Core has seen for one room (or globally), keyed by
 * type. Types whose read fails or is absent are left out.
 */
export const loadAllNativeAccountData = async (
  roomId?: string
): Promise<Map<string, AccountDataObject>> => {
  const types = await listNativeAccountDataTypes(roomId);
  const entries = await Promise.all(
    types.map(async (type) => {
      const content = await loadNativeAccountData(type, roomId).catch(() => null);
      return content ? ([type, content] as const) : undefined;
    })
  );
  return new Map(entries.filter((entry) => entry !== undefined));
};
