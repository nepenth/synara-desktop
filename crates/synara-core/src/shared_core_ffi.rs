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
//! P4-S7 adds typed typing/presence wrappers for the five already-registered
//! Core commands in that family only.
//! P4-S8 adds a typed `verification_list` wrapper for the already-registered
//! `matrix_verification_list` Core command only.
//! P4-S9 adds typed verification SAS wrappers for the seven already-registered
//! start/accept/begin_sas/confirm/mismatch/cancel/dismiss Core commands only.
//! P4-S9-2 adds typed device wrappers for the four already-registered
//! snapshot/rename/delete-start/delete-cancel Core commands only.
//! Backup status, room-key transfer status, and cross-signing setup stay off
//! this slice: they sit next to leftover passphrase/path/password envelopes.
//! P4-S9-3 adds a typed `room_join_rule_snapshot` wrapper for the
//! already-registered `matrix_room_join_rule_snapshot` Core command only.
//! There is no join-rule writer on Core.
//! P4-S9-4 adds typed image-pack get/set wrappers for the six
//! already-registered Core commands. Pack metadata/IDs/URLs/JSON may
//! cross. Image/media bytes stay off. Later/m.direct stay off.
//! This still exposes no generic command FFI or APNs surface.

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use matrix_sdk::Client;
use zeroize::Zeroizing;

use crate::app::account_data::{
    NativeGlobalImagePacksSnapshot, NativeImagePack, NativeImagePackOwner,
    NativeRoomImagePacksSnapshot, NativeUserImagePackSnapshot,
};
use crate::app::auth::{
    login_with_password as core_login_with_password, DevicePlatform, LoginOptions,
};
use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::app::devices::{
    NativeDeviceDeleteAuthentication, NativeDeviceDeleteResult, NativeDeviceOwner,
    NativeDeviceSnapshot, NativeDeviceTrust,
};
use crate::app::lifecycle::{
    persist_session_after_login, restore_session_from_vault, restore_session_onto_client,
    SessionMaterial, SessionMaterialId, SessionMaterialVault,
};
use crate::app::presence::{
    NativePresenceOwner, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription,
};
use crate::app::room_list::{
    NativeInvite, NativeInviteSnapshot, NativeInviteTriage, NativeRoomListSnapshot,
};
use crate::app::room_profile::{MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleOwner};
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
use crate::app::typing::{NativeTypingOwner, NativeTypingSnapshot};
use crate::app::verification::{
    NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationOwner, NativeVerificationPhase, NativeVerificationRequest,
    NativeVerificationSas,
};
use crate::core::Core;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{IosFailClosedPlatform, Platform, SecretVault};
use crate::transport::{
    CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory, MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
};

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
const TYPING_PRESENCE_GENERATION: u64 = 0;
const TYPING_SNAPSHOT_COMMAND: &str = "matrix_typing_snapshot";
const TYPING_SET_COMMAND: &str = "matrix_typing_set";
const PRESENCE_SNAPSHOT_COMMAND: &str = "matrix_presence_snapshot";
const PRESENCE_SUBSCRIBE_COMMAND: &str = "matrix_presence_subscribe";
const PRESENCE_UNSUBSCRIBE_COMMAND: &str = "matrix_presence_unsubscribe";
const TYPING_SNAPSHOT_NO_SESSION_CODE: &str = "p2-typing-snapshot-no-session";
const TYPING_SET_NO_SESSION_CODE: &str = "p2-typing-set-no-session";
const PRESENCE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-presence-snapshot-no-session";
const PRESENCE_SUBSCRIBE_NO_SESSION_CODE: &str = "p2-presence-subscribe-no-session";
const PRESENCE_UNSUBSCRIBE_NO_SESSION_CODE: &str = "p2-presence-unsubscribe-no-session";
const TYPING_NO_SESSION_DESCRIPTION: &str = "No typing session is available.";
const PRESENCE_NO_SESSION_DESCRIPTION: &str = "No presence session is available.";
const TYPING_SNAPSHOT_FAILED_CODE: &str = "p4-s7-typing-snapshot-failed";
const TYPING_SNAPSHOT_FAILED_DESCRIPTION: &str = "The typing snapshot could not be loaded.";
const TYPING_SET_FAILED_CODE: &str = "p4-s7-typing-set-failed";
const TYPING_SET_FAILED_DESCRIPTION: &str = "The typing notice could not be updated.";
const TYPING_ROOM_MISSING_CODE: &str = "v-rooms.4-typing-room-missing";
const TYPING_ROOM_MISSING_DESCRIPTION: &str = "The typing room is not available.";
const TYPING_INVALID_ROOM_CODE: &str = "v-rooms.4-typing-invalid-room";
const TYPING_INVALID_ROOM_DESCRIPTION: &str = "The typing room id is invalid.";
const PRESENCE_SNAPSHOT_FAILED_CODE: &str = "p4-s7-presence-snapshot-failed";
const PRESENCE_SNAPSHOT_FAILED_DESCRIPTION: &str = "The presence snapshot could not be loaded.";
const PRESENCE_SUBSCRIBE_FAILED_CODE: &str = "p4-s7-presence-subscribe-failed";
const PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION: &str =
    "The presence subscription could not be created.";
