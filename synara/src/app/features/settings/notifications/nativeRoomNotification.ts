import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativeRoomNotificationMode = 'all' | 'mentions' | 'mute' | 'default';

export type NativeRoomNotificationSnapshot = {
  roomId: string;
  mode: NativeRoomNotificationMode;
};

export type NativeRoomNotificationsSnapshot = {
  rooms: NativeRoomNotificationSnapshot[];
};

const isMode = (value: unknown): value is NativeRoomNotificationMode =>
  value === 'all' || value === 'mentions' || value === 'mute' || value === 'default';

const isRoomId = (value: unknown): value is string =>
  typeof value === 'string' && value.startsWith('!') && value.includes(':') && value.length > 3;

type NativeRoomNotificationListener = () => void;
const listeners = new Set<NativeRoomNotificationListener>();

export function subscribeNativeRoomNotifications(
  listener: NativeRoomNotificationListener
): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function emitNativeRoomNotificationsChanged(): void {
  listeners.forEach((listener) => listener());
}

export async function nativeRoomNotificationSnapshot(
  roomId: string
): Promise<NativeRoomNotificationSnapshot> {
  const result = await invokeDesktopWithAvailability<NativeRoomNotificationSnapshot>(
    'matrix_room_notification_snapshot',
    { roomId }
  );
  if (
    !result.available ||
    !result.value ||
    !isRoomId(result.value.roomId) ||
    !isMode(result.value.mode)
  ) {
    throw new Error('Native room notification mode is unavailable.');
  }
  return { roomId: result.value.roomId, mode: result.value.mode };
}

export async function nativeRoomNotificationSet(
  roomId: string,
  mode: NativeRoomNotificationMode
): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_room_notification_set',
    { roomId, mode }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native room-notification update is unavailable.');
  }
  emitNativeRoomNotificationsChanged();
}

export async function nativeRoomNotificationsSnapshot(): Promise<NativeRoomNotificationSnapshot[]> {
  const result = await invokeDesktopWithAvailability<NativeRoomNotificationsSnapshot>(
    'matrix_room_notifications_snapshot'
  );
  if (!result.available || !result.value || !Array.isArray(result.value.rooms)) {
    throw new Error('Native room notifications are unavailable.');
  }
  return result.value.rooms.filter(
    (room): room is NativeRoomNotificationSnapshot =>
      !!room && isRoomId(room.roomId) && isMode(room.mode) && room.mode !== 'default'
  );
}
