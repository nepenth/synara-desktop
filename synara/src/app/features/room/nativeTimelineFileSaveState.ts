import { useCallback, useEffect, useState, useSyncExternalStore } from 'react';
import type { MatrixMediaSaveEvent } from '../../matrix/media';
import { nativeTimelineDownloadFailure } from './nativeTimelineErrorCopy';
import { saveNativeTimelineFileAttachment } from './nativeTimelineFileSave';

/**
 * Where one attachment's save is. Keyed by the row's Core media handle, so the
 * file chip, the media viewer's Save and the markdown preview's Download
 * share one save, and a row that scrolls away and back finds it again.
 *
 * `total` is unknown when the homeserver sent no Content-Length. `finishing`
 * is the decrypt-and-write step after the network part. `done` carries the
 * name Rust gave the file in Downloads, which may differ from the message's.
 */
export type NativeTimelineFileSaveState =
  | { kind: 'downloading'; startedAt: number; received: number; total?: number }
  | { kind: 'finishing'; startedAt: number }
  | { kind: 'done'; filename: string }
  | { kind: 'failed'; message: string; retryable: boolean };

export type NativeTimelineFileSaveTarget = {
  handleId: string;
  filename?: string;
  mimeType?: string;
};

type Save = (
  target: NativeTimelineFileSaveTarget,
  onProgress: (event: MatrixMediaSaveEvent) => void
) => Promise<string>;

/** Saves that finish sooner than this show no progress at all, so they do not flash. */
export const NATIVE_TIMELINE_FILE_SAVE_QUIET_MS = 300;

const states = new Map<string, NativeTimelineFileSaveState>();
const flights = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();
let noticedRoomId: string | undefined;

const publish = (handleId: string, state: NativeTimelineFileSaveState | undefined) => {
  if (state) states.set(handleId, state);
  else states.delete(handleId);
  listeners.forEach((listener) => listener());
};

const isActive = (state: NativeTimelineFileSaveState | undefined): boolean =>
  state?.kind === 'downloading' || state?.kind === 'finishing';

export const nativeTimelineFileSaveState = (
  handleId: string
): NativeTimelineFileSaveState | undefined => states.get(handleId);

/**
 * Start saving `target` into Downloads, or join the save already running for
 * its handle. Settles when the save has; failures end in the `failed` state
 * rather than a rejection.
 */
export const startNativeTimelineFileSave = (
  target: NativeTimelineFileSaveTarget,
  {
    save = saveNativeTimelineFileAttachment,
    now = Date.now,
  }: { save?: Save; now?: () => number } = {}
): Promise<void> => {
  const { handleId } = target;
  const running = flights.get(handleId);
  if (running) return running;
  const startedAt = now();
  publish(handleId, { kind: 'downloading', startedAt, received: 0 });
  // Progress only moves a running save forward. A late event after the
  // result, or after Finishing, is ignored.
  const onProgress = (event: MatrixMediaSaveEvent) => {
    const current = states.get(handleId);
    if (current?.kind !== 'downloading') return;
    if (event.event === 'started') {
      publish(handleId, { ...current, total: event.data.total ?? undefined });
    } else if (event.event === 'progress') {
      publish(handleId, { ...current, received: event.data.received });
    } else {
      publish(handleId, { kind: 'finishing', startedAt });
    }
  };
  let settle: () => void = () => undefined;
  const flight = new Promise<void>((resolve) => {
    settle = resolve;
  });
  flights.set(handleId, flight);
  void (async () => {
    try {
      const filename = await save(target, onProgress);
      publish(handleId, { kind: 'done', filename });
    } catch (error) {
      publish(handleId, { kind: 'failed', ...nativeTimelineDownloadFailure(error) });
    } finally {
      flights.delete(handleId);
      settle();
    }
  })();
  return flight;
};

/** Forget a finished or failed save's line. A running save is left alone. */
export const dismissNativeTimelineFileSave = (handleId: string): void => {
  if (!states.has(handleId) || isActive(states.get(handleId))) return;
  publish(handleId, undefined);
};

/**
 * Saved and failed lines last until dismissed or until the user changes room.
 * Running saves keep going and report back if the user returns.
 */
export const noteNativeTimelineFileSaveRoom = (roomId: string): void => {
  const previous = noticedRoomId;
  noticedRoomId = roomId;
  if (previous === undefined || previous === roomId) return;
  const settled = [...states].filter(([, state]) => !isActive(state)).map(([id]) => id);
  if (settled.length === 0) return;
  settled.forEach((handleId) => states.delete(handleId));
  listeners.forEach((listener) => listener());
};

export const resetNativeTimelineFileSavesForTests = (): void => {
  states.clear();
  flights.clear();
  noticedRoomId = undefined;
  listeners.forEach((listener) => listener());
};

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/** The save state for `handleId`, kept in sync across every view of it. */
export const useNativeTimelineFileSave = (
  handleId: string | undefined
): NativeTimelineFileSaveState | undefined => {
  const getSnapshot = useCallback(() => (handleId ? states.get(handleId) : undefined), [handleId]);
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
};

/**
 * Whether a running save has lasted long enough to show its progress. A row
 * that remounts mid-save shows it at once, because the start time is shared.
 */
export const useNativeTimelineFileSaveProgressVisible = (
  state: NativeTimelineFileSaveState | undefined,
  now: () => number = Date.now
): boolean => {
  const startedAt =
    state?.kind === 'downloading' || state?.kind === 'finishing' ? state.startedAt : undefined;
  const elapsedEnough = () =>
    startedAt !== undefined && now() - startedAt >= NATIVE_TIMELINE_FILE_SAVE_QUIET_MS;
  const [visible, setVisible] = useState(elapsedEnough);
  useEffect(() => {
    if (startedAt === undefined) {
      setVisible(false);
      return undefined;
    }
    const remaining = startedAt + NATIVE_TIMELINE_FILE_SAVE_QUIET_MS - now();
    if (remaining <= 0) {
      setVisible(true);
      return undefined;
    }
    setVisible(false);
    const timer = window.setTimeout(() => setVisible(true), remaining);
    return () => window.clearTimeout(timer);
  }, [now, startedAt]);
  return visible;
};

/** Whole-number percentage of a known total, capped at 100. */
export const nativeTimelineFileSavePercent = (received: number, total: number): number =>
  total > 0 ? Math.min(100, Math.floor((received / total) * 100)) : 0;

/** "Saved to Downloads as …" with the name the file actually got. */
export const nativeTimelineFileSavedMessage = (filename: string): string =>
  `Saved to Downloads as ${filename}`;

/**
 * Button copy for a save button: its idle label, or how far the save is.
 * The progress shows only once `progressVisible` (after the quiet period).
 */
export const nativeTimelineFileSaveButtonLabel = (
  state: NativeTimelineFileSaveState | undefined,
  progressVisible: boolean,
  idle: string
): string => {
  if (!progressVisible) return idle;
  if (state?.kind === 'finishing') return 'Finishing…';
  if (state?.kind !== 'downloading') return idle;
  return state.total
    ? `Downloading ${nativeTimelineFileSavePercent(state.received, state.total)}%`
    : 'Downloading…';
};