const PRESENCE_UNSUBSCRIBE_FAILED_CODE: &str = "p4-s7-presence-unsubscribe-failed";
const PRESENCE_UNSUBSCRIBE_FAILED_DESCRIPTION: &str =
    "The presence subscription could not be released.";
const PRESENCE_INVALID_USER_CODE: &str = "v-presence-invalid-user-id";
const PRESENCE_INVALID_USER_DESCRIPTION: &str = "The presence user id is invalid.";
const PRESENCE_INVALID_SUBSCRIPTION_CODE: &str = "v-presence-invalid-subscription-id";
const PRESENCE_INVALID_SUBSCRIPTION_DESCRIPTION: &str = "The presence subscription id is invalid.";
const VERIFICATION_LIST_COMMAND: &str = "matrix_verification_list";
const VERIFICATION_LIST_GENERATION: u64 = 0;
const VERIFICATION_LIST_NO_SESSION_CODE: &str = "p2-verification-list-no-session";
const VERIFICATION_LIST_NO_SESSION_DESCRIPTION: &str = "No verification session is available.";
const VERIFICATION_LIST_FAILED_CODE: &str = "p4-s8-list-failed";
const VERIFICATION_LIST_FAILED_DESCRIPTION: &str = "The verification inbox could not be loaded.";
const VERIFICATION_SAS_GENERATION: u64 = 0;
const VERIFICATION_START_COMMAND: &str = "matrix_verification_start";
const VERIFICATION_ACCEPT_COMMAND: &str = "matrix_verification_accept";
const VERIFICATION_BEGIN_SAS_COMMAND: &str = "matrix_verification_begin_sas";
const VERIFICATION_CONFIRM_COMMAND: &str = "matrix_verification_confirm";
const VERIFICATION_MISMATCH_COMMAND: &str = "matrix_verification_mismatch";
const VERIFICATION_CANCEL_COMMAND: &str = "matrix_verification_cancel";
const VERIFICATION_DISMISS_COMMAND: &str = "matrix_verification_dismiss";
const VERIFICATION_START_NO_SESSION_CODE: &str = "p2-verification-start-no-session";
const VERIFICATION_ACCEPT_NO_SESSION_CODE: &str = "p2-verification-accept-no-session";
const VERIFICATION_BEGIN_SAS_NO_SESSION_CODE: &str = "p2-verification-begin-sas-no-session";
const VERIFICATION_CONFIRM_NO_SESSION_CODE: &str = "p2-verification-confirm-no-session";
const VERIFICATION_MISMATCH_NO_SESSION_CODE: &str = "p2-verification-mismatch-no-session";
const VERIFICATION_CANCEL_NO_SESSION_CODE: &str = "p2-verification-cancel-no-session";
const VERIFICATION_DISMISS_NO_SESSION_CODE: &str = "p2-verification-dismiss-no-session";
const VERIFICATION_SAS_NO_SESSION_DESCRIPTION: &str = "No verification session is available.";
const VERIFICATION_SAS_FAILED_CODE: &str = "p4-s9-sas-failed";
const VERIFICATION_SAS_FAILED_DESCRIPTION: &str =
    "The verification request could not be completed.";
