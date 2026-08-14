//! UniFFI construction, restore, and dedicated password-login facade.
//!
//! P4-S2 exposed only `SharedCore::new` with a fail-closed vault. P4-S3a adds
//! `new_with_secret_store` so Swift can install a Keychain-backed
//! [`SecretVault`]. P4-S3b adds `restore_persisted_session`. P4-S3c adds
//! `login_with_password`: a dedicated FFI argument, never `Core.command`,
//! never registered as `matrix_login_password`. The password is not stored,
//! not copied into a DTO, never echoed, and is zeroized on drop.
//! P4-S3d adds `attach_session_owners` for the desktop owner set.
//! P4-S4 adds a typed `room_list_snapshot` wrapper that calls the
//! already-registered `matrix_room_list_snapshot` Core command only.
//! P4-S5 adds a typed `invites_snapshot` wrapper that calls the
//! already-registered `matrix_invites_snapshot` Core command only.
//! P4-S6 adds typed `timeline_open` / `timeline_close` / `timeline_paginate`
//! wrappers for those three already-registered Core commands only.
//! This still exposes no generic command FFI or APNs surface.

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use matrix_sdk::Client;
use zeroize::Zeroizing;

use crate::app::account_data::NativeImagePackOwner;
use crate::app::auth::{
    login_with_password as core_login_with_password, DevicePlatform, LoginOptions,
};
use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::app::devices::NativeDeviceOwner;
use crate::app::lifecycle::{
    persist_session_after_login, restore_session_from_vault, restore_session_onto_client,
    SessionMaterial, SessionMaterialId, SessionMaterialVault,
};
use crate::app::presence::NativePresenceOwner;
use crate::app::room_list::{
    NativeInvite, NativeInviteSnapshot, NativeInviteTriage, NativeRoomListSnapshot,
};
use crate::app::room_profile::NativeRoomJoinRuleOwner;
use crate::app::store::{
    get_or_create_store_key, AccountIdentity, StoreKeyId, StoreKeyMaterial, StoreKeyVault,
    StoreKeyVaultError, STORE_KEY_LEN,
};
use crate::app::sync::{build_sync_service, SyncServiceConfig};
use crate::app::timeline::{
    NativeTimelineDirection, NativeTimelineOpenPosition, NativeTimelineOpenReadback,
    NativeTimelineOwner, NativeTimelineViewportHint, TimelinePageState, TimelineViewPosition,
    TimelineViewSnapshot,
};
use crate::app::typing::NativeTypingOwner;
use crate::app::verification::NativeVerificationOwner;
use crate::core::Core;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{IosFailClosedPlatform, Platform, SecretVault};
use crate::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};

const VAULT_UNAVAILABLE_CODE: &str = "p4-s3b-secret-vault-unavailable";
const VAULT_UNAVAILABLE_DESCRIPTION: &str = "The secret store is unavailable.";
const IDENTITY_INVALID_CODE: &str = "p4-s3b-identity-invalid";
const IDENTITY_INVALID_DESCRIPTION: &str = "The session identity is invalid.";
const STORE_ROOT_INVALID_CODE: &str = "p4-s3b-store-root-invalid";
const STORE_ROOT_INVALID_DESCRIPTION: &str = "The session store root is invalid.";
const MATERIAL_MISSING_CODE: &str = "p4-s3b-session-material-missing";
const MATERIAL_MISSING_DESCRIPTION: &str = "No restorable session is available.";
const RESTORE_FAILED_CODE: &str = "p4-s3b-restore-failed";
const RESTORE_FAILED_DESCRIPTION: &str = "The persisted session could not be restored.";
const LOGIN_VAULT_UNAVAILABLE_CODE: &str = "p4-s3c-secret-vault-unavailable";
const LOGIN_VAULT_UNAVAILABLE_DESCRIPTION: &str = "The secret store is unavailable.";
const LOGIN_IDENTITY_INVALID_CODE: &str = "p4-s3c-identity-invalid";
const LOGIN_IDENTITY_INVALID_DESCRIPTION: &str = "The session identity is invalid.";
const LOGIN_STORE_ROOT_INVALID_CODE: &str = "p4-s3c-store-root-invalid";
const LOGIN_STORE_ROOT_INVALID_DESCRIPTION: &str = "The session store root is invalid.";
const LOGIN_FAILED_CODE: &str = "p4-s3c-login-failed";
const LOGIN_FAILED_DESCRIPTION: &str = "The session could not be authenticated.";
const ATTACH_SESSION_MISSING_CODE: &str = "p4-s3d-session-missing";
const ATTACH_SESSION_MISSING_DESCRIPTION: &str = "No retained session is available.";
const ATTACH_ALREADY_CODE: &str = "p4-s3d-already-attached";
const ATTACH_ALREADY_DESCRIPTION: &str = "Session owners are already attached.";
const ATTACH_FAILED_CODE: &str = "p4-s3d-attach-failed";
const ATTACH_FAILED_DESCRIPTION: &str = "Session owners could not be attached.";
const ATTACHED_OWNER_NAMES: &[&str] = &[
    "typing",
    "presence",
    "verification",
    "devices",
    "join_rules",
    "image_packs",
    "timelines",
    "sync",
];
const ROOM_LIST_COMMAND: &str = "matrix_room_list_snapshot";
const ROOM_LIST_READ_ONLY_GENERATION: u64 = 0;
const ROOM_LIST_NO_SESSION_CODE: &str = "p2-room-list-snapshot-no-session";
const ROOM_LIST_NO_SESSION_DESCRIPTION: &str = "No room list session is available.";
const ROOM_LIST_SYNC_NOT_STARTED_CODE: &str = "p4-s4-sync-not-started";
const ROOM_LIST_SYNC_NOT_STARTED_DESCRIPTION: &str = "The room list is not live.";
const ROOM_LIST_FAILED_CODE: &str = "p4-s4-snapshot-failed";
const ROOM_LIST_FAILED_DESCRIPTION: &str = "The room list could not be loaded.";
const INVITES_COMMAND: &str = "matrix_invites_snapshot";
const INVITES_READ_ONLY_GENERATION: u64 = 0;
const INVITES_NO_SESSION_CODE: &str = "p2-invites-snapshot-no-session";
const INVITES_NO_SESSION_DESCRIPTION: &str = "No invite session is available.";
const INVITES_FAILED_CODE: &str = "p4-s5-snapshot-failed";
const INVITES_FAILED_DESCRIPTION: &str = "The invite inbox could not be loaded.";
const TIMELINE_READ_ONLY_GENERATION: u64 = 0;
const TIMELINE_OPEN_COMMAND: &str = "matrix_timeline_open";
const TIMELINE_CLOSE_COMMAND: &str = "matrix_timeline_close";
const TIMELINE_PAGINATE_COMMAND: &str = "matrix_timeline_paginate";
const TIMELINE_OPEN_NO_SESSION_CODE: &str = "p2-timeline-open-no-session";
const TIMELINE_CLOSE_NO_SESSION_CODE: &str = "p2-timeline-close-no-session";
const TIMELINE_PAGINATE_NO_SESSION_CODE: &str = "p2-timeline-paginate-no-session";
const TIMELINE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_OPEN_FAILED_CODE: &str = "p4-s6-open-failed";
const TIMELINE_OPEN_FAILED_DESCRIPTION: &str = "The timeline could not be opened.";
const TIMELINE_CLOSE_FAILED_CODE: &str = "p4-s6-close-failed";
const TIMELINE_CLOSE_FAILED_DESCRIPTION: &str = "The timeline could not be closed.";
const TIMELINE_PAGINATE_FAILED_CODE: &str = "p4-s6-paginate-failed";
const TIMELINE_PAGINATE_FAILED_DESCRIPTION: &str = "The timeline could not be paginated.";
const TIMELINE_ROOM_NOT_FOUND_CODE: &str = "v-timeline-normal-room-not-found";
const TIMELINE_ROOM_NOT_FOUND_DESCRIPTION: &str = "The timeline room is not available.";
const TIMELINE_INVALID_ROOM_CODE: &str = "d0.3-timeline-invalid-room-id";
const TIMELINE_INVALID_ROOM_DESCRIPTION: &str = "The timeline room id is invalid.";
const TIMELINE_VIEW_NOT_OPEN_CODE: &str = "v-timeline-view-not-open";
const TIMELINE_VIEW_NOT_OPEN_DESCRIPTION: &str = "The timeline view is not open.";

