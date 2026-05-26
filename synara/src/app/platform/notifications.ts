import {
  getDesktopNotificationPermission,
  requestDesktopNotificationPermission,
  showDesktopNotification,
  type DesktopNotificationPermission,
} from '../utils/desktop';
import {
  normalizeSystemNotificationRequest,
  type SystemNotificationRequest,
} from '../notifications/systemNotification';

export type PlatformNotificationPayload = SystemNotificationRequest;
export type PlatformNotificationPermission = DesktopNotificationPermission;

export const getPlatformNotificationPermission = getDesktopNotificationPermission;
export const requestPlatformNotificationPermission = requestDesktopNotificationPermission;

export const showPlatformNotification = async (
  notification: PlatformNotificationPayload
): Promise<boolean> => {
  const normalized = normalizeSystemNotificationRequest(notification);
  if (!normalized) return false;

  return showDesktopNotification({
    title: normalized.title,
    body: normalized.body,
    route: normalized.route,
  });
};
