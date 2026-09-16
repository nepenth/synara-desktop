import { invokeDesktopWithAvailability } from '../../utils/desktop';

export type NativeTimelineTimestampToEventReadback = {
  roomId: string;
  eventId: string;
  originServerTs: number;
};

const acceptTimestampToEventReadback = (
  value: unknown
): NativeTimelineTimestampToEventReadback | undefined => {
  if (!value || typeof value !== 'object') return undefined;
  const readback = value as NativeTimelineTimestampToEventReadback;
  if (
    typeof readback.roomId !== 'string' ||
    readback.roomId.length === 0 ||
    typeof readback.eventId !== 'string' ||
    readback.eventId.length === 0 ||
    typeof readback.originServerTs !== 'number' ||
    !Number.isFinite(readback.originServerTs)
  ) {
    return undefined;
  }
  return readback;
};

export async function timestampToEventWithNativeOwner(
  roomId: string,
  timestampMs: number
): Promise<NativeTimelineTimestampToEventReadback> {
  const result = await invokeDesktopWithAvailability<NativeTimelineTimestampToEventReadback>(
    'matrix_timeline_timestamp_to_event',
    { roomId, timestampMs }
  );
  const readback = result.available ? acceptTimestampToEventReadback(result.value) : undefined;
  if (!readback) {
    throw new Error('Could not find a message at that time.');
  }
  return readback;
}
