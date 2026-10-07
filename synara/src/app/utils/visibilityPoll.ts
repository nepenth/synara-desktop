/** Poll cadence for native list snapshots while the window is visible. */
export const VISIBLE_POLL_INTERVAL_MS = 1_000;
/**
 * Poll cadence while the window is hidden (for example, closed to the tray).
 * The lists still refresh, but no longer cost a burst of IPC every second.
 */
export const HIDDEN_POLL_INTERVAL_MS = 15_000;

export type VisibilityPollEnvironment = {
  isHidden: () => boolean;
  setTimeout: (callback: () => void, ms: number) => number;
  clearTimeout: (id: number) => void;
  onVisibilityChange: (listener: () => void) => () => void;
};

const browserEnvironment = (): VisibilityPollEnvironment => ({
  isHidden: () => document.visibilityState === 'hidden',
  setTimeout: (callback, ms) => window.setTimeout(callback, ms),
  clearTimeout: (id) => window.clearTimeout(id),
  onVisibilityChange: (listener) => {
    document.addEventListener('visibilitychange', listener);
    return () => document.removeEventListener('visibilitychange', listener);
  },
});

/**
 * Run `tick` every `visibleMs` while the document is visible and every
 * `hiddenMs` while it is hidden. Becoming visible runs one tick immediately.
 * Does not run an initial tick; callers already refresh on mount.
 * Returns a stop function.
 */
export const startVisibilityAwarePoll = (
  tick: () => void,
  visibleMs: number = VISIBLE_POLL_INTERVAL_MS,
  hiddenMs: number = HIDDEN_POLL_INTERVAL_MS,
  environment: VisibilityPollEnvironment = browserEnvironment()
): (() => void) => {
  let timer: number | undefined;
  let stopped = false;

  const schedule = () => {
    if (stopped) return;
    timer = environment.setTimeout(
      () => {
        timer = undefined;
        tick();
        schedule();
      },
      environment.isHidden() ? hiddenMs : visibleMs
    );
  };

  const stopListening = environment.onVisibilityChange(() => {
    if (stopped) return;
    if (timer !== undefined) environment.clearTimeout(timer);
    timer = undefined;
    if (!environment.isHidden()) tick();
    schedule();
  });

  schedule();
  return () => {
    stopped = true;
    if (timer !== undefined) environment.clearTimeout(timer);
    timer = undefined;
    stopListening();
  };
};
