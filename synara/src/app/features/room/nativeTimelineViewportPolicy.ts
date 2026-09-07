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

export type NativeLiveReadTargetInput = {
  selectedRoomId: string;
  snapshotRoomId: string;
  documentActive: boolean;
  hideActivity: boolean;
  atLiveBottom: boolean;
  positionKind: 'live_bottom' | 'unread' | 'focused' | 'restored' | undefined;
  canMarkRead: boolean;
  latestVisibleEventId?: string;
  ownReadEventId?: string;
  isMarkedUnread: boolean;
};

/**
 * Select the exact remote live-tail event that the native SDK owner may mark.
 * Snapshot revisions are intentionally absent: non-event rebuilds must not
 * emit duplicate receipts, and background windows must not claim visibility.
 */
export const nativeLiveReadTarget = ({
  selectedRoomId,
  snapshotRoomId,
  documentActive,
  hideActivity,
  atLiveBottom,
  positionKind,
  canMarkRead,
  latestVisibleEventId,
  ownReadEventId,
  isMarkedUnread,
}: NativeLiveReadTargetInput): string | undefined => {
  if (
    selectedRoomId !== snapshotRoomId ||
    !documentActive ||
    hideActivity ||
    !atLiveBottom ||
    positionKind !== 'live_bottom' ||
    !canMarkRead ||
    !isValidEventIdHint(latestVisibleEventId)
  ) {
    return undefined;
  }
  if (!isMarkedUnread && ownReadEventId === latestVisibleEventId) return undefined;
  return latestVisibleEventId;
};

/** A manual unread transition is a distinct intent even on the same tail. */
export const nativeLiveReadAttemptKey = (
  roomId: string,
  eventId: string,
  isMarkedUnread: boolean
): string => `${roomId}:${eventId}:${isMarkedUnread ? 'explicit-unread' : 'read-frontier'}`;

export type NativeFollowLiveTargetInput = {
  roomId: string;
  atLiveBottom: boolean;
  positionKind: 'live_bottom' | 'unread' | 'focused' | 'restored' | undefined;
  latestVisibleEventId?: string;
};

/**
 * Select the painted tail eligible for a Core-verified follow-live
 * transition. Forward pagination never re-anchors a non-live stream, so a
 * room opened at unread/restored/focused would otherwise gate automatic
 * receipts off forever no matter how far the user scrolls. Core still
 * verifies the observation against the SDK tail and fails closed when the
 * loaded window does not reach live.
 */
export const nativeFollowLiveTarget = ({
  atLiveBottom,
  positionKind,
  latestVisibleEventId,
}: NativeFollowLiveTargetInput): string | undefined => {
  if (!atLiveBottom) return undefined;
  if (positionKind === 'live_bottom') return undefined;
  if (!isValidEventIdHint(latestVisibleEventId)) return undefined;
  return latestVisibleEventId;
};

/** One follow-live attempt per painted tail; a newer tail retries. */
export const nativeFollowLiveAttemptKey = (roomId: string, eventId: string): string =>
  `${roomId}:${eventId}:follow-live`;

/** Resolve the SDK-projected tail without applying presentation filters. */
export const latestNativeReadEventId = (
  eventIds: readonly (string | undefined)[]
): string | undefined => {
  for (let index = eventIds.length - 1; index >= 0; index -= 1) {
    const eventId = eventIds[index];
    if (isValidEventIdHint(eventId)) return eventId;
  }
  return undefined;
};

/** Core pairs the rendered remote tail with its receipt target. A newer metadata
 * delta arriving before its rows cannot authorize an unseen message. */
export const nativeVisibleReadFrontier = (
  renderedTailEventId: string | undefined,
  frontier: { visibleTailEventId?: string; receiptTailEventId?: string } | undefined
): string | undefined =>
  renderedTailEventId &&
  renderedTailEventId === frontier?.visibleTailEventId &&
  isValidEventIdHint(frontier.receiptTailEventId)
    ? frontier.receiptTailEventId
    : undefined;
