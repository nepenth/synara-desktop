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

/**
 * Jump-to-last-read is offered for exactly one thing: an unread frontier Core
 * reported that the presenter has not yet placed in the viewport.
 *
 * The pending marker is the single owner of both visibility conditions the
 * product wants ("the room has unread" and "we are not already at last-read"):
 * Core only reports an unread anchor when the room has unread relative to it,
 * and the presenter clears the marker on placement, on an explicit Jump to
 * latest, or when the room changes. Receipt state must not be consulted here:
 * arriving at the live tail auto-sends a receipt, which would hide the action
 * even though the user has not seen the messages between last-read and the
 * tail. Likewise the marker landing in the loaded window is not "already
 * there" - rows can be loaded far outside the viewport.
 */
export const shouldShowJumpToLastRead = (pendingLastRead: string | undefined): boolean =>
  pendingLastRead !== undefined && isValidEventIdHint(pendingLastRead);

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

export const NATIVE_TIMELINE_DEFAULT_ROW_ESTIMATE_PX = 64;
export const NATIVE_TIMELINE_MEDIA_MAX_PX = 480;
export const NATIVE_TIMELINE_STICKER_MAX_PX = 256;
const NATIVE_TIMELINE_MEASURED_SIZE_CACHE_LIMIT = 4000;
const nativeTimelineMeasuredSizes = new Map<string, number>();

export type NativeTimelineRowSizeHint = {
  kind: string;
  grouped: boolean;
  bodyLineCount?: number;
  hasFormattedCode?: boolean;
  messageType?: string;
  mediaWidth?: number;
  mediaHeight?: number;
};

/** Scale Matrix `info.w/h` into the presenter media box so image loads do not reflow. */
export const reservedNativeTimelineMediaSize = (
  width: number | undefined,
  height: number | undefined,
  maxWidth: number,
  maxHeight: number
): { width: number; height: number } | undefined => {
  if (
    typeof width !== 'number' ||
    typeof height !== 'number' ||
    !Number.isFinite(width) ||
    !Number.isFinite(height) ||
    width <= 0 ||
    height <= 0 ||
    maxWidth <= 0 ||
    maxHeight <= 0
  ) {
    return undefined;
  }
  const scale = Math.min(maxWidth / width, maxHeight / height, 1);
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
};

export const nativeTimelineMeasuredSizeKey = (roomId: string, rowKey: string): string =>
  `${roomId}\0${rowKey}`;

export const rememberNativeTimelineMeasuredSize = (key: string, size: number): void => {
  if (!Number.isFinite(size) || size <= 0) return;
  if (nativeTimelineMeasuredSizes.has(key)) nativeTimelineMeasuredSizes.delete(key);
  nativeTimelineMeasuredSizes.set(key, Math.round(size));
  if (nativeTimelineMeasuredSizes.size > NATIVE_TIMELINE_MEASURED_SIZE_CACHE_LIMIT) {
    const oldest = nativeTimelineMeasuredSizes.keys().next().value;
    if (oldest !== undefined) nativeTimelineMeasuredSizes.delete(oldest);
  }
};

export const nativeTimelineMeasuredSize = (key: string): number | undefined =>
  nativeTimelineMeasuredSizes.get(key);

const chromeHeight = (grouped: boolean): number => (grouped ? 32 : 56);

/** Kind-aware fallback used until `measureElement` records a real row height. */
export const estimateNativeTimelineRowSize = (hint: NativeTimelineRowSizeHint): number => {
  const chrome = chromeHeight(hint.grouped);
  switch (hint.kind) {
    case 'date_separator':
    case 'read_marker':
    case 'unread_marker':
    case 'timeline_start':
    case 'pagination':
      return 40;
    case 'membership':
    case 'state':
    case 'other':
      return 40;
    case 'redacted':
    case 'encrypted_unavailable':
      return hint.grouped ? 44 : 64;
    case 'call':
      return 72;
    case 'poll':
      return 140;
    case 'sticker': {
      const media =
        reservedNativeTimelineMediaSize(
          hint.mediaWidth,
          hint.mediaHeight,
          NATIVE_TIMELINE_STICKER_MAX_PX,
          NATIVE_TIMELINE_STICKER_MAX_PX
        )?.height ?? 96;
      return chrome + media;
    }
    case 'message': {
      if (hint.messageType === 'image' || hint.messageType === 'video') {
        const media =
          reservedNativeTimelineMediaSize(
            hint.mediaWidth,
            hint.mediaHeight,
            NATIVE_TIMELINE_MEDIA_MAX_PX,
            NATIVE_TIMELINE_MEDIA_MAX_PX
          )?.height ?? 180;
        return chrome + media;
      }
      if (hint.messageType === 'audio') return chrome + 48;
      if (hint.messageType === 'file') return chrome + 40;
      if (hint.hasFormattedCode) return chrome + 140;
      const lines = Math.min(8, Math.max(1, hint.bodyLineCount ?? 1));
      return chrome + lines * 24;
    }
    default:
      return NATIVE_TIMELINE_DEFAULT_ROW_ESTIMATE_PX;
  }
};
