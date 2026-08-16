//! Matrix integration surface for the production Tauri crate.
//!
//! P1.3: versioned IPC schema foundation.
//! P1.4: Synara-owned domain DTOs (product projections for IPC bodies).
//! P2.1: Matrix supervisor actor foundation (pure lifecycle + generation).
//! P2.2: Per-account store paths + encryption-key vault foundation.
//! P2.3: SDK client builder (unauthenticated Client construction only).
//! P2.4: Task supervision and cancellation (generation-stamped async work).
//! P2.5: Privacy-filtered diagnostics and health model.
//! P2.6: Destructive lifecycle (logout, local wipe, failed-store recovery).
//! P3.1: Discovery and login-flow service (harness; no login execution).
//! P3.2 / V-AUTH.2: Password login + device naming (desktop password-only).
//! P3.4: Interactive auth (UIA) multi-stage foundation (harness; no secrets).
//! P3.5: Session secret / refresh-token persistence foundation (host vault only).
//! P3.8: Remote logout flow + recovery UX copy keys (harness).
//! P4.1: Sync service readiness / reconnect model (harness foundation).
//! P4.2: Room-list snapshot and delta projection (harness foundation).
//! P4.5: Space hierarchy / filters / parents (harness foundation).
//! P4.6: Room member / power-level index foundation (harness).
//! P4.7: Presence stream index foundation (harness; no SDK presence).
//! P4.8: Route / deep-link resolution foundation (harness).
//! P5.1: Timeline registry and lifecycle (harness foundation).
//! P5.2: Timeline snapshot / ordered-diff projection (harness foundation).
//! P5.3: Timeline pagination state machine foundation.
//! P5.4: Timeline focus / event-context opening foundation.
//! P5.5: Read markers / unread positioning foundation (harness).
//! P5.10: UTD / decryption update propagation foundation.
//! P5.6: Relations / reactions / replaces index foundation (harness).
//! P5.7: Poll and room state/membership projection foundation (harness).
//! P5.8: Thread list / summary index foundation (harness).
//! P6.1: Outbound send queue + local-echo foundation (harness; no Room::send).
//! P7.4: Outbound attachment / media send queue foundation (handle only; no bytes).
//! P6.2: Receipt index foundation (harness; no SDK send_receipt).
//! P6.3: Typing index foundation (harness; no SDK typing send).
//! P6.4: Media upload queue foundation (metadata only; no bytes / no SDK upload).
//! P7.3: Media cache / retention index foundation (metadata only; no disk I/O).
//! P7.2: Media download / local-delivery queue foundation (metadata only; no bytes).
//! P6.5: Room profile / alias / directory / join-history / upgrade foundation.
//! P6.6: User profile / ignore list foundation (harness; no avatar bytes).
//! P6.8: Search session / result index foundation (harness).
//! P6.10: Public room directory search session foundation (harness).
//! P6.9: Room membership / lifecycle ops queue foundation (harness; no SDK network).
//! P7.1: Notification candidate index foundation (harness; privacy-filtered).
//! P7.5: Save/share/open/drag media export intent foundation (metadata only).
//! P8.1: Security / crypto status projection foundation (harness; no secrets).
//! P9.1: Widget / Element Call session registry foundation (harness).
//! P8.4: Cross-signing / identity state foundation (harness; no key material).
//! P3.7: Legacy-session detection / transition coordinator (clean-break; no JS client).
//! P8.5: Key backup / recovery setup-restore-repair foundation (harness; no secrets).
//! P8.2: Device list / trust projection foundation (harness; no keys).
//! P8.7: UTD retry / encrypted-history recovery foundation (harness).
//! P8.6: Room-key import/export transfer foundation (harness; no key material).
//! P8.8: Crypto-store continuity / corruption handling foundation (harness).
//! P8.3: Verification request inbox + SAS display foundation (harness; no secrets).
//! D0.1–D0.3 add the production desktop login, sync/room-list, and opened-room
//! timeline read path. No dual-backend selector; remaining product slices are
//! migrated incrementally.

pub mod account_data;
pub mod auth;
pub mod backup;
pub mod client_builder;
pub mod cross_signing;
pub mod crypto_store;
pub mod devices;
pub mod diagnostics;
// SNC-P1-2: matrix/dto moved into crates/synara-core; re-export so all
// `crate::matrix::dto::…` paths keep resolving (path-only, no behavior change).
pub use synara_core::dto;
// SNC-P1-3: matrix/ipc moved into crates/synara-core; re-export so all
// `crate::matrix::ipc::…` paths keep resolving (path-only, no behavior change).
pub use synara_core::transport as ipc;
pub mod legacy;
pub mod lifecycle;
pub mod media;
pub mod media_cache;
pub mod media_export;
pub mod members;
pub mod notifications;
pub mod polls;
pub mod presence;
pub mod raw_content;
pub mod receipts;
pub mod relations;
pub mod room_directory;
pub mod room_keys;
pub mod room_list;
pub mod room_ops;
pub mod room_profile;
pub mod routes;
pub mod search;
pub mod secret_storage;
pub mod security;
pub mod send;
pub mod spaces;
pub mod store;
pub mod supervisor;
pub mod sync;
pub mod tasks;
pub mod threads;
pub mod timeline;
pub mod typing;
pub mod unread;
pub mod user_profile;
pub mod utd_recovery;
pub mod verification;