/// Static fail-closed vault error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IosSecretVaultError {
    Unavailable { code: String, description: String },
}

impl std::fmt::Display for IosSecretVaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IosSecretVaultError {}

/// Swift-owned key/value callback. UniFFI UDL scaffolding consumes its
/// generated trait stub, so the crate must define this surface itself.
pub trait IosSecretVault: Send + Sync {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError>;
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError>;
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError>;
}

/// Privacy-safe restore outcome. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestoreDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed restore error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRestoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionRestoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionRestoreError {}

fn restore_failed(code: &'static str, description: &'static str) -> SessionRestoreError {
    SessionRestoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe login outcome. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLoginDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed login error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLoginError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionLoginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionLoginError {}

fn login_failed(code: &'static str, description: &'static str) -> SessionLoginError {
    SessionLoginError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe attach outcome. Owner names only; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAttachDto {
    pub owners: Vec<String>,
}

/// Static fail-closed attach error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAttachError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionAttachError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionAttachError {}

fn attach_failed(code: &'static str, description: &'static str) -> SessionAttachError {
    SessionAttachError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe room-list snapshot. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListSnapshotDto {
    pub session_generation: u64,
    pub ordered_room_ids: Vec<String>,
    pub rooms: Vec<RoomListRoomDto>,
}

/// One privacy-safe room-list row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListRoomDto {
    pub room_id: String,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    pub avatar_url: Option<String>,
    pub membership: String,
    pub is_direct: bool,
    pub is_space: bool,
    pub is_favorite: bool,
    pub unread_count: u32,
    pub highlight_count: u32,
    pub marked_unread: bool,
    pub last_activity_ts: Option<u64>,
}

/// Static fail-closed room-list error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomListSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomListSnapshotError {}

fn room_list_failed(code: &'static str, description: &'static str) -> RoomListSnapshotError {
    RoomListSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_list_core_error(error: MatrixIpcError) -> RoomListSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some("p2-room-list-snapshot-no-session") => {
            room_list_failed(ROOM_LIST_NO_SESSION_CODE, ROOM_LIST_NO_SESSION_DESCRIPTION)
        }
        Some(
            "d0.2-room-list-snapshot-timeout"
            | "d0.2-room-list-stream-ended"
            | "d0.2-room-list-reset-missing"
            | "d0.2-room-list-open-failed"
            | "d0.2-room-list-filter-failed",
        ) => room_list_failed(
            ROOM_LIST_SYNC_NOT_STARTED_CODE,
            ROOM_LIST_SYNC_NOT_STARTED_DESCRIPTION,
        ),
        _ => room_list_failed(ROOM_LIST_FAILED_CODE, ROOM_LIST_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe invite snapshot. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteSnapshotDto {
    pub session_generation: u64,
    pub invites: Vec<InviteDto>,
}

/// One privacy-safe invite row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteDto {
    pub room_id: String,
    pub room_name: String,
    pub avatar_handle_id: Option<String>,
    pub room_topic: Option<String>,
    pub room_alias: Option<String>,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_ignored: bool,
    pub invite_ts: Option<u64>,
    pub reason: Option<String>,
    pub is_space: bool,
    pub is_direct: bool,
    pub is_encrypted: bool,
    pub triage: String,
}

/// Static fail-closed invite-snapshot error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for InviteSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for InviteSnapshotError {}

fn invites_failed(code: &'static str, description: &'static str) -> InviteSnapshotError {
    InviteSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_invites_core_error(error: MatrixIpcError) -> InviteSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some(
            "p2-invites-snapshot-no-session"
            | "v-rooms.1-invites-requires-session"
            | "v-send.r-room-profile-join-rule-requires-session",
        ) => invites_failed(INVITES_NO_SESSION_CODE, INVITES_NO_SESSION_DESCRIPTION),
        _ => invites_failed(INVITES_FAILED_CODE, INVITES_FAILED_DESCRIPTION),
    }
}

fn invite_dto(invite: NativeInvite) -> InviteDto {
    InviteDto {
        room_id: invite.room_id,
        room_name: invite.room_name,
        avatar_handle_id: invite.avatar_handle_id,
        room_topic: invite.room_topic,
        room_alias: invite.room_alias,
        sender_id: invite.sender_id,
        sender_name: invite.sender_name,
        sender_ignored: invite.sender_ignored,
        invite_ts: invite.invite_ts,
        reason: invite.reason,
        is_space: invite.is_space,
        is_direct: invite.is_direct,
        is_encrypted: invite.is_encrypted,
        triage: match invite.triage {
            NativeInviteTriage::Known => "known".to_owned(),
            NativeInviteTriage::Public => "public".to_owned(),
            NativeInviteTriage::Spam => "spam".to_owned(),
        },
    }
}

/// Requested open placement. Kind is a closed string; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOpenPositionDto {
    pub kind: String,
    pub at_bottom: bool,
    pub restored_anchor_event_id: Option<String>,
    pub live_tail_event_id: Option<String>,
    pub updated_at_ms: Option<u64>,
    pub event_id: Option<String>,
}

/// Privacy-safe resolved view placement. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewPositionDto {
    pub kind: String,
    pub event_id: Option<String>,
}