const VERIFICATION_SAS_OWNER_DESCRIPTION: &str = "The verification request is not available.";
const DEVICE_COMMAND_GENERATION: u64 = 0;
const DEVICE_SNAPSHOT_COMMAND: &str = "matrix_device_snapshot";
const DEVICE_RENAME_COMMAND: &str = "matrix_device_rename";
const DEVICE_DELETE_START_COMMAND: &str = "matrix_device_delete_start";
const DEVICE_DELETE_CANCEL_COMMAND: &str = "matrix_device_delete_cancel";
const DEVICE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-device-snapshot-no-session";
const DEVICE_RENAME_NO_SESSION_CODE: &str = "p2-device-rename-no-session";
const DEVICE_DELETE_START_NO_SESSION_CODE: &str = "p2-device-delete-start-no-session";
const DEVICE_DELETE_CANCEL_NO_SESSION_CODE: &str = "p2-device-delete-cancel-no-session";
const DEVICE_NO_SESSION_DESCRIPTION: &str = "No device session is available.";
const DEVICE_FAILED_CODE: &str = "p4-s9-2-device-failed";
const DEVICE_FAILED_DESCRIPTION: &str = "The device request could not be completed.";
const DEVICE_OWNER_DESCRIPTION: &str = "The device request is not available.";
const JOIN_RULE_SNAPSHOT_COMMAND: &str = "matrix_room_join_rule_snapshot";
const JOIN_RULE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-join-rule-snapshot-no-session";
const JOIN_RULE_NO_SESSION_DESCRIPTION: &str = "No join-rule session is available.";
const JOIN_RULE_FAILED_CODE: &str = "p4-s9-3-join-rule-failed";
const JOIN_RULE_FAILED_DESCRIPTION: &str = "The join-rule request could not be completed.";
const JOIN_RULE_OWNER_DESCRIPTION: &str = "The join-rule request is not available.";
const IMAGE_PACK_COMMAND_GENERATION: u64 = 0;
const GET_GLOBAL_IMAGE_PACKS_COMMAND: &str = "matrix_get_global_image_packs";
const GET_USER_IMAGE_PACK_COMMAND: &str = "matrix_get_user_image_pack";
const GET_ROOM_IMAGE_PACKS_COMMAND: &str = "matrix_get_room_image_packs";
const SET_USER_IMAGE_PACK_COMMAND: &str = "matrix_set_user_image_pack";
const SET_GLOBAL_IMAGE_PACKS_COMMAND: &str = "matrix_set_global_image_packs";
const SET_ROOM_IMAGE_PACK_COMMAND: &str = "matrix_set_room_image_pack";
const GET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE: &str = "p2-global-image-packs-no-session";
const GET_USER_IMAGE_PACK_NO_SESSION_CODE: &str = "p2-user-image-pack-no-session";
const GET_ROOM_IMAGE_PACKS_NO_SESSION_CODE: &str = "p2-room-image-packs-no-session";
const SET_USER_IMAGE_PACK_NO_SESSION_CODE: &str = "p2-set-user-image-pack-no-session";
const SET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE: &str = "p2-set-global-image-packs-no-session";
const SET_ROOM_IMAGE_PACK_NO_SESSION_CODE: &str = "p2-set-room-image-pack-no-session";
const IMAGE_PACK_NO_SESSION_DESCRIPTION: &str = "No image-pack session is available.";
const IMAGE_PACK_FAILED_CODE: &str = "p4-s9-4-image-pack-failed";
const IMAGE_PACK_FAILED_DESCRIPTION: &str = "The image-pack request could not be completed.";
const IMAGE_PACK_INVALID_JSON_CODE: &str = "p4-s9-4-image-pack-invalid-json";
const IMAGE_PACK_INVALID_JSON_DESCRIPTION: &str = "The image-pack content is invalid.";
const IMAGE_PACK_OWNER_DESCRIPTION: &str = "The image-pack request is not available.";

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

/// Privacy-safe typing room row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingRoomDto {
    pub room_id: String,
    pub user_ids: Vec<String>,
}

/// Privacy-safe typing snapshot. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingSnapshotDto {
    pub session_generation: u64,
    pub rooms: Vec<TypingRoomDto>,
}

/// Static fail-closed typing error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypingCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TypingCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TypingCommandError {}

fn typing_failed(code: &'static str, description: &'static str) -> TypingCommandError {
    TypingCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_typing_snapshot_core_error(error: MatrixIpcError) -> TypingCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-typing-snapshot-no-session") => typing_failed(
            TYPING_SNAPSHOT_NO_SESSION_CODE,
            TYPING_NO_SESSION_DESCRIPTION,
        ),
        _ => typing_failed(
            TYPING_SNAPSHOT_FAILED_CODE,
            TYPING_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

fn map_typing_set_core_error(error: MatrixIpcError) -> TypingCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-typing-set-no-session") => {
            typing_failed(TYPING_SET_NO_SESSION_CODE, TYPING_NO_SESSION_DESCRIPTION)
        }
        Some("v-rooms.4-typing-invalid-room") => {
            typing_failed(TYPING_INVALID_ROOM_CODE, TYPING_INVALID_ROOM_DESCRIPTION)
        }
        Some("v-rooms.4-typing-room-missing") => {
            typing_failed(TYPING_ROOM_MISSING_CODE, TYPING_ROOM_MISSING_DESCRIPTION)
        }
        Some("v-rooms.4-typing-room-not-joined") => typing_failed(
            "v-rooms.4-typing-room-not-joined",
            TYPING_ROOM_MISSING_DESCRIPTION,
        ),
        _ => typing_failed(TYPING_SET_FAILED_CODE, TYPING_SET_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe presence snapshot. Identity fields only; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub user_id: String,
    pub state: Option<String>,
    pub currently_active: bool,
    pub last_active_ts: Option<u64>,
    pub status_msg: Option<String>,
}

/// Privacy-safe presence subscription. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSubscriptionDto {
    pub subscription_id: String,
    pub user_id: String,
    pub session_generation: u64,
}

/// Static fail-closed presence error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PresenceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PresenceCommandError {}

