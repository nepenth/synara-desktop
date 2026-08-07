import { useEffect, useState } from 'react';
import { getLoadedLiveTimelineEvents } from '../utils/timelineLifecycle';
import type { EventedRoomReading } from '../utils/roomEvents';
import { RoomEvent } from '../utils/roomEvents';

const getEventReaders = (room: EventedRoomReading, evtId?: string) => {
  if (!evtId) return [];

  // if eventId is locally generated
  // we don't have read receipt for it yet
  if (!evtId.startsWith('$')) return [];

  const liveEvents = getLoadedLiveTimelineEvents(
    room as unknown as Parameters<typeof getLoadedLiveTimelineEvents>[0]
  );
  const userIds: string[] = [];

  for (let i = liveEvents.length - 1; i >= 0; i -= 1) {
    userIds.splice(userIds.length, 0, ...room.getUsersReadUpTo(liveEvents[i]));
    if (liveEvents[i].getId() === evtId) break;
  }

  return [...new Set(userIds)];
};

export const useRoomEventReaders = (room: EventedRoomReading, eventId?: string): string[] => {
  const [readers, setReaders] = useState<string[]>(() => getEventReaders(room, eventId));

  useEffect(() => {
    setReaders(getEventReaders(room, eventId));

    const handleReceipt = (r: { roomId: string }) => {
      if (r.roomId !== room.roomId) return;
      setReaders(getEventReaders(room, eventId));
    };
    const handleTimelineLifecycleChange = () => {
      setReaders(getEventReaders(room, eventId));
    };

    const handleLocalEcho = (r: { roomId: string }, oldEventId?: string) => {
      // update members on local event id replaced
      // with server generated id
      if (r.roomId !== room.roomId || !oldEventId) return;
      if (oldEventId.startsWith('$')) return;
      if (oldEventId !== eventId) return;

      setReaders(getEventReaders(room, eventId));
    };

    room.on(RoomEvent.Receipt, handleReceipt);
    room.on(RoomEvent.LocalEchoUpdated, handleLocalEcho);
    room.on(RoomEvent.TimelineReset, handleTimelineLifecycleChange);
    room.on(RoomEvent.TimelineRefresh, handleTimelineLifecycleChange);
    return () => {
      room.removeListener(RoomEvent.Receipt, handleReceipt);
      room.removeListener(RoomEvent.LocalEchoUpdated, handleLocalEcho);
      room.removeListener(RoomEvent.TimelineReset, handleTimelineLifecycleChange);
      room.removeListener(RoomEvent.TimelineRefresh, handleTimelineLifecycleChange);
    };
  }, [room, eventId]);

  return readers;
};