/// Privacy-safe timeline snapshot. Identity/stream fields only; no token echo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSnapshotDto {
    pub schema_version: u32,
    pub session_generation: u64,
    pub room_id: String,
    pub revision: u64,
    pub position: TimelineViewPositionDto,
    pub pagination_backward: String,
    pub pagination_forward: String,
    pub own_read_event_id: Option<String>,
    pub unread_anchor_event_id: Option<String>,
    pub is_marked_unread: bool,
    pub pinned_event_ids: Vec<String>,
    pub row_count: u32,
    pub mark_read: bool,
    pub mark_unread: bool,
    pub paginate_backward: bool,
    pub paginate_forward: bool,
}

/// Privacy-safe timeline open readback. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOpenDto {
    pub schema_version: u32,
    pub stream_id: String,
    pub position: TimelineViewPositionDto,
    pub snapshot: TimelineSnapshotDto,
}

/// Static fail-closed timeline error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineError {}

fn timeline_failed(code: &'static str, description: &'static str) -> TimelineError {
    TimelineError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_open_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-open-no-session") => timeline_failed(
            TIMELINE_OPEN_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-normal-room-not-found") => timeline_failed(
            TIMELINE_ROOM_NOT_FOUND_CODE,
            TIMELINE_ROOM_NOT_FOUND_DESCRIPTION,
        ),
        Some("d0.3-timeline-room-not-found") => timeline_failed(
            "d0.3-timeline-room-not-found",
            TIMELINE_ROOM_NOT_FOUND_DESCRIPTION,
        ),
        Some("d0.3-timeline-invalid-room-id") => timeline_failed(
            TIMELINE_INVALID_ROOM_CODE,
            TIMELINE_INVALID_ROOM_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION),
    }
}

fn map_timeline_close_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-close-no-session") => timeline_failed(
            TIMELINE_CLOSE_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        _ => timeline_failed(
            TIMELINE_CLOSE_FAILED_CODE,
            TIMELINE_CLOSE_FAILED_DESCRIPTION,
        ),
    }
}

fn map_timeline_paginate_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-paginate-no-session") => timeline_failed(
            TIMELINE_PAGINATE_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(
            TIMELINE_PAGINATE_FAILED_CODE,
            TIMELINE_PAGINATE_FAILED_DESCRIPTION,
        ),
    }
}

fn open_position_from_dto(
    position: TimelineOpenPositionDto,
) -> Result<NativeTimelineOpenPosition, TimelineError> {
    match position.kind.as_str() {
        "live_bottom" => Ok(NativeTimelineOpenPosition::LiveBottom),
        "unread" => Ok(NativeTimelineOpenPosition::Unread),
        "focused" => {
            let event_id = position
                .event_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
                })?;
            Ok(NativeTimelineOpenPosition::Focused { event_id })
        }
        "normal" => Ok(NativeTimelineOpenPosition::Normal {
            viewport: NativeTimelineViewportHint {
                at_bottom: position.at_bottom,
                restored_anchor_event_id: position.restored_anchor_event_id,
                live_tail_event_id: position.live_tail_event_id,
                updated_at_ms: position.updated_at_ms,
            },
        }),
        _ => Err(timeline_failed(
            TIMELINE_OPEN_FAILED_CODE,
            TIMELINE_OPEN_FAILED_DESCRIPTION,
        )),
    }
}

fn paginate_direction(direction: &str) -> Result<NativeTimelineDirection, TimelineError> {
    match direction {
        "backwards" => Ok(NativeTimelineDirection::Backwards),
        "forwards" => Ok(NativeTimelineDirection::Forwards),
        _ => Err(timeline_failed(
            TIMELINE_PAGINATE_FAILED_CODE,
            TIMELINE_PAGINATE_FAILED_DESCRIPTION,
        )),
    }
}