fn presence_failed(code: &'static str, description: &'static str) -> PresenceCommandError {
    PresenceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_presence_snapshot_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-snapshot-no-session") => presence_failed(
            PRESENCE_SNAPSHOT_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-user-id") => presence_failed(
            PRESENCE_INVALID_USER_CODE,
            PRESENCE_INVALID_USER_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_SNAPSHOT_FAILED_CODE,
            PRESENCE_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

fn map_presence_subscribe_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-subscribe-no-session") => presence_failed(
            PRESENCE_SUBSCRIBE_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-user-id") => presence_failed(
            PRESENCE_INVALID_USER_CODE,
            PRESENCE_INVALID_USER_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_SUBSCRIBE_FAILED_CODE,
            PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION,
        ),
    }
}

fn map_presence_unsubscribe_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-unsubscribe-no-session") => presence_failed(
            PRESENCE_UNSUBSCRIBE_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-subscription-id") => presence_failed(
            PRESENCE_INVALID_SUBSCRIPTION_CODE,
            PRESENCE_INVALID_SUBSCRIPTION_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_UNSUBSCRIBE_FAILED_CODE,
            PRESENCE_UNSUBSCRIBE_FAILED_DESCRIPTION,
        ),
    }
}

fn presence_state_as_str(state: NativePresenceState) -> String {
    match state {
        NativePresenceState::Unknown => "unknown",
        NativePresenceState::Offline => "offline",
        NativePresenceState::Online => "online",
        NativePresenceState::Unavailable => "unavailable",
    }
    .to_owned()
}

fn presence_snapshot_dto(result: NativePresenceSnapshotResult) -> PresenceSnapshotDto {
    match result {
        NativePresenceSnapshotResult::Ready {
            session_generation,
            user_id,
            snapshot,
        } => PresenceSnapshotDto {
            status: "ready".to_owned(),
            session_generation,
            user_id,
            state: Some(presence_state_as_str(snapshot.state)),
            currently_active: snapshot.currently_active,
            last_active_ts: snapshot.last_active_ts,
            status_msg: snapshot.status_msg,
        },
        NativePresenceSnapshotResult::Unknown {
            session_generation,
            user_id,
        } => PresenceSnapshotDto {
            status: "unknown".to_owned(),
            session_generation,
            user_id,
            state: None,
            currently_active: false,
            last_active_ts: None,
            status_msg: None,
        },
    }
}

/// Privacy-safe SAS emoji. User-visible comparison only; no key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationEmojiDto {
    pub symbol: String,
    pub description: String,
}

/// Privacy-safe SAS comparison. Emoji/decimals only; no tokens or MACs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationSasDto {
    pub emoji: Option<Vec<VerificationEmojiDto>>,
    pub decimals: Option<Vec<u16>>,
}

/// Privacy-safe verification request row. Identity/flow fields only; no tokens.
/// S8 list omits SAS. S9 mutation returns may include optional SAS comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRequestDto {
    pub flow_id: String,
    pub other_user_id: String,
    pub other_device_id: Option<String>,
    pub direction: String,
    pub phase: String,
    pub started_ts: Option<u64>,
    pub sas: Option<VerificationSasDto>,
}

/// Privacy-safe verification inbox. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationInboxDto {
    pub session_generation: u64,
    pub requests: Vec<VerificationRequestDto>,
}

/// Static fail-closed verification-list error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationListError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for VerificationListError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for VerificationListError {}

