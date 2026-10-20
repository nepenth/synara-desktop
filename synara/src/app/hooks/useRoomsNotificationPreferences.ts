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
  /**
   * The latest load failed. The sets hold the last known modes (empty if
   * none ever loaded), so "Default" may not be the room's real mode.
   */
  loadFailed?: boolean;
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

/** Retry delay after a failed load, so a transient error does not stick. */
export const ROOM_NOTIFICATION_PREFERENCES_RETRY_MS = 15_000;

/**
 * Fold one load outcome into the preferences. A failed load keeps the last
 * known modes instead of resetting every room to "Default", which would make
 * muted rooms look unmuted.
 */
export const nextRoomsNotificationPreferences = (
  current: RoomsNotificationPreferences,
  outcome: { ok: true; rooms: NativeRoomNotificationSnapshot[] } | { ok: false }
): RoomsNotificationPreferences =>
  outcome.ok ? preferencesFromNativeRooms(outcome.rooms) : { ...current, loadFailed: true };

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
    let retry: ReturnType<typeof setTimeout> | undefined;
    void nativeRoomNotificationsSnapshot()
      .then((rooms) => {
        if (!disposed)
          setPreferences((current) =>
            nextRoomsNotificationPreferences(current, { ok: true, rooms })
          );
      })
      .catch(() => {
        if (disposed) return;
        setPreferences((current) => nextRoomsNotificationPreferences(current, { ok: false }));
        retry = setTimeout(
          () => setEpoch((value) => value + 1),
          ROOM_NOTIFICATION_PREFERENCES_RETRY_MS
        );
      });
    return () => {
      disposed = true;
      if (retry !== undefined) clearTimeout(retry);
    };
  }, [epoch]);

  return preferences;
};

/** Whether the per-room modes failed to load; false outside the provider. */
export const useRoomsNotificationPreferencesLoadFailed = (): boolean =>
  useContext(RoomsNotificationPreferencesContext)?.loadFailed === true;

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
