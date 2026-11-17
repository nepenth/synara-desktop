/**
 * Coalesces a stream of changes into occasional writes: a trailing debounce
 * with an optional max-wait. Pure and clock-injectable so the draft and
 * preview paths can be tested without a DOM.
 */

/**
 * Quiet time before a composer draft is written. Steady typing has 100-250 ms
 * between keys (40-100 wpm), so 400 ms coalesces a burst into one write while
 * a pause between phrases still saves. It matches the link-preview debounce.
 */
export const DRAFT_SAVE_DEBOUNCE_MS = 400;

/**
 * Typing without a pause still writes this often, so a crash (the one exit no
 * flush can catch) loses at most this much text.
 */
export const DRAFT_SAVE_MAX_WAIT_MS = 2_000;

export type SchedulerClock = {
  now: () => number;
  setTimeout: (run: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
};

export const realClock: SchedulerClock = {
  now: () => Date.now(),
  setTimeout: (run, ms) => globalThis.setTimeout(run, ms),
  clearTimeout: (handle) => globalThis.clearTimeout(handle as ReturnType<typeof setTimeout>),
};

export type CoalescingScheduler<T> = {
  /** A change to `key`. A pending change of another key is written first. */
  schedule: (key: string, value: T) => void;
  /** Write the pending change now. `urgent`: leaving the room or the window. */
  flush: (urgent?: boolean) => void;
  /** Drop `key`'s pending change unwritten: it was sent or cleared. */
  cancel: (key: string) => void;
  pendingKey: () => string | undefined;
  /** Write what is pending and stop; later changes are written at once. */
  dispose: () => void;
};

type Options<T> = {
  write: (key: string, value: T, urgent: boolean) => void;
  delayMs: number;
  /** Longest a change waits while changes keep coming; Infinity for none. */
  maxWaitMs?: number;
  clock?: SchedulerClock;
};

export function createCoalescingScheduler<T>({
  write,
  delayMs,
  maxWaitMs = Number.POSITIVE_INFINITY,
  clock = realClock,
}: Options<T>): CoalescingScheduler<T> {
  let pending: { key: string; value: T; firstAt: number; lastAt: number } | undefined;
  let timer: unknown;
  let disposed = false;

  const stopTimer = () => {
    if (timer === undefined) return;
    clock.clearTimeout(timer);
    timer = undefined;
  };

  const run = (urgent: boolean) => {
    stopTimer();
    const taken = pending;
    pending = undefined;
    // Taken before writing, so a change made by the write starts afresh.
    if (taken) write(taken.key, taken.value, urgent);
  };

  const dueAt = (entry: NonNullable<typeof pending>) =>
    Math.min(entry.lastAt + delayMs, entry.firstAt + maxWaitMs);

  // One timer per burst, not one per change: when it fires early (more
  // changes came in), it re-arms for the remaining time.
  const onTimer = () => {
    timer = undefined;
    if (!pending) return;
    const wait = dueAt(pending) - clock.now();
    if (wait > 0) {
      timer = clock.setTimeout(onTimer, wait);
      return;
    }
    run(false);
  };

  return {
    schedule: (key, value) => {
      if (pending && pending.key !== key) run(true);
      const now = clock.now();
      if (disposed) {
        write(key, value, true);
        return;
      }
      pending = pending
        ? { key, value, firstAt: pending.firstAt, lastAt: now }
        : { key, value, firstAt: now, lastAt: now };
      if (timer === undefined) timer = clock.setTimeout(onTimer, delayMs);
    },
    flush: (urgent = true) => run(urgent),
    cancel: (key) => {
      if (pending?.key !== key) return;
      pending = undefined;
      stopTimer();
    },
    pendingKey: () => pending?.key,
    dispose: () => {
      run(true);
      disposed = true;
    },
  };
}
