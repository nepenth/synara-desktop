import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { ConditionKind, IPushRules, PushRuleKind, asPushRuleClient } from '../utils/pushRules';
import type { MatrixClientReading } from '../utils/room';
import { Icons, IconSrc } from 'folds';
import { AccountDataEvent } from '../../types/matrix/accountData';
import { useAccountData } from './useAccountData';
import { isRoomId } from '../utils/matrix';
import {
  getNotificationMode,
  getNotificationModeActions,
  NotificationMode,
} from './useNotificationMode';
import { useAsyncCallback } from './useAsyncCallback';
import { useMatrixClient } from './useMatrixClient';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
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

export const useRoomsNotificationPreferences = (): RoomsNotificationPreferences => {
  const nativeSession = isNativeMatrixSession();
  const pushRules = useAccountData(AccountDataEvent.PushRules)?.getContent<IPushRules>();
  const [nativePreferences, setNativePreferences] = useState<RoomsNotificationPreferences>(
    EMPTY_ROOM_NOTIFICATION_PREFERENCES
  );
  const [nativeEpoch, setNativeEpoch] = useState(0);

  useEffect(() => {
    if (!nativeSession) return undefined;
    return subscribeNativeRoomNotifications(() => setNativeEpoch((value) => value + 1));
  }, [nativeSession]);

  useEffect(() => {
    if (!nativeSession) return undefined;
    let disposed = false;
    void nativeRoomNotificationsSnapshot()
      .then((rooms) => {
        if (!disposed) setNativePreferences(preferencesFromNativeRooms(rooms));
      })
      .catch(() => {
        if (!disposed) setNativePreferences(EMPTY_ROOM_NOTIFICATION_PREFERENCES);
      });
    return () => {
      disposed = true;
    };
  }, [nativeSession, nativeEpoch]);

  const preferences: RoomsNotificationPreferences = useMemo(() => {
    const global = pushRules?.global;
    const room = global?.room ?? [];
    const override = global?.override ?? [];

    const pref: RoomsNotificationPreferences = {
      mute: new Set(),
      specialMessages: new Set(),
      allMessages: new Set(),
    };

    override.forEach((rule) => {
      if (isRoomId(rule.rule_id) && getNotificationMode(rule.actions) === NotificationMode.OFF) {
        pref.mute.add(rule.rule_id);
      }
    });
    room.forEach((rule) => {
      if (getNotificationMode(rule.actions) === NotificationMode.OFF) {
        pref.specialMessages.add(rule.rule_id);
      }
    });
    room.forEach((rule) => {
      if (getNotificationMode(rule.actions) !== NotificationMode.OFF) {
        pref.allMessages.add(rule.rule_id);
      }
    });

    return pref;
  }, [pushRules]);

  return nativeSession ? nativePreferences : preferences;
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

export const setRoomNotificationPreference = async (
  mx: MatrixClientReading,
  roomId: string,
  mode: RoomNotificationMode,
  previousMode: RoomNotificationMode
): Promise<void> => {
  if (isNativeMatrixSession()) {
    await nativeRoomNotificationSet(roomId, nativeModeFromRoomNotificationMode(mode));
    return;
  }

  const pushRuleClient = asPushRuleClient(mx);

  // remove the old preference
  if (
    previousMode === RoomNotificationMode.AllMessages ||
    previousMode === RoomNotificationMode.SpecialMessages
  ) {
    await pushRuleClient.deletePushRule('global', PushRuleKind.RoomSpecific, roomId);
  }
  if (previousMode === RoomNotificationMode.Mute) {
    await pushRuleClient.deletePushRule('global', PushRuleKind.Override, roomId);
  }

  // set new preference
  if (mode === RoomNotificationMode.Unset) {
    return;
  }

  if (mode === RoomNotificationMode.Mute) {
    await pushRuleClient.addPushRule('global', PushRuleKind.Override, roomId, {
      conditions: [
        {
          kind: ConditionKind.EventMatch,
          key: 'room_id',
          pattern: roomId,
        },
      ],
      actions: getNotificationModeActions(NotificationMode.OFF),
    });
    return;
  }

  await pushRuleClient.addPushRule('global', PushRuleKind.RoomSpecific, roomId, {
    actions:
      mode === RoomNotificationMode.AllMessages
        ? getNotificationModeActions(NotificationMode.NotifyLoud)
        : getNotificationModeActions(NotificationMode.OFF),
  });
};

export const useSetRoomNotificationPreference = (roomId: string) => {
  const mx = useMatrixClient();

  const [modeState, setMode] = useAsyncCallback(
    useCallback(
      (mode: RoomNotificationMode, previousMode: RoomNotificationMode) =>
        setRoomNotificationPreference(mx, roomId, mode, previousMode),
      [mx, roomId]
    )
  );

  return {
    modeState,
    setMode,
  };
};
