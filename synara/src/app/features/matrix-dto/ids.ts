/**
 * Opaque Matrix identifier string aliases (product strings, not SDK types).
 */

/** Room id, e.g. `!opaque:example.org`. */
export type RoomId = string;

/** User id, e.g. `@alice:example.org`. */
export type UserId = string;

/** Event id, e.g. `$opaqueEventId`. */
export type EventId = string;

/** Device id, e.g. `DEVICEID`. */
export type DeviceId = string;

/** Opaque product media handle id (not an mxc URI). */
export type MediaHandleId = string;

/** Opaque upload job id. */
export type UploadId = string;

/** Opaque notification candidate id. */
export type NotificationCandidateId = string;

/** Opaque widget / call session id. */
export type WidgetId = string;

/** Timeline virtualization key. */
export type TimelineItemId = string;