fn verification_list_failed(
    code: &'static str,
    description: &'static str,
) -> VerificationListError {
    VerificationListError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_verification_list_core_error(error: MatrixIpcError) -> VerificationListError {
    match error.diagnostic_id.as_deref() {
        Some("p2-verification-list-no-session") => verification_list_failed(
            VERIFICATION_LIST_NO_SESSION_CODE,
            VERIFICATION_LIST_NO_SESSION_DESCRIPTION,
        ),
        _ => verification_list_failed(
            VERIFICATION_LIST_FAILED_CODE,
            VERIFICATION_LIST_FAILED_DESCRIPTION,
        ),
    }
}

fn verification_direction_as_str(direction: NativeVerificationDirection) -> String {
    match direction {
        NativeVerificationDirection::Incoming => "incoming",
        NativeVerificationDirection::Outgoing => "outgoing",
    }
    .to_owned()
}

fn verification_phase_as_str(phase: NativeVerificationPhase) -> String {
    match phase {
        NativeVerificationPhase::Requested => "requested",
        NativeVerificationPhase::Ready => "ready",
        NativeVerificationPhase::Started => "started",
        NativeVerificationPhase::SasReady => "sas_ready",
        NativeVerificationPhase::Confirmed => "confirmed",
        NativeVerificationPhase::Done => "done",
        NativeVerificationPhase::Mismatched => "mismatched",
        NativeVerificationPhase::Cancelled => "cancelled",
    }
    .to_owned()
}

fn verification_request_dto(request: NativeVerificationRequest) -> VerificationRequestDto {
    VerificationRequestDto {
        flow_id: request.flow_id,
        other_user_id: request.other_user_id,
        other_device_id: request.other_device_id,
        direction: verification_direction_as_str(request.direction),
        phase: verification_phase_as_str(request.phase),
        started_ts: request.started_ts,
        sas: None,
    }
}

fn verification_emoji_dto(emoji: NativeVerificationEmoji) -> VerificationEmojiDto {
    VerificationEmojiDto {
        symbol: emoji.symbol,
        description: emoji.description,
    }
}

fn verification_sas_dto(sas: NativeVerificationSas) -> VerificationSasDto {
    VerificationSasDto {
        emoji: sas
            .emoji
            .map(|emoji| emoji.into_iter().map(verification_emoji_dto).collect()),
        decimals: sas.decimals.map(|decimals| decimals.to_vec()),
    }
}

fn verification_request_dto_with_sas(request: NativeVerificationRequest) -> VerificationRequestDto {
    VerificationRequestDto {
        flow_id: request.flow_id,
        other_user_id: request.other_user_id,
        other_device_id: request.other_device_id,
        direction: verification_direction_as_str(request.direction),
        phase: verification_phase_as_str(request.phase),
        started_ts: request.started_ts,
        sas: request.sas.map(verification_sas_dto),
    }
}

/// Static fail-closed verification-SAS error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationSasError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for VerificationSasError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for VerificationSasError {}

fn verification_sas_failed(code: &str, description: &'static str) -> VerificationSasError {
    VerificationSasError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_verification_sas_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> VerificationSasError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            verification_sas_failed(code, VERIFICATION_SAS_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-crypto.1-") => {
            verification_sas_failed(code, VERIFICATION_SAS_OWNER_DESCRIPTION)
        }
        _ => verification_sas_failed(
            VERIFICATION_SAS_FAILED_CODE,
            VERIFICATION_SAS_FAILED_DESCRIPTION,
        ),
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

    pub async fn typing_snapshot(&self) -> Result<TypingSnapshotDto, TypingCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: TYPING_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_typing_snapshot_core_error)?;
        let snapshot: NativeTypingSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                typing_failed(
                    TYPING_SNAPSHOT_FAILED_CODE,
                    TYPING_SNAPSHOT_FAILED_DESCRIPTION,
                )
            })?;
        Ok(TypingSnapshotDto {
            session_generation: snapshot.session_generation,
            rooms: snapshot
                .rooms
                .into_iter()
                .map(|room| TypingRoomDto {
                    room_id: room.room_id,
                    user_ids: room.user_ids,
                })
                .collect(),
        })
    }

    pub async fn typing_set(
        &self,
        room_id: String,
        typing: bool,
    ) -> Result<(), TypingCommandError> {
        self.core
            .command(CommandEnvelope {
                command: TYPING_SET_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "roomId": room_id, "typing": typing }),
            })
            .await
            .map_err(map_typing_set_core_error)?;
        Ok(())
    }

    pub async fn presence_snapshot(
        &self,
        user_id: String,
    ) -> Result<PresenceSnapshotDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(map_presence_snapshot_core_error)?;
        let result: NativePresenceSnapshotResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                presence_failed(
                    PRESENCE_SNAPSHOT_FAILED_CODE,
                    PRESENCE_SNAPSHOT_FAILED_DESCRIPTION,
                )
            })?;
        Ok(presence_snapshot_dto(result))
    }

    pub async fn presence_subscribe(
        &self,
        user_id: String,
    ) -> Result<PresenceSubscriptionDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SUBSCRIBE_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(map_presence_subscribe_core_error)?;
        let subscription: NativePresenceSubscription = serde_json::from_value(response.payload)
            .map_err(|_| {
                presence_failed(
                    PRESENCE_SUBSCRIBE_FAILED_CODE,
                    PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(PresenceSubscriptionDto {
            subscription_id: subscription.subscription_id,
            user_id: subscription.user_id,
            session_generation: subscription.session_generation,
        })
    }

    pub async fn presence_unsubscribe(
        &self,
        subscription_id: String,
    ) -> Result<(), PresenceCommandError> {
        self.core
            .command(CommandEnvelope {
                command: PRESENCE_UNSUBSCRIBE_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "subscriptionId": subscription_id }),
            })
            .await
            .map_err(map_presence_unsubscribe_core_error)?;
        Ok(())
    }

    pub async fn verification_list(&self) -> Result<VerificationInboxDto, VerificationListError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: VERIFICATION_LIST_COMMAND.to_owned(),
                session_generation: VERIFICATION_LIST_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_verification_list_core_error)?;
        let inbox: NativeVerificationInbox =
            serde_json::from_value(response.payload).map_err(|_| {
                verification_list_failed(
                    VERIFICATION_LIST_FAILED_CODE,
                    VERIFICATION_LIST_FAILED_DESCRIPTION,
                )
            })?;
        Ok(VerificationInboxDto {
            session_generation: inbox.session_generation,
            requests: inbox
                .requests
                .into_iter()
                .map(verification_request_dto)
                .collect(),
        })
    }

    pub async fn verification_start(
        &self,
        device_id: Option<String>,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: VERIFICATION_START_COMMAND.to_owned(),
                session_generation: VERIFICATION_SAS_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "deviceId": device_id }),
            })
            .await
            .map_err(|error| {
                map_verification_sas_core_error(VERIFICATION_START_NO_SESSION_CODE, error)
            })?;
        parse_verification_sas_request(response.payload)
    }

    pub async fn verification_accept(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_ACCEPT_COMMAND,
            VERIFICATION_ACCEPT_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_begin_sas(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_BEGIN_SAS_COMMAND,
            VERIFICATION_BEGIN_SAS_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_confirm(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_CONFIRM_COMMAND,
            VERIFICATION_CONFIRM_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_mismatch(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_MISMATCH_COMMAND,
            VERIFICATION_MISMATCH_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_cancel(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_CANCEL_COMMAND,
            VERIFICATION_CANCEL_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_dismiss(&self, flow_id: String) -> Result<(), VerificationSasError> {
        self.core
            .command(CommandEnvelope {
                command: VERIFICATION_DISMISS_COMMAND.to_owned(),
                session_generation: VERIFICATION_SAS_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "flowId": flow_id }),
            })
            .await
            .map_err(|error| {
                map_verification_sas_core_error(VERIFICATION_DISMISS_NO_SESSION_CODE, error)
            })?;
        Ok(())
    }

    async fn verification_flow_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: VERIFICATION_SAS_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "flowId": flow_id }),
            })
            .await
            .map_err(|error| map_verification_sas_core_error(no_session, error))?;
        parse_verification_sas_request(response.payload)
    }

    pub async fn device_snapshot(&self) -> Result<DeviceSnapshotDto, DeviceCommandError> {
        let response = self
            .device_null_command(DEVICE_SNAPSHOT_COMMAND, DEVICE_SNAPSHOT_NO_SESSION_CODE)
            .await?;
        let snapshot: NativeDeviceSnapshot = serde_json::from_value(response)
            .map_err(|_| device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION))?;
        Ok(device_snapshot_dto(snapshot))
    }

    pub async fn device_rename(
        &self,
        device_id: String,
        display_name: String,
    ) -> Result<DeviceSnapshotDto, DeviceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: DEVICE_RENAME_COMMAND.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "deviceId": device_id,
                    "displayName": display_name,
                }),
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_RENAME_NO_SESSION_CODE, error))?;
        let snapshot: NativeDeviceSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION))?;
        Ok(device_snapshot_dto(snapshot))
    }

    pub async fn device_delete_start(
        &self,
        device_ids: Vec<String>,
    ) -> Result<DeviceDeleteDto, DeviceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: DEVICE_DELETE_START_COMMAND.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "deviceIds": device_ids }),
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_DELETE_START_NO_SESSION_CODE, error))?;
        let result: NativeDeviceDeleteResult = serde_json::from_value(response.payload)
            .map_err(|_| device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION))?;
        Ok(device_delete_dto(result))
    }

    pub async fn device_delete_cancel(
        &self,
        operation_id: u64,
        session_generation: u64,
    ) -> Result<(), DeviceCommandError> {
        self.core
            .command(CommandEnvelope {
                command: DEVICE_DELETE_CANCEL_COMMAND.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "operationId": operation_id,
                    "sessionGeneration": session_generation,
                }),
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_DELETE_CANCEL_NO_SESSION_CODE, error))?;
        Ok(())
    }

    pub async fn room_join_rule_snapshot(
        &self,
        room_id: String,
        session_generation: u64,
    ) -> Result<RoomJoinRuleSnapshotDto, JoinRuleCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: JOIN_RULE_SNAPSHOT_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "sessionGeneration": session_generation,
                }),
            })
            .await
            .map_err(|error| map_join_rule_core_error(error))?;
        let snapshot: MatrixRoomJoinRuleSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION))?;
        Ok(RoomJoinRuleSnapshotDto {
            status: snapshot.status,
            room_id: snapshot.room_id,
            session_generation: snapshot.session_generation,
            join_rule: snapshot.join_rule,
        })
    }

    pub async fn get_global_image_packs(
        &self,
    ) -> Result<GlobalImagePacksSnapshotDto, ImagePackCommandError> {
        let payload = self
            .image_pack_null_command(
                GET_GLOBAL_IMAGE_PACKS_COMMAND,
                GET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE,
            )
            .await?;
        let snapshot: NativeGlobalImagePacksSnapshot =
            serde_json::from_value(payload).map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(GlobalImagePacksSnapshotDto {
            session_generation: snapshot.session_generation,
            packs: snapshot
                .packs
                .into_iter()
                .map(image_pack_dto)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub async fn get_user_image_pack(
        &self,
    ) -> Result<UserImagePackSnapshotDto, ImagePackCommandError> {
        let payload = self
            .image_pack_null_command(
                GET_USER_IMAGE_PACK_COMMAND,
                GET_USER_IMAGE_PACK_NO_SESSION_CODE,
            )
            .await?;
        let snapshot: NativeUserImagePackSnapshot =
            serde_json::from_value(payload).map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(UserImagePackSnapshotDto {
            session_generation: snapshot.session_generation,
            pack: snapshot.pack.map(image_pack_dto).transpose()?,
        })
    }

    pub async fn get_room_image_packs(
        &self,
        room_id: String,
    ) -> Result<RoomImagePacksSnapshotDto, ImagePackCommandError> {
        let payload = self
            .core
            .command(CommandEnvelope {
                command: GET_ROOM_IMAGE_PACKS_COMMAND.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "roomId": room_id }),
            })
            .await
            .map_err(|error| {
                map_image_pack_core_error(GET_ROOM_IMAGE_PACKS_NO_SESSION_CODE, error)
            })?;
        let snapshot: NativeRoomImagePacksSnapshot = serde_json::from_value(payload.payload)
            .map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(RoomImagePacksSnapshotDto {
            session_generation: snapshot.session_generation,
            room_id: snapshot.room_id,
            packs: snapshot
                .packs
                .into_iter()
                .map(image_pack_dto)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub async fn set_user_image_pack(
        &self,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        self.image_pack_set_content(
            SET_USER_IMAGE_PACK_COMMAND,
            SET_USER_IMAGE_PACK_NO_SESSION_CODE,
            content_json,
        )
        .await
    }

    pub async fn set_global_image_packs(
        &self,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        self.image_pack_set_content(
            SET_GLOBAL_IMAGE_PACKS_COMMAND,
            SET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE,
            content_json,
        )
        .await
    }

    pub async fn set_room_image_pack(
        &self,
        room_id: String,
        state_key: String,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        let content = parse_image_pack_content_json(&content_json)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SET_ROOM_IMAGE_PACK_COMMAND.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "stateKey": state_key,
                    "content": content,
                }),
            })
            .await
            .map_err(|error| {
                map_image_pack_core_error(SET_ROOM_IMAGE_PACK_NO_SESSION_CODE, error)
            })?;
        image_pack_write_dto(response.payload)
    }

    async fn image_pack_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, ImagePackCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_image_pack_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn image_pack_set_content(
        &self,
        command: &'static str,
        no_session: &'static str,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        let content = parse_image_pack_content_json(&content_json)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "content": content }),
            })
            .await
            .map_err(|error| map_image_pack_core_error(no_session, error))?;
        image_pack_write_dto(response.payload)
    }

    async fn device_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, DeviceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_device_core_error(no_session, error))?;
        Ok(response.payload)
    }
}

