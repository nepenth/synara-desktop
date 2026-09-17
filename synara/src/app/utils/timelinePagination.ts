export type TimelinePaginationDirection = 'backward' | 'forward';

export type TimelinePaginationErrors = Partial<Record<TimelinePaginationDirection, string>>;

export type TimelinePageNativeState = 'available' | 'exhausted' | 'loading' | 'unavailable';

export type TimelineHistoryOverlayKind = 'hidden' | 'loading' | 'error' | 'load_more';

export type TimelineHistoryOverlay = {
  kind: TimelineHistoryOverlayKind;
  message?: string;
};

export const createTimelinePaginationErrorMessage = (err: unknown): string => {
  if (err instanceof Error && err.message.trim().length > 0) {
    return err.message;
  }
  if (err && typeof err === 'object' && 'message' in err) {
    const message = (err as { message?: unknown }).message;
    if (typeof message === 'string' && message.trim().length > 0) {
      return message;
    }
  }
  return 'Failed to load messages.';
};

export const setTimelinePaginationError = (
  errors: TimelinePaginationErrors,
  direction: TimelinePaginationDirection,
  err: unknown
): TimelinePaginationErrors => ({
  ...errors,
  [direction]: createTimelinePaginationErrorMessage(err),
});

export const clearTimelinePaginationError = (
  errors: TimelinePaginationErrors,
  direction: TimelinePaginationDirection
): TimelinePaginationErrors => {
  if (!errors[direction]) return errors;
  const next = { ...errors };
  delete next[direction];
  return next;
};

export const shouldShowTimelinePaginationLoader = (
  canPaginate: boolean,
  errors: TimelinePaginationErrors,
  direction: TimelinePaginationDirection
): boolean => canPaginate && !errors[direction];

/** True while this direction is fetching, including optimistic local in-flight. */
export const isTimelinePaginationLoading = ({
  nativeState,
  inFlight,
  hasError,
}: {
  nativeState?: TimelinePageNativeState;
  inFlight: boolean;
  hasError: boolean;
}): boolean => !hasError && (inFlight || nativeState === 'loading');

/** Live tail has nothing newer unless the window is focused/unread with a gap. */
export const canPaginateTimelineForward = ({
  atLiveBottom,
  positionKind,
}: {
  atLiveBottom: boolean;
  positionKind: string;
}): boolean => {
  if (positionKind === 'live_bottom') return false;
  if (atLiveBottom && positionKind !== 'focused' && positionKind !== 'unread') return false;
  return true;
};

/**
 * Sticky history-edge chrome. Errors win over loading so a failed fetch is
 * never mistaken for an in-progress spinner. Loading only paints while the
 * viewport is actually on that edge. `load_more` is only for a large loaded
 * window sitting on the history edge; sparse rooms keep their own button.
 */
export const resolveTimelineHistoryOverlay = ({
  nativeState,
  inFlight,
  error,
  atEdge,
  canPaginate,
  hasSparseLoadButton,
}: {
  nativeState?: TimelinePageNativeState;
  inFlight: boolean;
  error?: string;
  atEdge: boolean;
  canPaginate: boolean;
  hasSparseLoadButton?: boolean;
}): TimelineHistoryOverlay => {
  if (error) return { kind: 'error', message: error };
  // Live tail has nothing newer. Do not paint "Loading newer messages" just
  // because the viewport is on the bottom edge or native forward is idle-loading.
  if (!canPaginate) return { kind: 'hidden' };
  if (atEdge && (inFlight || nativeState === 'loading')) return { kind: 'loading' };
  if (atEdge && nativeState === 'available' && !hasSparseLoadButton) {
    return { kind: 'load_more' };
  }
  return { kind: 'hidden' };
};
