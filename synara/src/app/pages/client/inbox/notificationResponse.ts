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

export class HomeserverNotificationsError extends Error {
  readonly errcode?: string;

  constructor(errcode?: string, message?: string) {
    super(message || 'The homeserver returned a notifications error.');
    this.name = errcode || 'HomeserverNotificationsError';
    this.errcode = errcode;
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
 *
 * Two accommodations keep this honest against real-world servers:
 * - an error-shaped envelope (`errcode`/`error`) surfaces the homeserver's
 *   own diagnostic instead of the generic validation message;
 * - an explicit `notifications: null` reads as an empty timeline. Some
 *   homeservers encode an empty list as null (Go nil-slice JSON), which is not
 *   a failure. A missing key is still malformed.
 */
export const normalizeNotificationsResponse = (value: unknown): NotificationsResponseReading => {
  if (!isRecord(value)) {
    throw new InvalidNotificationsResponseError();
  }
  const errcode = typeof value.errcode === 'string' ? value.errcode : undefined;
  const serverMessage = typeof value.error === 'string' ? value.error : undefined;
  if (errcode !== undefined || serverMessage !== undefined) {
    throw new HomeserverNotificationsError(errcode, serverMessage);
  }
  if (value.notifications === null) {
    return {
      notifications: [],
      next_token: typeof value.next_token === 'string' ? value.next_token : undefined,
    };
  }
  if (!Array.isArray(value.notifications)) {
    throw new InvalidNotificationsResponseError();
  }
  const notifications = value.notifications.filter(isNotification);
  return {
    notifications,
    next_token: typeof value.next_token === 'string' ? value.next_token : undefined,
  };
};
