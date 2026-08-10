//! Opaque Matrix identifier string types (product strings, not Ruma types).
//!
//! These are plain `String` aliases with documentation. Validation beyond
//! non-empty checks belongs at the host boundary in later phases.

/// Room id, e.g. `!opaque:example.org`.
pub type RoomId = String;

/// User id, e.g. `@alice:example.org`.
pub type UserId = String;

/// Event id, e.g. `$opaqueEventId`.
pub type EventId = String;

/// Device id, e.g. `DEVICEID`.
pub type DeviceId = String;

/// Opaque product media handle id (not an mxc URI).
pub type MediaHandleId = String;

/// Opaque upload job id.
pub type UploadId = String;

/// Opaque notification candidate id.
pub type NotificationCandidateId = String;

/// Timeline virtualization key (often equals `eventId` or a local-echo id).
pub type TimelineItemId = String;
