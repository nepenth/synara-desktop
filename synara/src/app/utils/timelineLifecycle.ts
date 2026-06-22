import { Direction, MatrixClient, MatrixEvent, Room, RoomState } from 'matrix-js-sdk';
import type { EventTimeline } from 'matrix-js-sdk/lib/models/event-timeline';

export const getRoomCurrentState = (room: Room): RoomState | undefined =>
  room.currentState ?? room.getLiveTimeline().getState(Direction.Forward);

export const getLoadedLiveTimelineEvents = (room: Room): MatrixEvent[] =>
  room.getLiveTimeline().getEvents() as MatrixEvent[];

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

