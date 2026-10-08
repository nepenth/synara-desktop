import { useMemo } from 'react';
import type { RoomPermissionCapabilities } from '../features/matrix-dto/generated';
import type {
  IPowerLevels,
  PowerLevelActions,
  PowerLevelNotificationsAction,
} from './usePowerLevels';

/**
 * What the signed-in user may do in a room. Core evaluates the room's power
 * levels with the room version's rules (creators outrank every level from
 * room v12) and ships `RoomPermissionCapabilities`; this API only reads it.
 * Without capabilities every check is denied.
 */
export type RoomPermissionsAPI = {
  event: (type: string) => boolean;
  stateEvent: (type: string) => boolean;
  action: (action: PowerLevelActions) => boolean;
  notificationAction: (action: PowerLevelNotificationsAction) => boolean;
};

const DENY_ALL: RoomPermissionsAPI = {
  event: () => false,
  stateEvent: () => false,
  action: () => false,
  notificationAction: () => false,
};

const MESSAGE_EVENT_CAPABILITY: Record<string, keyof RoomPermissionCapabilities> = {
  'm.room.message': 'canSendMessage',
  'm.reaction': 'canReact',
};

const STATE_EVENT_CAPABILITY: Record<string, keyof RoomPermissionCapabilities> = {
  'm.room.name': 'canChangeName',
  'm.room.topic': 'canChangeTopic',
  'm.room.avatar': 'canChangeAvatar',
  'm.room.canonical_alias': 'canChangeCanonicalAlias',
  'm.room.history_visibility': 'canChangeHistoryVisibility',
  'm.room.join_rules': 'canChangeJoinRules',
  'm.room.encryption': 'canEnableEncryption',
  'm.room.power_levels': 'canChangePowerLevels',
  'm.room.pinned_events': 'canChangePinnedEvents',
  'm.room.tombstone': 'canUpgradeRoom',
  'm.space.child': 'canManageSpaceChildren',
};

const ACTION_CAPABILITY: Partial<Record<PowerLevelActions, keyof RoomPermissionCapabilities>> = {
  invite: 'canInvite',
  kick: 'canKick',
  ban: 'canBan',
  redact: 'canRedactOthers',
};

export const getRoomPermissionsAPI = (powerLevels: IPowerLevels): RoomPermissionsAPI => {
  const caps = powerLevels.nativeCapabilities;
  if (powerLevels.nativeUnavailable || !caps) return DENY_ALL;
  const flag = (key: keyof RoomPermissionCapabilities | undefined): boolean =>
    key !== undefined && caps[key] === true;

  return {
    event: (type) => {
      const known = MESSAGE_EVENT_CAPABILITY[type];
      if (known) return flag(known);
      return caps.eventAllowed[type] ?? caps.eventsDefaultAllowed;
    },
    stateEvent: (type) => {
      const known = STATE_EVENT_CAPABILITY[type];
      if (known) return flag(known);
      return caps.eventAllowed[type] ?? caps.stateDefaultAllowed;
    },
    // `historical` is not a Matrix permission any server enforces; only the
    // actions Core evaluates can be granted.
    action: (action) => flag(ACTION_CAPABILITY[action]),
    notificationAction: (action) => action === 'room' && caps.canNotifyRoom,
  };
};

export const useRoomPermissions = (powerLevels: IPowerLevels): RoomPermissionsAPI =>
  useMemo(() => getRoomPermissionsAPI(powerLevels), [powerLevels]);