fn parse_verification_sas_request(
    payload: serde_json::Value,
) -> Result<VerificationRequestDto, VerificationSasError> {
    let request: NativeVerificationRequest = serde_json::from_value(payload).map_err(|_| {
        verification_sas_failed(
            VERIFICATION_SAS_FAILED_CODE,
            VERIFICATION_SAS_FAILED_DESCRIPTION,
        )
    })?;
    Ok(verification_request_dto_with_sas(request))
}

/// Privacy-safe device row. Identity/presentation fields only; no keys or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummaryDto {
    pub device_id: String,
    pub display_name: Option<String>,
    pub last_seen_ip: Option<String>,
    pub last_seen_ts: Option<u64>,
    pub trust: String,
    pub is_current: bool,
}

/// Privacy-safe device inbox. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSnapshotDto {
    pub session_generation: u64,
    pub devices: Vec<DeviceSummaryDto>,
}

/// Privacy-safe delete challenge. Authentication type only; no password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDeleteChallengeDto {
    pub operation_id: u64,
    pub session_generation: u64,
    pub authentication: String,
    pub authentication_failed: bool,
}

/// Privacy-safe delete start result. Complete snapshot or challenge; no password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDeleteDto {
    pub outcome: String,
    pub snapshot: Option<DeviceSnapshotDto>,
    pub challenge: Option<DeviceDeleteChallengeDto>,
}

