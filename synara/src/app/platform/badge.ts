import {
  getBadgeCount,
  summarizeNotifications,
  type NotificationSummary,
  type NotificationSummaryInput,
} from '../notifications/badgeSummary';
import { setDesktopBadgeCount } from '../utils/desktop';

export const setPlatformBadgeCount = setDesktopBadgeCount;

export const getPlatformNotificationCount = (input: NotificationSummaryInput): number =>
  getBadgeCount(input);

export const getPlatformNotificationSummary = (
  input: NotificationSummaryInput
): NotificationSummary => summarizeNotifications(input);
