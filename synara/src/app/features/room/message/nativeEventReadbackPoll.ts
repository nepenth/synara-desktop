/** First retry for a focused-event readback that is still decrypting. */
export const EVENT_READBACK_INITIAL_DELAY_MS = 1_000;
/** Slowest retry; each readback may hit the network through the registry. */
export const EVENT_READBACK_MAX_DELAY_MS = 30_000;

export type EventReadbackState = 'pending' | 'unavailable' | 'decrypted';

/**
 * Delay before the next readback, or `undefined` to stop. A decrypted event
 * is final. `unavailable` is shown and polling stops; the row re-polls when it
 * remounts. `pending` backs off exponentially up to the cap.
 */
export const nextEventReadbackDelayMs = (
  state: EventReadbackState,
  previousDelayMs: number | undefined
): number | undefined => {
  if (state !== 'pending') return undefined;
  if (previousDelayMs === undefined) return EVENT_READBACK_INITIAL_DELAY_MS;
  return Math.min(previousDelayMs * 2, EVENT_READBACK_MAX_DELAY_MS);
};