fn page_state_as_str(state: TimelinePageState) -> String {
    match state {
        TimelinePageState::Available => "available",
        TimelinePageState::Exhausted => "exhausted",
        TimelinePageState::Loading => "loading",
        TimelinePageState::Unavailable => "unavailable",
    }
    .to_owned()
}

fn view_position_dto(position: TimelineViewPosition) -> TimelineViewPositionDto {
    match position {
        TimelineViewPosition::LiveBottom => TimelineViewPositionDto {
            kind: "live_bottom".to_owned(),
            event_id: None,
        },
        TimelineViewPosition::Unread { anchor_event_id } => TimelineViewPositionDto {
            kind: "unread".to_owned(),
            event_id: Some(anchor_event_id),
        },
        TimelineViewPosition::Focused { target_event_id } => TimelineViewPositionDto {
            kind: "focused".to_owned(),
            event_id: Some(target_event_id),
        },
        TimelineViewPosition::Restored { anchor_event_id } => TimelineViewPositionDto {
            kind: "restored".to_owned(),
            event_id: anchor_event_id,
        },
    }
}

fn timeline_snapshot_dto(snapshot: TimelineViewSnapshot) -> TimelineSnapshotDto {
    TimelineSnapshotDto {
        schema_version: snapshot.schema_version,
        session_generation: snapshot.session_generation,
        room_id: snapshot.room_id,
        revision: snapshot.revision,
        position: view_position_dto(snapshot.position),
        pagination_backward: page_state_as_str(snapshot.pagination.backward),
        pagination_forward: page_state_as_str(snapshot.pagination.forward),
        own_read_event_id: snapshot.read_state.own_read_event_id,
        unread_anchor_event_id: snapshot.read_state.unread_anchor_event_id,
        is_marked_unread: snapshot.read_state.is_marked_unread,
        pinned_event_ids: snapshot.pinned_event_ids,
        row_count: u32::try_from(snapshot.rows.len()).unwrap_or(u32::MAX),
        mark_read: snapshot.capabilities.mark_read,
        mark_unread: snapshot.capabilities.mark_unread,
        paginate_backward: snapshot.capabilities.paginate_backward,
        paginate_forward: snapshot.capabilities.paginate_forward,
    }
}

enum RestoredClientSlot {
    Empty,
    InFlight,
    /// Retained for later S3d attach. Read by tests; unused by S3b product code.
    #[allow(dead_code)]
    Ready(Client),
}

enum OwnerAttachSlot {
    Empty,
    InFlight,
    Ready,
}

/// Retained shared Core for the iOS UniFFI boundary.
pub struct SharedCore {
    core: Core,
    secret_store: Arc<dyn SecretVault + Send + Sync>,
    restored_client: Mutex<RestoredClientSlot>,
    owner_attach: Mutex<OwnerAttachSlot>,
}