const _: fn() -> &'static str = matrix_ipc_schema_markers;

/// Touch Matrix foundation paths so they remain linked in non-test builds.
pub fn matrix_ipc_schema_markers() -> &'static str {
    let _version = ipc::MATRIX_IPC_PROTOCOL_VERSION;
    let _kinds = ipc::MATRIX_IPC_KINDS.len();
    let _errors = ipc::MatrixIpcErrorCategory::ALL.len();
    let _topics = ipc::StreamTopic::ALL.len();
    let _forbid_media = ipc::FORBID_MEDIA_BYTES_OVER_JSON_IPC;
    let _queue = ipc::MAX_STREAM_QUEUE_DEPTH;
    let _dto = dto::MATRIX_DTO_MARKER;
    let _dto_forbid = dto::FORBID_MEDIA_BYTES_OVER_JSON_IPC;
    let _lifecycles = dto::SessionLifecycle::ALL.len();
    let _memberships = dto::Membership::ALL.len();
    let _supervisor = supervisor::matrix_supervisor_markers();
    let _store = store::matrix_store_markers();
    let _builder = client_builder::matrix_client_builder_markers();
    let _tasks = tasks::matrix_tasks_markers();
    let _diagnostics = diagnostics::matrix_diagnostics_markers();
    let _lifecycle = lifecycle::matrix_lifecycle_markers();
    let _account_data = account_data::matrix_account_data_markers();
    let _auth = auth::matrix_auth_markers();
    let _sync = sync::matrix_sync_markers();
    let _room_keys = room_keys::matrix_room_keys_markers();
    let _room_list = room_list::matrix_room_list_markers();
    let _room_directory = room_directory::matrix_room_directory_markers();
    let _routes = routes::matrix_routes_markers();
    let _spaces = spaces::matrix_spaces_markers();
    let _members = members::matrix_members_markers();
    let _timeline = timeline::matrix_timeline_markers();
    let _raw_content = raw_content::matrix_raw_content_markers();
    let _relations = relations::matrix_relations_markers();
    let _polls = polls::matrix_polls_markers();
    let _search = search::matrix_search_markers();
    let _security = security::matrix_security_markers();
    let _legacy = legacy::matrix_legacy_markers();
    let _backup = backup::matrix_backup_markers();
    let _devices = devices::matrix_devices_markers();
    let _verification = verification::matrix_verification_markers();
    let _cross_signing = cross_signing::matrix_cross_signing_markers();
    let _crypto_store = crypto_store::matrix_crypto_store_markers();
    let _send = send::matrix_send_markers();
    let _receipts = receipts::matrix_receipts_markers();
    let _threads = threads::matrix_threads_markers();
    let _typing = typing::matrix_typing_markers();
    let _utd_recovery = utd_recovery::matrix_utd_recovery_markers();
    let _presence = presence::matrix_presence_markers();
    let _media = media::matrix_media_markers();
    let _media_cache = media_cache::matrix_media_cache_markers();
    let _media_export = media_export::matrix_media_export_markers();
    let _notifications = notifications::matrix_notifications_markers();
    let _unread = unread::matrix_unread_markers();
    let _user_profile = user_profile::matrix_user_profile_markers();
    let _room_ops = room_ops::matrix_room_ops_markers();

    let _room_profile = room_profile::matrix_room_profile_markers();
    debug_assert_eq!(_version, 1);
    debug_assert!(_kinds > 0);
    debug_assert!(_errors > 0);
    debug_assert!(_topics > 0);
    debug_assert!(_forbid_media);
    debug_assert!(_queue > 0);
    debug_assert!(_dto_forbid);
    debug_assert!(_lifecycles > 0);
    debug_assert!(_memberships > 0);
    debug_assert_eq!(_dto, "matrix-domain-dtos-p1.4");
    debug_assert_eq!(_supervisor, supervisor::MATRIX_SUPERVISOR_MARKER);
    debug_assert_eq!(_store, store::MATRIX_STORE_MARKER);
    debug_assert_eq!(_builder, client_builder::MATRIX_CLIENT_BUILDER_MARKER);
    debug_assert_eq!(_tasks, tasks::MATRIX_TASKS_MARKER);
    debug_assert_eq!(_diagnostics, diagnostics::MATRIX_DIAGNOSTICS_MARKER);
    debug_assert_eq!(_lifecycle, lifecycle::MATRIX_LIFECYCLE_MARKER);
    debug_assert_eq!(_account_data, account_data::MATRIX_ACCOUNT_DATA_MARKER);
    debug_assert_eq!(_auth, auth::MATRIX_AUTH_MARKER);
    debug_assert_eq!(_sync, sync::MATRIX_SYNC_MARKER);
    debug_assert_eq!(_room_keys, room_keys::MATRIX_ROOM_KEYS_MARKER);
    debug_assert_eq!(_room_list, room_list::MATRIX_ROOM_LIST_MARKER);
    debug_assert_eq!(
        _room_directory,
        room_directory::MATRIX_ROOM_DIRECTORY_MARKER
    );
    debug_assert_eq!(_routes, routes::MATRIX_ROUTES_MARKER);
    debug_assert_eq!(_spaces, spaces::MATRIX_SPACES_MARKER);
    debug_assert_eq!(_members, members::MATRIX_MEMBERS_MARKER);
    debug_assert_eq!(_timeline, timeline::MATRIX_TIMELINE_MARKER);
    debug_assert_eq!(_raw_content, raw_content::MATRIX_RAW_CONTENT_MARKER);
    debug_assert_eq!(_relations, relations::MATRIX_RELATIONS_MARKER);
    debug_assert_eq!(_polls, polls::MATRIX_POLLS_MARKER);
    debug_assert_eq!(_search, search::MATRIX_SEARCH_MARKER);
    debug_assert_eq!(_security, security::MATRIX_SECURITY_MARKER);
    debug_assert_eq!(_legacy, legacy::MATRIX_LEGACY_MARKER);
    debug_assert_eq!(_backup, backup::MATRIX_BACKUP_MARKER);
    debug_assert_eq!(_devices, devices::MATRIX_DEVICES_MARKER);
    debug_assert_eq!(_verification, verification::MATRIX_VERIFICATION_MARKER);
    debug_assert_eq!(_cross_signing, cross_signing::MATRIX_CROSS_SIGNING_MARKER);
    debug_assert_eq!(_crypto_store, crypto_store::MATRIX_CRYPTO_STORE_MARKER);
    debug_assert_eq!(_send, send::MATRIX_SEND_MARKER);
    debug_assert_eq!(_receipts, receipts::MATRIX_RECEIPTS_MARKER);
    debug_assert_eq!(_threads, threads::MATRIX_THREADS_MARKER);
    debug_assert_eq!(_typing, typing::MATRIX_TYPING_MARKER);
    debug_assert_eq!(_utd_recovery, utd_recovery::MATRIX_UTD_RECOVERY_MARKER);
    debug_assert_eq!(_presence, presence::MATRIX_PRESENCE_MARKER);
    debug_assert_eq!(_media, media::MATRIX_MEDIA_MARKER);
    debug_assert_eq!(_media_cache, media_cache::MATRIX_MEDIA_CACHE_MARKER);
    debug_assert_eq!(_media_export, media_export::MATRIX_MEDIA_EXPORT_MARKER);
    debug_assert_eq!(_notifications, notifications::MATRIX_NOTIFICATIONS_MARKER);
    debug_assert_eq!(_unread, unread::MATRIX_UNREAD_MARKER);
    debug_assert_eq!(_user_profile, user_profile::MATRIX_USER_PROFILE_MARKER);
    debug_assert_eq!(_room_ops, room_ops::MATRIX_ROOM_OPS_MARKER);
    debug_assert_eq!(_room_profile, room_profile::MATRIX_ROOM_PROFILE_MARKER);
    "matrix-ipc-protocol-v1+domain-dtos-p1.4+supervisor-p2.1+store-p2.2+client-builder-p2.3+tasks-p2.4+diagnostics-p2.5+lifecycle-p2.6+auth-p3.3+p3.4+session-persist-p3.5+sync-p4.1+room-list-p4.2+routes-p4.8+spaces-p4.5+members-p4.6+timeline-p5.1+diffs-p5.2+pagination-p5.3+search-p6.8+relations-p5.6+send-p6.1+receipts-p6.2+typing-p6.3+media-p6.4+download-p7.2+room-profile-p6.5+threads-p5.8+notifications-p7.1+security-p8.1+cross-signing-p8.4+legacy-p3.7+devices-p8.2+verification-p8.3+backup-p8.5+user-profile-p6.6+room-ops-p6.9+crypto-store-p8.8+presence-p4.7+room-keys-p8.6+utd-recovery-p8.7+polls-p5.7+unread-p5.5+raw-content-p5.9+account-data-p6.7+media-cache-p7.3+room-directory-p6.10+attachment-p7.4+media-export-p7.5"
}