/// Static fail-closed device-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for DeviceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for DeviceCommandError {}

fn device_failed(code: &str, description: &'static str) -> DeviceCommandError {
    DeviceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_device_core_error(no_session: &'static str, error: MatrixIpcError) -> DeviceCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => device_failed(code, DEVICE_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-crypto.7-") => {
            device_failed(code, DEVICE_OWNER_DESCRIPTION)
        }
        _ => device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe join-rule snapshot. Closed vocabulary only; no allow-list or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomJoinRuleSnapshotDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub join_rule: String,
}

/// Static fail-closed join-rule error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinRuleCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for JoinRuleCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for JoinRuleCommandError {}

fn join_rule_failed(code: &str, description: &'static str) -> JoinRuleCommandError {
    JoinRuleCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_join_rule_core_error(error: MatrixIpcError) -> JoinRuleCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == JOIN_RULE_SNAPSHOT_NO_SESSION_CODE => {
            join_rule_failed(code, JOIN_RULE_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-room-profile-join-rule-") => {
            join_rule_failed(code, JOIN_RULE_OWNER_DESCRIPTION)
        }
        _ => join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe image-pack row. Metadata/IDs/mxc URLs/JSON only; never image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePackDto {
    pub id: String,
    pub room_id: Option<String>,
    pub state_key: Option<String>,
    pub content_json: String,
}

/// Privacy-safe user pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserImagePackSnapshotDto {
    pub session_generation: u64,
    pub pack: Option<ImagePackDto>,
}

/// Privacy-safe room pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomImagePacksSnapshotDto {
    pub session_generation: u64,
    pub room_id: String,
    pub packs: Vec<ImagePackDto>,
}

