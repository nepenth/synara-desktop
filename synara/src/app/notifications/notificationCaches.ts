import type { UnreadInfo } from '../../types/matrix/room';
import { createBoundedLruMap, createBoundedLruSet } from '../utils/boundedLru';

export const NOTIFIED_EVENT_IDS_MAX = 500;
export const UNREAD_NOTIFICATION_CACHE_MAX = 200;

export const notifiedEventIdsCache = createBoundedLruSet(NOTIFIED_EVENT_IDS_MAX);
export const unreadNotificationCache = createBoundedLruMap<string, UnreadInfo>(
  UNREAD_NOTIFICATION_CACHE_MAX
);

export const clearNotificationCaches = (): void => {
  notifiedEventIdsCache.clear();
  unreadNotificationCache.clear();
};