impl SharedCore {
    /// Construct a real Core with the fail-closed iOS Platform.
    pub fn new() -> Self {
        let platform = IosFailClosedPlatform::new();
        let secret_store = Platform::secret_store(&platform);
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store,
            restored_client: Mutex::new(RestoredClientSlot::Empty),
            owner_attach: Mutex::new(OwnerAttachSlot::Empty),
        }
    }

    /// Construct a real Core whose `Platform::secret_store` is the Swift vault.
    pub fn new_with_secret_store(store: Box<dyn IosSecretVault>) -> Self {
        let vault: Arc<dyn SecretVault + Send + Sync> =
            Arc::new(CallbackSecretVault { inner: store });
        let platform = IosFailClosedPlatform::with_secret_store(Arc::clone(&vault));
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store: vault,
            restored_client: Mutex::new(RestoredClientSlot::Empty),
            owner_attach: Mutex::new(OwnerAttachSlot::Empty),
        }
    }

    /// Restore an already-persisted session from the S3a vault. No password.
    ///
    /// `store_root` is the shell-owned SDK store directory. It is never echoed.
    /// This is not `matrix_restore_session` and does not attach owners or
    /// expose `Core.command`.
    pub async fn restore_persisted_session(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
    ) -> Result<SessionRestoreDto, SessionRestoreError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| restore_failed(IDENTITY_INVALID_CODE, IDENTITY_INVALID_DESCRIPTION))?;
        let root = parse_store_root(&store_root)
            .map_err(|_| restore_failed(STORE_ROOT_INVALID_CODE, STORE_ROOT_INVALID_DESCRIPTION))?;
        let claim = RestoreClaim::acquire(&self.restored_client)?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        if vault
            .get(&SessionMaterialId::from_identity(&identity))
            .map_err(|_| restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION))?
            .is_none()
        {
            return Err(restore_failed(
                MATERIAL_MISSING_CODE,
                MATERIAL_MISSING_DESCRIPTION,
            ));
        }

        let store_key = store_key_for(&self.secret_store, &identity)?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let outcome = restore_session_from_vault(&client, &identity, &vault)
            .await
            .map_err(|error| match error {
                crate::app::lifecycle::LifecycleError::Vault {
                    diagnostic_id: "p3.6-session-material-missing",
                    ..
                } => restore_failed(MATERIAL_MISSING_CODE, MATERIAL_MISSING_DESCRIPTION),
                _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
            })?;

        let snapshot = SessionSnapshot {
            session_generation: 1,
            user_id: outcome.meta.user_id.clone(),
            device_id: outcome.meta.device_id.clone(),
            homeserver_url: outcome.meta.homeserver_url.clone(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;

        if claim.commit(client).is_err() {
            let _ = self.core.close().await;
            return Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            ));
        }

        Ok(SessionRestoreDto {
            user_id: outcome.meta.user_id,
            device_id: outcome.meta.device_id,
            homeserver_url: outcome.meta.homeserver_url,
        })
    }

    /// Password login through Core, persisted into the S3a vault for S3b restore.
    ///
    /// `password` is a dedicated FFI argument. It is never stored, never copied
    /// into the DTO, never echoed, and is zeroized when this frame returns.
    /// This is not `matrix_login_password` and does not attach owners.
    pub async fn login_with_password(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        password: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let password = Zeroizing::new(password);
        self.login_with_password_inner(&user_id, &homeserver_url, &store_root, password.as_str())
            .await
    }

    async fn login_with_password_inner(
        &self,
        user_id: &str,
        homeserver_url: &str,
        store_root: &str,
        password: &str,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let identity = AccountIdentity::new(user_id, homeserver_url).map_err(|_| {
            login_failed(
                LOGIN_IDENTITY_INVALID_CODE,
                LOGIN_IDENTITY_INVALID_DESCRIPTION,
            )
        })?;
        let root = parse_store_root(store_root).map_err(|_| {
            login_failed(
                LOGIN_STORE_ROOT_INVALID_CODE,
                LOGIN_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        if password.is_empty() {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        let claim = RestoreClaim::acquire(&self.restored_client)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let store_key =
            store_key_for(&self.secret_store, &identity).map_err(|error| match error {
                SessionRestoreError::Failed { code, .. } if code == VAULT_UNAVAILABLE_CODE => {
                    login_failed(
                        LOGIN_VAULT_UNAVAILABLE_CODE,
                        LOGIN_VAULT_UNAVAILABLE_DESCRIPTION,
                    )
                }
                _ => login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION),
            })?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let outcome = core_login_with_password(
            &client,
            identity.user_id(),
            password,
            &LoginOptions {
                request_refresh_token: true,
                device_display_name: Some(DevicePlatform::Ios.device_display_name().to_owned()),
            },
        )
        .await
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let live_identity = AccountIdentity::new(&outcome.user_id, &outcome.homeserver_url)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        if live_identity != identity {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        self.persist_open_and_retain(
            client,
            &live_identity,
            &vault,
            claim,
            outcome.user_id,
            outcome.device_id,
            outcome.homeserver_url,
        )
        .await
    }

    /// Test-only persist+open+retain through the production login path.
    ///
    /// Plants a Matrix session on an unauthenticated Client (no homeserver),
    /// then calls the same `store_key_for` + `persist_session_after_login` +
    /// `Core::open` + retain sequence `login_with_password` uses. Not on UDL.
    #[doc(hidden)]
    pub async fn persist_planted_session_for_test(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        device_id: String,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url).map_err(|_| {
            login_failed(
                LOGIN_IDENTITY_INVALID_CODE,
                LOGIN_IDENTITY_INVALID_DESCRIPTION,
            )
        })?;
        let root = parse_store_root(&store_root).map_err(|_| {
            login_failed(
                LOGIN_STORE_ROOT_INVALID_CODE,
                LOGIN_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        let claim = RestoreClaim::acquire(&self.restored_client)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let store_key =
            store_key_for(&self.secret_store, &identity).map_err(|error| match error {
                SessionRestoreError::Failed { code, .. } if code == VAULT_UNAVAILABLE_CODE => {
                    login_failed(
                        LOGIN_VAULT_UNAVAILABLE_CODE,
                        LOGIN_VAULT_UNAVAILABLE_DESCRIPTION,
                    )
                }
                _ => login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION),
            })?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let material = SessionMaterial::from_matrix_tokens(
            &identity,
            &device_id,
            &access_token,
            refresh_token.as_deref(),
        )
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        restore_session_onto_client(&client, &identity, &material)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        self.persist_open_and_retain(
            client,
            &identity,
            &vault,
            claim,
            identity.user_id().to_owned(),
            device_id,
            identity.homeserver_url().to_owned(),
        )
        .await
    }

    async fn persist_open_and_retain(
        &self,
        client: Client,
        identity: &AccountIdentity,
        vault: &SecretStoreSessionVault,
        claim: RestoreClaim<'_>,
        user_id: String,
        device_id: String,
        homeserver_url: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        persist_session_after_login(&client, identity, vault)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        let snapshot = SessionSnapshot {
            session_generation: 1,
            user_id: user_id.clone(),
            device_id: device_id.clone(),
            homeserver_url: homeserver_url.clone(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        if claim.commit(client).is_err() {
            let _ = self.core.close().await;
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }

        Ok(SessionLoginDto {
            user_id,
            device_id,
            homeserver_url,
        })
    }

    /// Attach the desktop owner set on the retained Client. No Core.command.
    ///
    /// Builds owners with no-op emit sinks (Platform::emit stays a later
    /// slice). SyncService is attached but not started so iOS does not run a
    /// second live sync while MatrixRustSDK still owns product room list.
    /// Fail-closed if no Client is retained or owners are already attached.
    pub async fn attach_session_owners(&self) -> Result<SessionAttachDto, SessionAttachError> {
        let claim = AttachClaim::acquire(&self.owner_attach)?;
        let client = {
            let guard = self
                .restored_client
                .lock()
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
            match &*guard {
                RestoredClientSlot::Ready(client) => client.clone(),
                RestoredClientSlot::Empty | RestoredClientSlot::InFlight => {
                    return Err(attach_failed(
                        ATTACH_SESSION_MISSING_CODE,
                        ATTACH_SESSION_MISSING_DESCRIPTION,
                    ));
                }
            }
        };
        let generation = self
            .core
            .session_snapshot()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?
            .ok_or_else(|| {
                attach_failed(
                    ATTACH_SESSION_MISSING_CODE,
                    ATTACH_SESSION_MISSING_DESCRIPTION,
                )
            })?
            .session_generation;
        if generation == 0 {
            return Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION));
        }

        let typing = Arc::new(
            NativeTypingOwner::start(&client, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let presence = Arc::new(
            NativePresenceOwner::start(&client, Arc::new(|_| {}), generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let verification = Arc::new(NativeVerificationOwner::new(&client, generation));
        let devices = Arc::new(
            NativeDeviceOwner::start(&client, Arc::new(|_| {}), generation)
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let join_rules = Arc::new(
            NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let image_packs = Arc::new(
            NativeImagePackOwner::start(&client, Arc::new(|_| {}), generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let timelines = Arc::new(NativeTimelineOwner::new(
            &client,
            Arc::new(|_| {}),
            generation,
        ));
        let sync = Arc::new(
            build_sync_service(&client, generation, SyncServiceConfig::default())
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );

        self.core
            .attach_typing(typing)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_presence(presence)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_verification(verification)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_devices(devices)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_join_rules(join_rules)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_image_packs(image_packs)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_timelines(timelines)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_sync(sync)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;

        claim
            .commit()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        Ok(SessionAttachDto {
            owners: ATTACHED_OWNER_NAMES
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        })
    }

    /// Typed consume of the already-registered `matrix_room_list_snapshot`.
    ///
    /// Uses `Core::command` with the same null camelCase payload desktop
    /// sends. Does not start SyncService (no dual live sync); an unstarted
    /// owner yields the handler's empty snapshot. Does not expose a generic
    /// command FFI.
    pub async fn room_list_snapshot(&self) -> Result<RoomListSnapshotDto, RoomListSnapshotError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_LIST_COMMAND.to_owned(),
                session_generation: ROOM_LIST_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_room_list_core_error)?;
        let snapshot: NativeRoomListSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| room_list_failed(ROOM_LIST_FAILED_CODE, ROOM_LIST_FAILED_DESCRIPTION))?;
        Ok(RoomListSnapshotDto {
            session_generation: snapshot.session_generation,
            ordered_room_ids: snapshot.ordered_room_ids,
            rooms: snapshot
                .rooms
                .into_iter()
                .map(|room| RoomListRoomDto {
                    room_id: room.room_id,
                    name: room.name,
                    canonical_alias: room.canonical_alias,
                    avatar_url: room.avatar_url,
                    membership: room.membership.as_str().to_owned(),
                    is_direct: room.is_direct,
                    is_space: room.is_space,
                    is_favorite: room.is_favorite,
                    unread_count: room.unread_count,
                    highlight_count: room.highlight_count,
                    marked_unread: room.marked_unread,
                    last_activity_ts: room.last_activity_ts,
                })
                .collect(),
        })
    }

    pub async fn invites_snapshot(&self) -> Result<InviteSnapshotDto, InviteSnapshotError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: INVITES_COMMAND.to_owned(),
                session_generation: INVITES_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_invites_core_error)?;
        let snapshot: NativeInviteSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| invites_failed(INVITES_FAILED_CODE, INVITES_FAILED_DESCRIPTION))?;
        Ok(InviteSnapshotDto {
            session_generation: snapshot.session_generation,
            invites: snapshot.invites.into_iter().map(invite_dto).collect(),
        })
    }

    pub async fn timeline_open(
        &self,
        room_id: String,
        position: TimelineOpenPositionDto,
    ) -> Result<TimelineOpenDto, TimelineError> {
        let position = open_position_from_dto(position)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_OPEN_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "position": position,
                }),
            })
            .await
            .map_err(map_timeline_open_core_error)?;
        let readback: NativeTimelineOpenReadback = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
            })?;
        Ok(TimelineOpenDto {
            schema_version: readback.schema_version,
            stream_id: readback.stream_id,
            position: view_position_dto(readback.position),
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }

    pub async fn timeline_close(&self, stream_id: String) -> Result<bool, TimelineError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_CLOSE_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "streamId": stream_id }),
            })
            .await
            .map_err(map_timeline_close_core_error)?;
        serde_json::from_value(response.payload).map_err(|_| {
            timeline_failed(
                TIMELINE_CLOSE_FAILED_CODE,
                TIMELINE_CLOSE_FAILED_DESCRIPTION,
            )
        })
    }

    pub async fn timeline_paginate(
        &self,
        stream_id: String,
        direction: String,
    ) -> Result<TimelineSnapshotDto, TimelineError> {
        let direction = paginate_direction(&direction)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_PAGINATE_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "streamId": stream_id,
                    "direction": direction,
                }),
            })
            .await
            .map_err(map_timeline_paginate_core_error)?;
        let snapshot: TimelineViewSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_failed(
                    TIMELINE_PAGINATE_FAILED_CODE,
                    TIMELINE_PAGINATE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(timeline_snapshot_dto(snapshot))
    }
}

