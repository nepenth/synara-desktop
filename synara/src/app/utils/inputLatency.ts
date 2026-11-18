/**
 * Keystroke-to-paint latency for the performance diagnostics.
 *
 * A key press in a text field is timed from the event's own `timeStamp` (when
 * the browser received it, so time spent queued behind other work counts) to
 * just after the next frame: a `requestAnimationFrame` callback runs before
 * that frame paints, and a task queued from it runs once the frame is done.
 * That is close to "how long until the typed character could be on screen",
 * which is what a stuttering composer feels like.
 *
 * Only active while the performance diagnostics are on; nothing here runs a
 * loop of its own, so measuring does not keep the page repainting.
 */

export type InputLatencySummary = {
  count: number;
  p50Ms: number;
  p95Ms: number;
  maxMs: number;
};

/** Nearest-rank percentile of an ascending list; 0 for an empty list. */
export const percentile = (sorted: readonly number[], fraction: number): number => {
  if (sorted.length === 0) return 0;
  const rank = Math.ceil(fraction * sorted.length);
  return sorted[Math.min(sorted.length, Math.max(1, rank)) - 1];
};

export const summarizeInputLatency = (
  samples: readonly number[]
): InputLatencySummary | undefined => {
  if (samples.length === 0) return undefined;
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    count: sorted.length,
    p50Ms: Math.round(percentile(sorted, 0.5)),
    p95Ms: Math.round(percentile(sorted, 0.95)),
    maxMs: Math.round(sorted[sorted.length - 1]),
  };
};

/** Keys that put or remove text; navigation and modifier keys are skipped. */
export const isTypingKey = (event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey'>) =>
  !event.ctrlKey &&
  !event.metaKey &&
  !event.altKey &&
  (event.key.length === 1 ||
    event.key === 'Backspace' ||
    event.key === 'Delete' ||
    event.key === 'Enter');

const isTextTarget = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return target instanceof HTMLTextAreaElement || target instanceof HTMLInputElement;
};

/** Bounded so a long typing session cannot grow memory between reads. */
const MAX_PENDING_SAMPLES = 2000;

/**
 * Starts timing typing keys anywhere in the document. `take()` returns the
 * samples since the last call and clears them; `stop()` removes the listener.
 */
export const startInputLatencyProbe = (target: Document = document) => {
  let samples: number[] = [];
  let stopped = false;

  const onKeyDown = (event: KeyboardEvent) => {
    if (stopped || !isTypingKey(event) || !isTextTarget(event.target)) return;
    const startedAt = event.timeStamp;
    window.requestAnimationFrame(() => {
      window.setTimeout(() => {
        if (stopped) return;
        const elapsed = performance.now() - startedAt;
        if (elapsed >= 0 && samples.length < MAX_PENDING_SAMPLES) samples.push(elapsed);
      }, 0);
    });
  };

  target.addEventListener('keydown', onKeyDown, { capture: true, passive: true });
  return {
    take: (): number[] => {
      const taken = samples;
      samples = [];
      return taken;
    },
    stop: () => {
      stopped = true;
      target.removeEventListener('keydown', onKeyDown, { capture: true });
    },
  };
};
