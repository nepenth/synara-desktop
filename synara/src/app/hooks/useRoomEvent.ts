import { useCallback, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useMatrixClient } from './useMatrixClient';
import type { MatrixEventReading } from '../utils/room';
import type { EventedRoomReading } from '../utils/roomEvents';
import { eventFromWire, type RoomEventReading } from '../utils/nativeEventAdapter';

export type { RoomEventReading } from '../utils/nativeEventAdapter';
export type { RoomEventUnsignedReading } from '../utils/nativeEventAdapter';

/** Room projection with a local event resolver (real js-sdk Room satisfies this). */
export type RoomEventSourceReading = EventedRoomReading & {
  findEventById(eventId: string): MatrixEventReading | undefined;
};

const useFetchEvent = (room: RoomEventSourceReading, eventId: string) => {
  const mx = useMatrixClient();

  const fetchEventCallback = useCallback(async () => {
    const evt = await mx.fetchRoomEvent(room.roomId, eventId);
    if (!evt) {
      throw new Error('Room event not found');
    }
    return eventFromWire(evt, room.roomId);
  }, [mx, room.roomId, eventId]);

  return fetchEventCallback;
};

/**
 * @param room
 * @param eventId
 * @returns `RoomEventReading`, `undefined` means loading, `null` means failure
 */
export const useRoomEvent = (
  room: RoomEventSourceReading,
  eventId: string,
  getLocally?: () => MatrixEventReading | undefined
) => {
  const event = useMemo(() => {
    if (getLocally) return getLocally();
    return room.findEventById(eventId);
  }, [room, eventId, getLocally]);

  const fetchEvent = useFetchEvent(room, eventId);

  const { data, error } = useQuery({
    enabled: event === undefined,
    queryKey: [room.roomId, eventId],
    queryFn: fetchEvent,
    staleTime: Infinity,
    gcTime: 60 * 60 * 1000, // 1hour
  });

  if (event) return event as RoomEventReading;
  if (data) return data;
  if (error) return null;

  return undefined;
};