/// Claims the restore slot for one in-flight attempt. Drop releases it unless
/// [`RestoreClaim::commit`] stores the Client after a successful Core open.
struct RestoreClaim<'a> {
    slot: &'a Mutex<RestoredClientSlot>,
    committed: bool,
}

impl<'a> RestoreClaim<'a> {
    fn acquire(slot: &'a Mutex<RestoredClientSlot>) -> Result<Self, SessionRestoreError> {
        let mut guard = slot
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        match *guard {
            RestoredClientSlot::Empty => {
                *guard = RestoredClientSlot::InFlight;
                Ok(Self {
                    slot,
                    committed: false,
                })
            }
            RestoredClientSlot::InFlight | RestoredClientSlot::Ready(_) => Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            )),
        }
    }

    fn commit(mut self, client: Client) -> Result<(), SessionRestoreError> {
        let mut guard = self
            .slot
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        if !matches!(*guard, RestoredClientSlot::InFlight) {
            return Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            ));
        }
        *guard = RestoredClientSlot::Ready(client);
        self.committed = true;
        Ok(())
    }
}

impl Drop for RestoreClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut guard) = self.slot.lock() {
            if matches!(*guard, RestoredClientSlot::InFlight) {
                *guard = RestoredClientSlot::Empty;
            }
        }
    }
}

