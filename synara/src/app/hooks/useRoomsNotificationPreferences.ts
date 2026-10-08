import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { Icons, IconSrc } from 'folds';
import { useAsyncCallback } from './useAsyncCallback';
import {
  nativeRoomNotificationSet,
  nativeRoomNotificationsSnapshot,
  subscribeNativeRoomNotifications,
  type NativeRoomNotificationMode,
  type NativeRoomNotificationSnapshot,
} from '../features/settings/notifications/nativeRoomNotification';

export type RoomsNotificationPreferences = {
  mute: Set<string>;
  specialMessages: Set<string>;
  allMessages: Set<string>;
};

const RoomsNotificationPreferencesContext = createContext<RoomsNotificationPreferences | null>(
  null
);
export const RoomsNotificationPreferencesProvider = RoomsNotificationPreferencesContext.Provider;

export const useRoomsNotificationPreferencesContext = (): RoomsNotificationPreferences => {
  const preferences = useContext(RoomsNotificationPreferencesContext);

  if (!preferences) {
    throw new Error('No RoomsNotificationPreferences provided!');
  }

  return preferences;
};

const EMPTY_ROOM_NOTIFICATION_PREFERENCES: RoomsNotificationPreferences = {
  mute: new Set(),
  specialMessages: new Set(),
  allMessages: new Set(),
};

const preferencesFromNativeRooms = (
  rooms: NativeRoomNotificationSnapshot[]
): RoomsNotificationPreferences => {
  const pref: RoomsNotificationPreferences = {
    mute: new Set(),
    specialMessages: new Set(),
    allMessages: new Set(),
  };
  rooms.forEach((room) => {
    if (room.mode === 'mute') pref.mute.add(room.roomId);
    if (room.mode === 'mentions') pref.specialMessages.add(room.roomId);
    if (room.mode === 'all') pref.allMessages.add(room.roomId);
  });
  return pref;
};

/**
 * Per-room notification modes as Core resolves them from the account push
 * rules (`matrix_room_notifications_snapshot`). The renderer never re-derives
 * a mode from raw push-rule JSON.
 */
export const useRoomsNotificationPreferences = (): RoomsNotificationPreferences => {
  const [preferences, setPreferences] = useState<RoomsNotificationPreferences>(
    EMPTY_ROOM_NOTIFICATION_PREFERENCES
  );
  const [epoch, setEpoch] = useState(0);

  useEffect(() => subscribeNativeRoomNotifications(() => setEpoch((value) => value + 1)), []);

  useEffect(() => {
    let disposed = false;
    void nativeRoomNotificationsSnapshot()
      .then((rooms) => {
        if (!disposed) setPreferences(preferencesFromNativeRooms(rooms));
      })
      .catch(() => {
        if (!disposed) setPreferences(EMPTY_ROOM_NOTIFICATION_PREFERENCES);
      });
    return () => {
      disposed = true;
    };
  }, [epoch]);

  return preferences;
};

export enum RoomNotificationMode {
  Unset = 'Unset',
  Mute = 'Mute',
  SpecialMessages = 'SpecialMessages',
  AllMessages = 'AllMessages',
}

export const getRoomNotificationMode = (
  preferences: RoomsNotificationPreferences,
  roomId: string
): RoomNotificationMode => {
  if (preferences.mute.has(roomId)) {
    return RoomNotificationMode.Mute;
  }
  if (preferences.specialMessages.has(roomId)) {
    return RoomNotificationMode.SpecialMessages;
  }
  if (preferences.allMessages.has(roomId)) {
    return RoomNotificationMode.AllMessages;
  }

  return RoomNotificationMode.Unset;
};

export const useRoomNotificationPreference = (
  preferences: RoomsNotificationPreferences,
  roomId: string
): RoomNotificationMode =>
  useMemo(() => getRoomNotificationMode(preferences, roomId), [preferences, roomId]);

export const getRoomNotificationModeIcon = (mode?: RoomNotificationMode): IconSrc => {
  if (mode === RoomNotificationMode.Mute) return Icons.BellMute;
  if (mode === RoomNotificationMode.SpecialMessages) return Icons.BellPing;
  if (mode === RoomNotificationMode.AllMessages) return Icons.BellRing;

  return Icons.Bell;
};

const nativeModeFromRoomNotificationMode = (
  mode: RoomNotificationMode
): NativeRoomNotificationMode => {
  switch (mode) {
    case RoomNotificationMode.AllMessages:
      return 'all';
    case RoomNotificationMode.SpecialMessages:
      return 'mentions';
    case RoomNotificationMode.Mute:
      return 'mute';
    case RoomNotificationMode.Unset:
    default:
      return 'default';
  }
};

/** Core writes the push rules for the chosen mode. */
export const setRoomNotificationPreference = async (
  roomId: string,
  mode: RoomNotificationMode
): Promise<void> => {
  await nativeRoomNotificationSet(roomId, nativeModeFromRoomNotificationMode(mode));
};

export const useSetRoomNotificationPreference = (roomId: string) => {
  const [modeState, setMode] = useAsyncCallback(
    useCallback(
      // Core replaces the room's rule set as a whole, so callers' previous
      // mode argument is not needed.
      (mode: RoomNotificationMode) => setRoomNotificationPreference(roomId, mode),
      [roomId]
    )
  );

  return {
    modeState,
    setMode,
  };
};
