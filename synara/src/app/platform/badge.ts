import {
  getBadgeCount,
  summarizeNotifications,
  type BadgeUnreadSource,
  type NotificationSummary,
  type NotificationSummaryInput,
} from '../notifications/badgeSummary';
import { setDesktopBadgeCount } from '../utils/desktop';

export const setPlatformBadgeCount = setDesktopBadgeCount;

export const getPlatformNotificationCount = (
  unreadCounts: Iterable<BadgeUnreadSource>,
  laterActiveCount: number
): number => getBadgeCount(unreadCounts, laterActiveCount);

export const getPlatformNotificationSummary = (
  input: NotificationSummaryInput
): NotificationSummary => summarizeNotifications(input);
