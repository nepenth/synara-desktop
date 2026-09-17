import type { NotificationReading } from './notificationResponse';

export type RoomNotificationsGroup = {
  roomId: string;
  notifications: NotificationReading[];
};

export type NotificationTimeline = {
  nextToken?: string;
  groups: RoomNotificationsGroup[];
};

export const groupNotifications = (
  notifications: NotificationReading[],
  allowRooms: Set<string>
): RoomNotificationsGroup[] => {
  const groups: RoomNotificationsGroup[] = [];
  notifications.forEach((notification) => {
    if (!allowRooms.has(notification.room_id)) return;

    const groupIndex = groups.length - 1;
    const lastAddedGroup: RoomNotificationsGroup | undefined = groups[groupIndex];
    if (lastAddedGroup && notification.room_id === lastAddedGroup.roomId) {
      lastAddedGroup.notifications.push(notification);
      return;
    }
    groups.push({
      roomId: notification.room_id,
      notifications: [notification],
    });
  });
  return groups;
};

export const notificationGroupsEquivalent = (
  left: RoomNotificationsGroup[],
  right: RoomNotificationsGroup[]
): boolean => {
  if (left.length !== right.length) return false;
  return left.every((group, index) => {
    const other = right[index];
    if (group.roomId !== other.roomId) return false;
    if (group.notifications.length !== other.notifications.length) return false;
    return group.notifications.every(
      (notification, notificationIndex) =>
        notification.event.event_id === other.notifications[notificationIndex].event.event_id
    );
  });
};

export const shouldResetNotificationTimeline = ({
  from,
  hasGroups,
  highlightFilterChanged,
}: {
  from?: string;
  hasGroups: boolean;
  highlightFilterChanged: boolean;
}): boolean => {
  if (from) return false;
  if (highlightFilterChanged) return true;
  return !hasGroups;
};
