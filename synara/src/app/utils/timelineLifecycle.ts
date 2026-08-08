import type { MatrixClientReading, MatrixEventReading, RoomReading } from './room';

export const ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS = 10 * 60 * 1000;
export const TIMELINE_BOTTOM_TOLERANCE_PX = 1;

export const getTimelineBottomGap = (
  scrollHeight: number,
  scrollTop: number,
  viewportHeight: number
): number => scrollHeight - scrollTop - viewportHeight;

export const isTimelineViewportAtBottom = (
  scrollHeight: number,
  scrollTop: number,
  viewportHeight: number,
  tolerance = TIMELINE_BOTTOM_TOLERANCE_PX
): boolean => getTimelineBottomGap(scrollHeight, scrollTop, viewportHeight) <= tolerance;

type RoomTimelineViewportSnapshot = {
  atBottom: boolean;
  liveTailEventId?: string;
  updatedAtMs?: number;
};

type RoomTimelineViewportRestoreOptions = {
  hasUnread: boolean;
  nowMs: number;
  currentLiveTailEventId?: string;
  maxAgeMs?: number;
};

export const shouldRestoreRoomTimelineViewport = (
  viewport: RoomTimelineViewportSnapshot | undefined,
  {
    hasUnread,
    nowMs,
    currentLiveTailEventId,
    maxAgeMs = ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS,
  }: RoomTimelineViewportRestoreOptions
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

  return Math.max(0, nowMs - updatedAtMs) <= maxAgeMs;
};

/** Narrow structural projection of a room state mirroring the js-sdk RoomState
 * surface read by Synara: state-event lookups plus the indexed event map. */
type RoomStateReading = {
  getStateEvents(eventType: string): MatrixEventReading[];
  getStateEvents(eventType: string, stateKey: string): MatrixEventReading | null;
  events: Map<string, Map<string, MatrixEventReading>>;
};

export const getRoomCurrentState = (room: RoomReading): RoomStateReading | undefined =>
  (room.currentState ?? room.getLiveTimeline().getState('f')) as RoomStateReading | undefined;

export const getLoadedLiveTimelineEvents = (room: RoomReading): MatrixEventReading[] =>
  room.getLiveTimeline().getEvents();

export const getLoadedLiveTailEventId = (room: RoomReading): string | undefined => {
  const events = getLoadedLiveTimelineEvents(room);
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const eventId = events[index]?.getId();
    if (eventId) return eventId;
  }
  return undefined;
};

export const getLatestReceiptEventFromEvents = (
  events: MatrixEventReading[],
  readEventId?: string
): MatrixEventReading | undefined => {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (!event) continue;
    if (event.getId() === readEventId) return undefined;
    if (!event.isSending()) return event;
  }

  return undefined;
};

/** Narrow structural projection of a latest-timeline result: just the events
 * accessor Synara reads off the resolved js-sdk EventTimeline. */
type LatestTimelineReading = {
  getEvents(): MatrixEventReading[];
};

/** MatrixClient reading extended with the latest-timeline accessor (satisfied
 * by the js-sdk MatrixClient at runtime). */
type LatestTimelineClientReading = MatrixClientReading & {
  getLatestTimeline(timelineSet: unknown): Promise<LatestTimelineReading | null>;
};

/** RoomReading extended with the unfiltered timeline set accessor used only by
 * the latest-timeline lookup (satisfied by the js-sdk Room at runtime). */
type RoomWithTimelineSetReading = RoomReading & {
  getUnfilteredTimelineSet(): unknown;
};

export const getLatestRoomTimeline = async (
  mx: LatestTimelineClientReading,
  room: RoomWithTimelineSetReading
): Promise<LatestTimelineReading | undefined> => {
  try {
    return (await mx.getLatestTimeline(room.getUnfilteredTimelineSet())) ?? undefined;
  } catch {
    return undefined;
  }
};
