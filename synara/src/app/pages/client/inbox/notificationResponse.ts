export type NotificationEventReading = {
  event_id: string;
  type: string;
  sender: string;
  origin_server_ts: number;
  content: Record<string, any>;
  unsigned?: Record<string, any>;
  [key: string]: any;
};

export type NotificationReading = {
  event: NotificationEventReading;
  room_id: string;
  ts?: number;
  read?: boolean;
  [key: string]: unknown;
};

export type NotificationsResponseReading = {
  notifications: NotificationReading[];
  next_token?: string;
};

export class InvalidNotificationsResponseError extends Error {
  constructor() {
    super('The homeserver returned an invalid notifications response.');
    this.name = 'InvalidNotificationsResponseError';
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === 'object' && !Array.isArray(value);

const isNotification = (value: unknown): value is NotificationReading => {
  if (!isRecord(value) || typeof value.room_id !== 'string' || !isRecord(value.event)) {
    return false;
  }
  const { event } = value;
  return (
    typeof event.event_id === 'string' &&
    typeof event.type === 'string' &&
    typeof event.sender === 'string' &&
    typeof event.origin_server_ts === 'number' &&
    Number.isFinite(event.origin_server_ts) &&
    isRecord(event.content) &&
    (event.unsigned === undefined || isRecord(event.unsigned)) &&
    (value.ts === undefined || (typeof value.ts === 'number' && Number.isFinite(value.ts))) &&
    (value.read === undefined || typeof value.read === 'boolean')
  );
};

/**
 * Validate the untrusted `/notifications` response at the HTTP boundary.
 * A malformed envelope is a real load failure rather than a false empty Inbox.
 * Individual malformed entries are discarded so valid siblings remain usable.
 */
export const normalizeNotificationsResponse = (value: unknown): NotificationsResponseReading => {
  if (!isRecord(value) || !Array.isArray(value.notifications)) {
    throw new InvalidNotificationsResponseError();
  }
  const notifications = value.notifications.filter(isNotification);
  return {
    notifications,
    next_token: typeof value.next_token === 'string' ? value.next_token : undefined,
  };
};
