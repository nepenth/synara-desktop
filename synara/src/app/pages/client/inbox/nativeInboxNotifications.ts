import { invokeDesktopWithAvailability } from '../../../utils/desktop';
import {
  normalizeNotificationsResponse,
  type NotificationsResponseReading,
} from './notificationResponse';

export type InboxNotificationsQuery = {
  from?: string;
  limit?: number;
  only?: 'highlight';
};

type InboxNotificationsInvoke = typeof invokeDesktopWithAvailability;

export async function fetchNativeInboxNotifications(
  query: InboxNotificationsQuery,
  invoke: InboxNotificationsInvoke = invokeDesktopWithAvailability
): Promise<NotificationsResponseReading> {
  const result = await invoke<unknown>('matrix_inbox_notifications', query);
  if (!result.available) {
    throw new Error('Native notifications are unavailable.');
  }
  return normalizeNotificationsResponse(result.value);
}
