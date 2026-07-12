import { Direction, MatrixClient, MatrixEvent, Room, RoomState } from 'matrix-js-sdk';
import type { EventTimeline } from 'matrix-js-sdk/lib/models/event-timeline';

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

export const getRoomCurrentState = (room: Room): RoomState | undefined =>
  room.currentState ?? room.getLiveTimeline().getState(Direction.Forward);

export const getLoadedLiveTimelineEvents = (room: Room): MatrixEvent[] =>
  room.getLiveTimeline().getEvents() as MatrixEvent[];

export const getLoadedLiveTailEventId = (room: Room): string | undefined => {
  const events = getLoadedLiveTimelineEvents(room);
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const eventId = events[index]?.getId();
    if (eventId) return eventId;
  }
  return undefined;
};

export const getLatestReceiptEventFromEvents = (
  events: MatrixEvent[],
  readEventId?: string
): MatrixEvent | undefined => {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (!event) continue;
    if (event.getId() === readEventId) return undefined;
    if (!event.isSending()) return event;
  }

  return undefined;
};

export const getLatestRoomTimeline = async (
  mx: MatrixClient,
  room: Room
): Promise<EventTimeline | undefined> => {
  try {
    return (await mx.getLatestTimeline(room.getUnfilteredTimelineSet())) ?? undefined;
  } catch {
    return undefined;
  }
};
