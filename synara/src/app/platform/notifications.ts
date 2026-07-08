import {
  getDesktopNotificationPermission,
  isSynaraDesktop,
  listen,
  requestDesktopNotificationPermission,
  showDesktopNotification,
  type DesktopNotificationActionEventPayload,
  type DesktopNotificationPermission,
  type DesktopUnlisten,
} from '../utils/desktop';
import {
  normalizeSystemNotificationRequest,
  type SystemNotificationRequest,
} from '../notifications/systemNotification';

export type PlatformNotificationPayload = SystemNotificationRequest;
export type PlatformNotificationPermission = DesktopNotificationPermission;
export type PlatformNotificationActionEventPayload = DesktopNotificationActionEventPayload;

export const PLATFORM_NOTIFICATION_ACTION_EVENT = 'synara://notification-action';

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
    actions: normalized.actions,
    actionContext: normalized.actionContext,
  });
};

export const registerPlatformNotificationActionListener = async (
  handler: (payload: PlatformNotificationActionEventPayload) => void
): Promise<DesktopUnlisten | undefined> => {
  if (!isSynaraDesktop()) return undefined;

  return listen<DesktopNotificationActionEventPayload>(
    PLATFORM_NOTIFICATION_ACTION_EVENT,
    (event) => {
      handler(event.payload);
    }
  );
};
