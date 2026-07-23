//! Compile-only public API-shape probes for `matrix-sdk` / `matrix-sdk-ui` 0.18.0.
//!
//! Each probe forces the compiler to resolve named public types and function
//! signatures. **These probes do not prove runtime or network semantics.** They
//! never connect to a homeserver, open a store, or handle secrets.
//!
//! Upstream pin: tag `matrix-sdk-0.18.0`, commit
//! `1c44fb66214667c6d00acaf72ab592493653708b`.
//!
//! Modules group stable capability areas for P0.3b. P0.3a probe IDs are preserved.

#![forbid(unsafe_code)]
#![deny(unreachable_pub)]

pub mod account_data;
pub mod auth;
pub mod e2ee;
pub mod media;
pub mod messaging;
pub mod notifications;
pub mod p0_3a;
pub mod room_ops;
pub mod search;
pub mod spaces;
pub mod sync_rooms;
pub mod threads;
pub mod timeline;

/// Probe catalog (stable IDs used by capability docs). Successful compile probes only.
pub const PROBE_IDS: &[&str] = &[
    // P0.3a foundation
    "P0.3a-client-type",
    "P0.3a-client-builder-type",
    "P0.3a-client-builder-fn",
    "P0.3a-room-type",
    "P0.3a-room-room-id-fn",
    "P0.3a-sync-service-type",
    "P0.3a-sync-service-builder-fn",
    "P0.3a-room-list-service-type",
    "P0.3a-timeline-type",
    "P0.3a-timeline-builder-type",
    "P0.3a-room-ext-timeline-builder",
    // Auth / discovery / session
    "P0.3b-client-builder-homeserver-url",
    "P0.3b-client-builder-server-name",
    "P0.3b-client-builder-server-name-or-url",
    "P0.3b-auth-session-type",
    "P0.3b-client-matrix-auth",
    "P0.3b-matrix-auth-login-username",
    "P0.3b-matrix-auth-get-login-types",
    "P0.3b-client-restore-session",
    "P0.3b-client-logout",
    // Sync / room list / room lookup
    "P0.3b-sync-service-start",
    "P0.3b-sync-service-room-list-service",
    "P0.3b-client-sync-once",
    "P0.3b-room-list-service-all-rooms",
    "P0.3b-client-get-room",
    "P0.3b-client-rooms",
    "P0.3b-client-joined-rooms",
    "P0.3b-room-state",
    // Timeline
    "P0.3b-timeline-subscribe",
    "P0.3b-timeline-paginate-backwards",
    "P0.3b-timeline-paginate-forwards",
    "P0.3b-timeline-builder-with-focus",
    "P0.3b-timeline-focus-type",
    // Messaging
    "P0.3b-room-send",
    "P0.3b-room-send-state-event",
    "P0.3b-room-redact",
    "P0.3b-room-typing-notice",
    "P0.3b-room-send-single-receipt",
    "P0.3b-timeline-mark-as-read",
    // Media
    "P0.3b-media-type",
    "P0.3b-client-media",
    "P0.3b-media-upload",
    "P0.3b-media-get-media-content",
    // Room ops / profile
    "P0.3b-client-create-room",
    "P0.3b-client-join-room-by-id",
    "P0.3b-room-leave",
    "P0.3b-room-invite-user-by-id",
    "P0.3b-room-members",
    "P0.3b-room-set-name",
    "P0.3b-room-update-power-levels",
    "P0.3b-room-ban-user",
    "P0.3b-account-type",
    "P0.3b-client-account",
    "P0.3b-account-get-display-name",
    // Account data
    "P0.3b-account-account-data-raw",
    "P0.3b-account-set-account-data-raw",
    // Notifications
    "P0.3b-notification-settings-type",
    "P0.3b-client-notification-settings",
    "P0.3b-account-push-rules",
    "P0.3b-notification-set-room-notification-mode",
    // E2EE / devices / recovery
    "P0.3b-encryption-type",
    "P0.3b-client-encryption",
    "P0.3b-encryption-get-user-devices",
    "P0.3b-encryption-cross-signing-status",
    "P0.3b-encryption-recovery",
    "P0.3b-encryption-backups",
    "P0.3b-client-devices",
    "P0.3b-encryption-get-verification-request",
    // Search (stable high-level only)
    "P0.3b-client-search-users",
    "P0.3b-room-directory-search-type",
    "P0.3b-room-directory-search-new",
    // Spaces
    "P0.3b-client-joined-space-rooms",
    "P0.3b-space-service-type",
    "P0.3b-space-service-new",
    "P0.3b-space-room-list-type",
    "P0.3b-space-service-space-room-list",
    // Threads / relations
    "P0.3b-room-relations",
    "P0.3b-thread-list-service-type",
    "P0.3b-room-ext-thread-list-service",
    "P0.3b-timeline-is-threaded",
];

/// Run every successful compile-only probe so `cargo test` monomorphizes shapes.
///
/// Still compile-only: no network, no stores, no secrets.
pub fn run_all_probes() {
    p0_3a::run_all();
    auth::run_all();
    sync_rooms::run_all();
    timeline::run_all();
    messaging::run_all();
    media::run_all();
    room_ops::run_all();
    account_data::run_all();
    notifications::run_all();
    e2ee::run_all();
    search::run_all();
    spaces::run_all();
    threads::run_all();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_api_shape_probes_compile_and_run() {
        // Compile-only API-shape probe execution; does not prove runtime/network semantics.
        run_all_probes();
        assert_eq!(PROBE_IDS.len(), 80);
        // Uniqueness
        let mut seen = std::collections::BTreeSet::new();
        for id in PROBE_IDS {
            assert!(seen.insert(*id), "duplicate probe id: {id}");
        }
        assert!(p0_3a::probe_client_type().contains("Client"));
        assert!(p0_3a::probe_room_type().contains("Room"));
        assert!(p0_3a::probe_sync_service_type().contains("SyncService"));
        assert!(p0_3a::probe_timeline_type().contains("Timeline"));
        assert!(auth::probe_auth_session_type().contains("AuthSession"));
        assert!(media::probe_media_type().contains("Media"));
        assert!(e2ee::probe_encryption_type().contains("Encryption"));
        assert!(spaces::probe_space_service_type().contains("SpaceService"));
        assert!(threads::probe_thread_list_service_type().contains("ThreadListService"));
    }
}