/// Claims the owner-attach slot for one in-flight attempt.
struct AttachClaim<'a> {
    slot: &'a Mutex<OwnerAttachSlot>,
    committed: bool,
}

impl<'a> AttachClaim<'a> {
    fn acquire(slot: &'a Mutex<OwnerAttachSlot>) -> Result<Self, SessionAttachError> {
        let mut guard = slot
            .lock()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        match *guard {
            OwnerAttachSlot::Empty => {
                *guard = OwnerAttachSlot::InFlight;
                Ok(Self {
                    slot,
                    committed: false,
                })
            }
            OwnerAttachSlot::Ready => Err(attach_failed(
                ATTACH_ALREADY_CODE,
                ATTACH_ALREADY_DESCRIPTION,
            )),
            OwnerAttachSlot::InFlight => {
                Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))
            }
        }
    }

    fn commit(mut self) -> Result<(), SessionAttachError> {
        let mut guard = self
            .slot
            .lock()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        if !matches!(*guard, OwnerAttachSlot::InFlight) {
            return Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION));
        }
        *guard = OwnerAttachSlot::Ready;
        self.committed = true;
        Ok(())
    }
}

impl Drop for AttachClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut guard) = self.slot.lock() {
            if matches!(*guard, OwnerAttachSlot::InFlight) {
                *guard = OwnerAttachSlot::Empty;
            }
        }
    }
}

fn parse_store_root(store_root: &str) -> Result<&Path, ()> {
    let trimmed = store_root.trim();
    if trimmed.is_empty() {
        return Err(());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(());
    }
    Ok(path)
}

fn store_key_for(
    store: &Arc<dyn SecretVault + Send + Sync>,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let vault = SecretStoreKeyVault {
        store: Arc::clone(store),
    };
    get_or_create_store_key(&vault, &StoreKeyId::from_identity(identity)).map_err(|error| {
        match error {
            StoreKeyVaultError::BackendUnavailable { .. } => {
                restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION)
            }
            StoreKeyVaultError::CorruptPayload => {
                restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION)
            }
            _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
        }
    })
}

struct CallbackSecretVault {
    inner: Box<dyn IosSecretVault>,
}

impl SecretVault for CallbackSecretVault {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError> {
        self.inner
            .get(key.to_owned())
            .map_err(|_| vault_unavailable())
    }

    fn put(&self, key: &str, value: &[u8]) -> Result<(), MatrixIpcError> {
        self.inner
            .put(key.to_owned(), value.to_vec())
            .map_err(|_| vault_unavailable())
    }

    fn delete(&self, key: &str) -> Result<(), MatrixIpcError> {
        self.inner
            .delete(key.to_owned())
            .map_err(|_| vault_unavailable())
    }
}

struct SecretStoreSessionVault {
    store: Arc<dyn SecretVault + Send + Sync>,
}

impl SessionMaterialVault for SecretStoreSessionVault {
    fn get(
        &self,
        id: &SessionMaterialId,
    ) -> Result<Option<SessionMaterial>, crate::app::lifecycle::LifecycleError> {
        match self.store.get(id.account()) {
            Ok(Some(bytes)) => Ok(Some(SessionMaterial::from_sealed_blob(bytes))),
            Ok(None) => Ok(None),
            Err(_) => Err(crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }),
        }
    }

    fn set(
        &self,
        id: &SessionMaterialId,
        material: &SessionMaterial,
    ) -> Result<(), crate::app::lifecycle::LifecycleError> {
        self.store
            .put(id.account(), material.as_bytes())
            .map_err(|_| crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            })
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, crate::app::lifecycle::LifecycleError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store.delete(id.account()).map_err(|_| {
            crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }
        })?;
        Ok(existed)
    }
}

struct SecretStoreKeyVault {
    store: Arc<dyn SecretVault + Send + Sync>,
}

impl StoreKeyVault for SecretStoreKeyVault {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        match self.store.get(id.account()) {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) if bytes.len() == STORE_KEY_LEN => {
                let mut key_bytes = [0u8; STORE_KEY_LEN];
                key_bytes.copy_from_slice(&bytes);
                Ok(Some(StoreKeyMaterial::from_bytes(key_bytes)))
            }
            Ok(Some(_)) => Err(StoreKeyVaultError::CorruptPayload),
            Err(_) => Err(StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            }),
        }
    }

    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        self.store
            .put(id.account(), key.as_bytes().as_slice())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })
    }

    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store
            .delete(id.account())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })?;
        Ok(existed)
    }
}