/// Privacy-safe global pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalImagePacksSnapshotDto {
    pub session_generation: u64,
    pub packs: Vec<ImagePackDto>,
}

/// Privacy-safe pack write ack. Status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePackWriteDto {
    pub status: String,
}

/// Static fail-closed image-pack-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImagePackCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ImagePackCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ImagePackCommandError {}

fn image_pack_failed(code: &str, description: &'static str) -> ImagePackCommandError {
    ImagePackCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_image_pack_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ImagePackCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            image_pack_failed(code, IMAGE_PACK_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-pack-") => {
            image_pack_failed(code, IMAGE_PACK_OWNER_DESCRIPTION)
        }
        _ => image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION),
    }
}

fn parse_image_pack_content_json(
    content_json: &str,
) -> Result<serde_json::Value, ImagePackCommandError> {
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(image_pack_failed(
            IMAGE_PACK_FAILED_CODE,
            IMAGE_PACK_FAILED_DESCRIPTION,
        ));
    }
    serde_json::from_str(content_json).map_err(|_| {
        image_pack_failed(
            IMAGE_PACK_INVALID_JSON_CODE,
            IMAGE_PACK_INVALID_JSON_DESCRIPTION,
        )
    })
}

fn image_pack_dto(pack: NativeImagePack) -> Result<ImagePackDto, ImagePackCommandError> {
    let content_json = serde_json::to_string(&pack.content)
        .map_err(|_| image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION))?;
    Ok(ImagePackDto {
        id: pack.id,
        room_id: pack.room_id,
        state_key: pack.state_key,
        content_json,
    })
}

fn image_pack_write_dto(
    payload: serde_json::Value,
) -> Result<ImagePackWriteDto, ImagePackCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION))?;
    Ok(ImagePackWriteDto {
        status: status.to_owned(),
    })
}

fn device_trust_as_str(trust: NativeDeviceTrust) -> String {
    match trust {
        NativeDeviceTrust::Verified => "verified",
        NativeDeviceTrust::Unverified => "unverified",
        NativeDeviceTrust::Unsupported => "unsupported",
    }
    .to_owned()
}

fn device_snapshot_dto(snapshot: NativeDeviceSnapshot) -> DeviceSnapshotDto {
    DeviceSnapshotDto {
        session_generation: snapshot.session_generation,
        devices: snapshot
            .devices
            .into_iter()
            .map(|device| DeviceSummaryDto {
                device_id: device.device_id,
                display_name: device.display_name,
                last_seen_ip: device.last_seen_ip,
                last_seen_ts: device.last_seen_ts,
                trust: device_trust_as_str(device.trust),
                is_current: device.is_current,
            })
            .collect(),
    }
}

fn device_delete_dto(result: NativeDeviceDeleteResult) -> DeviceDeleteDto {
    match result {
        NativeDeviceDeleteResult::Complete { snapshot } => DeviceDeleteDto {
            outcome: "complete".to_owned(),
            snapshot: Some(device_snapshot_dto(snapshot)),
            challenge: None,
        },
        NativeDeviceDeleteResult::AuthenticationRequired { challenge } => DeviceDeleteDto {
            outcome: "authentication_required".to_owned(),
            snapshot: None,
            challenge: Some(DeviceDeleteChallengeDto {
                operation_id: challenge.operation_id,
                session_generation: challenge.session_generation,
                authentication: match challenge.authentication {
                    NativeDeviceDeleteAuthentication::Password => "password".to_owned(),
                },
                authentication_failed: challenge.authentication_failed,
            }),
        },
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
