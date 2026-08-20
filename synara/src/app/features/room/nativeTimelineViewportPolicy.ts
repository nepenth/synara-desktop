/**
 * SDK-neutral viewport restore policy for the native timeline owner.
 *
 * Mirrors the legacy RoomTimeline restore gates without importing
 * matrix-js-sdk. Used by NativeTimelinePresenter after V-TIMELINE.C1/C2 cutover.
 */

export const NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS = 10 * 60 * 1000;

export type NativeTimelineViewportHint = {
  atBottom?: boolean;
  restoredAnchorEventId?: string;
  liveTailEventId?: string;
  updatedAtMs?: number;
};

export type NativeTimelineViewportRestoreOptions = {
  hasUnread: boolean;
  nowMs: number;
  currentLiveTailEventId?: string;
  maxAgeMs?: number;
};

const isValidEventIdHint = (eventId: string | undefined): boolean =>
  typeof eventId === 'string' && eventId.startsWith('$') && eventId.length > 1;

/**
 * Whether a saved local viewport hint may influence a normal native open.
 * Unread still beats historical restore unless the hint is an exact live-tail
 * match at bottom.
 */
export const shouldRestoreNativeTimelineViewport = (
  viewport: NativeTimelineViewportHint | undefined,
  {
    hasUnread,
    nowMs,
    currentLiveTailEventId,
    maxAgeMs = NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  }: NativeTimelineViewportRestoreOptions
): boolean => {
  if (!viewport) return false;
  if (hasUnread) {
    return Boolean(
      viewport.atBottom &&
        viewport.liveTailEventId &&
        currentLiveTailEventId &&
        viewport.liveTailEventId === currentLiveTailEventId
    );
  }
  if (viewport.atBottom) return true;
  if (maxAgeMs < 0) return false;
  const { updatedAtMs } = viewport;
  if (typeof updatedAtMs !== 'number' || !Number.isFinite(updatedAtMs)) return false;
  if (Math.max(0, nowMs - updatedAtMs) > maxAgeMs) return false;
  return isValidEventIdHint(viewport.restoredAnchorEventId);
};

/**
 * Jump-to-latest is the way back to the live tail. Scroll-bottom of the
 * currently loaded window is not enough: an unread/focused/restored window
 * can be fully visible while newer messages live on a different timeline.
 */
export const shouldShowJumpToLatest = (
  positionKind: 'live_bottom' | 'unread' | 'focused' | 'restored' | undefined,
  scrolledToVisualBottom: boolean
): boolean => {
  if (positionKind && positionKind !== 'live_bottom') return true;
  return scrolledToVisualBottom === false;
};
