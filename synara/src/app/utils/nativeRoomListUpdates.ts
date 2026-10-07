import { listen } from '@tauri-apps/api/event';
import { isSynaraDesktop } from './desktop';
import {
  HIDDEN_POLL_INTERVAL_MS,
  startVisibilityAwarePoll,
  VISIBLE_POLL_INTERVAL_MS,
} from './visibilityPoll';

/**
 * Core wakes every list derived from sync (rooms, invites, the direct-message
 * map, space parents, Later and room notes) on this event. The payload carries
 * no room ids; consumers re-read their existing snapshot command.
 */
export const ROOM_LIST_UPDATED_EVENT = 'matrix-room-list-updated';

/** Safety-net poll while the event stream is live. */
export const EVENT_FALLBACK_VISIBLE_POLL_MS = 30_000;
export const EVENT_FALLBACK_HIDDEN_POLL_MS = 120_000;

export type NativeRoomListUpdate = { sessionGeneration: number; revision: number };

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

const isCounter = (value: unknown): value is number =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;

/** Fail-closed parser: exactly `sessionGeneration` and `revision`. */
export const parseNativeRoomListUpdate = (value: unknown): NativeRoomListUpdate | undefined => {
  if (!isRecord(value)) return undefined;
  const keys = Object.keys(value);
  if (keys.length !== 2 || !keys.includes('sessionGeneration') || !keys.includes('revision'))
    return undefined;
  const { sessionGeneration, revision } = value;
  if (!isCounter(sessionGeneration) || !isCounter(revision)) return undefined;
  return { sessionGeneration, revision };
};

export type RoomListUpdateSource = {
  /**
   * Register `onUpdate`. `onLive` runs once the stream is listening;
   * `onUnavailable` runs if it can never listen. Returns an unsubscribe.
   */
  subscribe: (
    onUpdate: (update: NativeRoomListUpdate) => void,
    onLive: () => void,
    onUnavailable: () => void
  ) => () => void;
};

type Subscriber = {
  onUpdate: (update: NativeRoomListUpdate) => void;
  onLive: () => void;
  onUnavailable: () => void;
};

type ListenFn = (
  event: string,
  handler: (event: { payload: unknown }) => void
) => Promise<() => void>;

/**
 * One Tauri listener shared by every list. It is registered with the first
 * subscriber and released with the last.
 */
export const createRoomListUpdateSource = (listenTo: ListenFn): RoomListUpdateSource => {
  const subscribers = new Set<Subscriber>();
  let state: 'idle' | 'pending' | 'live' | 'unavailable' = 'idle';
  let unlisten: (() => void) | undefined;
  let attempt = 0;

  const release = () => {
    attempt += 1;
    unlisten?.();
    unlisten = undefined;
    state = 'idle';
  };

  const register = () => {
    state = 'pending';
    attempt += 1;
    const current = attempt;
    listenTo(ROOM_LIST_UPDATED_EVENT, (event) => {
      const update = parseNativeRoomListUpdate(event.payload);
      if (!update) return;
      subscribers.forEach((subscriber) => subscriber.onUpdate(update));
    }).then(
      (stop) => {
        if (current !== attempt) {
          stop();
          return;
        }
        unlisten = stop;
        state = 'live';
        subscribers.forEach((subscriber) => subscriber.onLive());
      },
      () => {
        if (current !== attempt) return;
        state = 'unavailable';
        subscribers.forEach((subscriber) => subscriber.onUnavailable());
      }
    );
  };

  return {
    subscribe: (onUpdate, onLive, onUnavailable) => {
      const subscriber: Subscriber = { onUpdate, onLive, onUnavailable };
      subscribers.add(subscriber);
      if (state === 'idle') register();
      else if (state === 'live') onLive();
      else if (state === 'unavailable') onUnavailable();
      return () => {
        subscribers.delete(subscriber);
        if (subscribers.size === 0) release();
      };
    },
  };
};

const sharedSource = createRoomListUpdateSource((event, handler) => listen(event, handler));

export type RoomListUpdateDrivenPollOptions = {
  source?: RoomListUpdateSource;
  startPoll?: typeof startVisibilityAwarePoll;
  isDesktop?: () => boolean;
};

/**
 * Refresh a native list when Core reports a change, with a slow safety poll.
 *
 * The stream going live runs one catch-up tick, so a change Core emitted
 * before the listener registered is not lost. If the stream cannot listen,
 * this falls back to the regular visibility-aware cadence. Does not run an
 * initial tick; callers already refresh on mount. Returns a stop function.
 */
export const startRoomListUpdateDrivenPoll = (
  tick: () => void,
  {
    source = sharedSource,
    startPoll = startVisibilityAwarePoll,
    isDesktop = isSynaraDesktop,
  }: RoomListUpdateDrivenPollOptions = {}
): (() => void) => {
  if (!isDesktop()) return startPoll(tick, VISIBLE_POLL_INTERVAL_MS, HIDDEN_POLL_INTERVAL_MS);
  let stopped = false;
  let stopPoll = startPoll(tick, EVENT_FALLBACK_VISIBLE_POLL_MS, EVENT_FALLBACK_HIDDEN_POLL_MS);
  const unsubscribe = source.subscribe(
    () => {
      if (!stopped) tick();
    },
    () => {
      if (!stopped) tick();
    },
    () => {
      if (stopped) return;
      stopPoll();
      stopPoll = startPoll(tick, VISIBLE_POLL_INTERVAL_MS, HIDDEN_POLL_INTERVAL_MS);
    }
  );
  return () => {
    stopped = true;
    stopPoll();
    unsubscribe();
  };
};
