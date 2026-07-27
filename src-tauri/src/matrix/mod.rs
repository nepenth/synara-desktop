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
//! P3.2: Password/token login + device naming (harness).
//! P3.5: Session secret / refresh-token persistence foundation (host vault only).
//! P4.1: Sync service readiness / reconnect model (harness foundation).
//! P4.2: Room-list snapshot and delta projection (harness foundation).
//! P4.5: Space hierarchy / filters / parents (harness foundation).
//! P4.6: Room member / power-level index foundation (harness).
//! P4.8: Route / deep-link resolution foundation (harness).
//! P5.1: Timeline registry and lifecycle (harness foundation).
//! P5.2: Timeline snapshot / ordered-diff projection (harness foundation).
//! P5.3: Timeline pagination state machine foundation.
//! P5.6: Relations / reactions / replaces index foundation (harness).
//! P5.8: Thread list / summary index foundation (harness).
//! P6.1: Outbound send queue + local-echo foundation (harness; no Room::send).
//! P6.2: Receipt index foundation (harness; no SDK send_receipt).
//! P6.3: Typing index foundation (harness; no SDK typing send).
//! P6.4: Media upload queue foundation (metadata only; no bytes / no SDK upload).
//! P7.1: Notification candidate index foundation (harness; privacy-filtered).
//! P8.1: Security / crypto status projection foundation (harness; no secrets).
//! P9.1: Widget / Element Call session registry foundation (harness).
//! No production login/sync loop or Tauri command registration lives here yet.
//! No dual-backend selector. Product runtime remains matrix-js-sdk.

pub mod auth;
pub mod client_builder;
pub mod diagnostics;
pub mod dto;
pub mod ipc;
pub mod lifecycle;
pub mod media;
pub mod members;
pub mod notifications;
pub mod receipts;
pub mod relations;
pub mod room_list;
pub mod routes;
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
pub mod widgets;

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
    let _auth = auth::matrix_auth_markers();
    let _sync = sync::matrix_sync_markers();
    let _room_list = room_list::matrix_room_list_markers();
    let _routes = routes::matrix_routes_markers();
    let _spaces = spaces::matrix_spaces_markers();
    let _members = members::matrix_members_markers();
    let _timeline = timeline::matrix_timeline_markers();
    let _relations = relations::matrix_relations_markers();
    let _security = security::matrix_security_markers();
    let _send = send::matrix_send_markers();
    let _receipts = receipts::matrix_receipts_markers();
    let _threads = threads::matrix_threads_markers();
    let _typing = typing::matrix_typing_markers();
    let _media = media::matrix_media_markers();
    let _notifications = notifications::matrix_notifications_markers();
    let _widgets = widgets::matrix_widgets_markers();
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
    debug_assert_eq!(_auth, auth::MATRIX_AUTH_MARKER);
    debug_assert_eq!(_sync, sync::MATRIX_SYNC_MARKER);
    debug_assert_eq!(_room_list, room_list::MATRIX_ROOM_LIST_MARKER);
    debug_assert_eq!(_routes, routes::MATRIX_ROUTES_MARKER);
    debug_assert_eq!(_spaces, spaces::MATRIX_SPACES_MARKER);
    debug_assert_eq!(_members, members::MATRIX_MEMBERS_MARKER);
    debug_assert_eq!(_timeline, timeline::MATRIX_TIMELINE_MARKER);
    debug_assert_eq!(_relations, relations::MATRIX_RELATIONS_MARKER);
    debug_assert_eq!(_security, security::MATRIX_SECURITY_MARKER);
    debug_assert_eq!(_send, send::MATRIX_SEND_MARKER);
    debug_assert_eq!(_receipts, receipts::MATRIX_RECEIPTS_MARKER);
    debug_assert_eq!(_threads, threads::MATRIX_THREADS_MARKER);
    debug_assert_eq!(_typing, typing::MATRIX_TYPING_MARKER);
    debug_assert_eq!(_media, media::MATRIX_MEDIA_MARKER);
    debug_assert_eq!(_notifications, notifications::MATRIX_NOTIFICATIONS_MARKER);
    debug_assert_eq!(_widgets, widgets::MATRIX_WIDGETS_MARKER);
    "matrix-ipc-protocol-v1+domain-dtos-p1.4+supervisor-p2.1+store-p2.2+client-builder-p2.3+tasks-p2.4+diagnostics-p2.5+lifecycle-p2.6+auth-p3.2+session-persist-p3.5+sync-p4.1+room-list-p4.2+routes-p4.8+spaces-p4.5+members-p4.6+timeline-p5.1+diffs-p5.2+pagination-p5.3+relations-p5.6+send-p6.1+receipts-p6.2+typing-p6.3+media-p6.4+threads-p5.8+notifications-p7.1+security-p8.1+widgets-p9.1"
}