fn vault_unavailable() -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::StoreUnavailable)
        .with_diagnostic("p4-s3-secret-vault-unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::lifecycle::persist_session_material;
    use crate::app::store::StoreKeyId;
    use crate::transport::MatrixIpcErrorCategory;
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MemoryCallbackVault(std::sync::Arc<Mutex<HashMap<String, Vec<u8>>>>);

    impl IosSecretVault for MemoryCallbackVault {
        fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
            Ok(self.0.lock().expect("vault").get(&key).cloned())
        }

        fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
            self.0.lock().expect("vault").insert(key, value);
            Ok(())
        }

        fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
            self.0.lock().expect("vault").remove(&key);
            Ok(())
        }
    }

    fn alice() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-p4-s3b-{tag}-{nanos}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn test_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn shared_core_constructs_and_retains_the_built_in_core() {
        let shared_core = SharedCore::new();
        assert!(
            !shared_core.core.registered_commands().is_empty(),
            "P4-S2 must retain a real Core with its built-in registry"
        );
    }

    #[test]
    fn shared_core_with_secret_store_round_trips_through_the_callback() {
        let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
            HashMap::new(),
        ))));
        let shared = SharedCore::new_with_secret_store(store);
        assert!(
            !shared.core.registered_commands().is_empty(),
            "P4-S3a must still retain a real Core"
        );
    }

    #[test]
    fn callback_vault_maps_foreign_failure_to_static_store_unavailable() {
        struct FailingVault;
        impl IosSecretVault for FailingVault {
            fn get(&self, _: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
                Err(IosSecretVaultError::Unavailable {
                    code: "p4-s3-secret-vault-unavailable".to_owned(),
                    description: "The secret store is unavailable.".to_owned(),
                })
            }
            fn put(&self, _: String, _: Vec<u8>) -> Result<(), IosSecretVaultError> {
                unreachable!("put")
            }
            fn delete(&self, _: String) -> Result<(), IosSecretVaultError> {
                unreachable!("delete")
            }
        }

        let vault = CallbackSecretVault {
            inner: Box::new(FailingVault),
        };
        let error = vault.get("session").expect_err("must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::StoreUnavailable);
        assert!(!format!("{error:?}").contains("session"));
    }

    #[test]
    fn restore_without_vault_fails_closed_without_echoing_identity() {
        let shared = SharedCore::new();
        let root = temp_root("no-vault");
        let rt = test_runtime();
        let error = rt
            .block_on(shared.restore_persisted_session(
                "@alice:example.org".to_owned(),
                "https://matrix.example.org".to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("fail-closed vault cannot restore");
        let text = format!("{error:?}");
        assert!(text.contains(VAULT_UNAVAILABLE_CODE));
        assert!(!text.contains(MATERIAL_MISSING_CODE));
        assert!(!text.contains("@alice"));
        assert!(!text.contains("matrix.example.org"));
        assert!(!text.contains(root.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_rejects_hostile_identity_without_echo() {
        let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
            HashMap::new(),
        ))));
        let shared = SharedCore::new_with_secret_store(store);
        let root = temp_root("hostile");
        let rt = test_runtime();
        let hostile = "https://user:secret@evil.example/?password=hunter2";
        let error = rt
            .block_on(shared.restore_persisted_session(
                "not-a-user".to_owned(),
                hostile.to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("invalid identity");
        let text = format!("{error:?}{error}");
        assert!(text.contains(IDENTITY_INVALID_CODE));
        assert!(!text.contains("secret"));
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("evil.example"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_from_vault_installs_session_without_password_or_token_leak() {
        let access = "syt_s3b_access_token_value";
        let refresh = "syr_s3b_refresh_token_value";
        let identity = alice();
        let material =
            SessionMaterial::from_matrix_tokens(&identity, "DEVICEABC", access, Some(refresh))
                .unwrap();
        let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let persist_vault = SecretStoreSessionVault {
            store: Arc::new(CallbackSecretVault {
                inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
            }),
        };
        persist_session_material(&persist_vault, &identity, &material).unwrap();
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
            std::sync::Arc::clone(&map),
        )));
        let root = temp_root("restore");
        let rt = test_runtime();
        let _enter = rt.enter();
        let dto = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect("restore");
        assert_eq!(dto.user_id, "@alice:example.org");
        assert_eq!(dto.device_id, "DEVICEABC");
        assert_eq!(dto.homeserver_url, "https://matrix.example.org");
        let dbg = format!("{dto:?}");
        assert!(!dbg.contains(access));
        assert!(!dbg.contains(refresh));
        assert!(!dbg.contains("password"));
        let snapshot = shared.core.session_snapshot().expect("projection");
        assert!(snapshot.is_some());
        assert!(matches!(
            *shared.restored_client.lock().expect("client"),
            RestoredClientSlot::Ready(_)
        ));
        let keys: Vec<String> = map.lock().expect("vault").keys().cloned().collect();
        assert!(keys.iter().any(|key| key.starts_with("store-key:")));
        assert!(keys.iter().any(|key| key.starts_with("matrix-session:")));
        assert!(!keys.iter().any(|key| key.contains("p4-s3b-store-key")));
        let second = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("second restore");
        assert!(format!("{second:?}").contains(RESTORE_FAILED_CODE));
        assert!(matches!(
            *shared.restored_client.lock().expect("client"),
            RestoredClientSlot::Ready(_)
        ));
        drop(shared);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_rejects_wrong_length_store_key_without_replacing_it() {
        let identity = alice();
        let material = SessionMaterial::from_matrix_tokens(
            &identity,
            "DEVICEABC",
            "syt_s3b_corrupt_key_access",
            None,
        )
        .unwrap();
        let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let persist_vault = SecretStoreSessionVault {
            store: Arc::new(CallbackSecretVault {
                inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
            }),
        };
        persist_session_material(&persist_vault, &identity, &material).unwrap();
        let store_key_account = StoreKeyId::from_identity(&identity).account().to_owned();
        map.lock()
            .expect("vault")
            .insert(store_key_account.clone(), vec![0u8; 8]);
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
            std::sync::Arc::clone(&map),
        )));
        let root = temp_root("corrupt-key");
        let rt = test_runtime();
        let error = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("corrupt store key");
        assert!(format!("{error:?}").contains(RESTORE_FAILED_CODE));
        let stored = map
            .lock()
            .expect("vault")
            .get(&store_key_account)
            .cloned()
            .expect("key remains");
        assert_eq!(stored.len(), 8);
        assert!(!map
            .lock()
            .expect("vault")
            .keys()
            .any(|key| key.contains("p4-s3b-store-key")));
        let _ = fs::remove_dir_all(&root);
    }
}
