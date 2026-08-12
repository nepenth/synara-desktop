//! D0.1–D0.3 product password-login, native session, sync, and timeline ownership.
//!
//! This is the only desktop product boundary for password login. The live
//! `matrix_sdk::Client` and all access/refresh tokens remain in the Rust host.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use matrix_sdk::{
    attachment::AttachmentConfig,
    authentication::matrix::MatrixSession,
    encryption::CrossSigningStatus,
    media::{MediaFormat, MediaRequestParameters},
    room::edit::EditedContent,
    room::reply::{EnforceThread, Reply as AttachmentReply},
    ruma::{
        api::client::{
            room::{
                create_room::{self, v3::RoomPreset},
                Visibility,
            },
            state::get_state_event_for_key,
            uiaa,
        },
        events::{
            poll::unstable_response::UnstablePollResponseEventContent,
            relation::{Reply, Thread},
            room::{
                member::MembershipState,
                message::{
                    AddMentions, MessageFormat, MessageType, Relation, RelationWithoutReplacement,
                    ReplacementMetadata, ReplyWithinThread, RoomMessageEventContent,
                    RoomMessageEventContentWithoutRelation,
                },
                power_levels::UserPowerLevel,
                ImageInfo, MediaSource,
            },
            sticker::StickerEventContent,
            AnyInitialStateEvent, AnyMessageLikeEventContent, AnySyncMessageLikeEvent,
            AnySyncTimelineEvent, Mentions, StateEventType,
        },
        serde::Raw,
        EventId, Int, MxcUri, OwnedEventId, OwnedMxcUri, OwnedRoomId, OwnedRoomOrAliasId,
        OwnedServerName, OwnedTransactionId, OwnedUserId, RoomVersionId, UInt,
    },
    Client, Room, RoomMemberships, SessionMeta, SessionTokens,
};
use mime::Mime;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;
use zeroize::Zeroize;

use super::{
    complete_password_reset, discover_login_flows, login_with_password, normalize_homeserver_url,
    password_reset_ephemeral_user_id, probe_register_flows, register_ephemeral_user_id,
    register_submit, request_password_email_token, request_register_email_token, AuthError,
    HttpLoginFlowTransport, LoginFlow, LoginOptions, PasswordEmailTokenResult,
    PasswordResetOutcome, RegisterAuthStage, RegisterFlowsProbe, RegisterSubmitOutcome,
    RegisterUiaFlow,
};
use crate::matrix::account_data::{
    add_room_to_mdirect, clear_completed_later_live, complete_later_item_live,
    complete_room_todo_item_live, delete_room_note_item_live, mark_later_reminded_live,
    move_room_todo_item_live, remove_room_from_mdirect, set_global_image_packs,
    set_room_image_pack, set_user_image_pack, snapshot_global_image_packs, snapshot_later,
    snapshot_mdirect, snapshot_room_image_packs, snapshot_room_notes, snapshot_user_image_pack,
    snooze_later_item_live, upsert_later_item, upsert_room_note_item,
    NativeGlobalImagePacksSnapshot, NativeImagePackOwner, NativeLaterSnapshot,
    NativeMDirectMutationResult, NativeMDirectSnapshot, NativeRoomImagePacksSnapshot,
    NativeRoomNotesSnapshot, NativeUserImagePackSnapshot, RoomNoteMoveDirection, SynaraLaterItem,
    SynaraRoomNoteItem,
};
use crate::matrix::backup::live::{
    self as live_backup, NativeBackupOperationResult, NativeBackupStatus,
};
use crate::matrix::client_builder::{
    build_unauthenticated_client, ClientBuildConfig, ClientBuilderError,
};
use crate::matrix::cross_signing::live::{
    project_status, supported_authentication, NativeCrossSigningSetupOutcome,
    NativeCrossSigningSetupResult, NativeCrossSigningStatus, SupportedBootstrapAuthentication,
};
use crate::matrix::devices::{
    live::{snapshot as live_device_snapshot, supported_delete_authentication},
    NativeDeviceDeleteChallenge, NativeDeviceDeleteResult, NativeDeviceOwner, NativeDeviceSnapshot,
    PendingDeviceDeletion,
};
use crate::matrix::dto::{Membership as ProductMembership, RoomMember as ProductRoomMember};
use crate::matrix::ipc::MAX_WIRE_COUNTER;
use crate::matrix::lifecycle::{
    clear_session_material, persist_session_after_login, restore_session_from_vault,
    restore_session_onto_client, KeyringSessionMaterialVault, SessionMaterial,
};
use crate::matrix::presence::NativePresenceOwner;
use crate::matrix::room_keys::{
    live::{
        self as live_room_keys, NativeRoomKeyFileSelection, NativeRoomKeyTransferResult,
        NativeRoomKeyTransferStatus, SelectedRoomKeyImport,
    },
    RoomKeyTransferFlow,
};
use crate::matrix::room_list::{
    snapshot_from_sync_owner, snapshot_invites, InviteAvatarHandles, NativeInvite,
    NativeInviteSnapshot, NativeRoomListSnapshot,
};
use crate::matrix::room_profile::NativeRoomJoinRuleOwner;
use crate::matrix::secret_storage::live::{
    self as live_secret_storage, NativeSecretStorageOperationResult, NativeSecretStorageStatus,
};
use crate::matrix::send::{
    normalize_poll, poll_response_content, poll_start_content, AttachmentEnqueue, AttachmentKind,
    AttachmentSendQueue, SendQueue,
};
use crate::matrix::spaces::{
    remove_space_child, reparent_restricted_join_allow, set_space_child, snapshot_space_children,
    snapshot_space_hierarchy, snapshot_space_parents, NativeRestrictedJoinReparentResult,
    NativeSpaceChildMutationResult, NativeSpaceChildrenSnapshot, NativeSpaceHierarchySnapshot,
    NativeSpaceParentsSnapshot,
};
use crate::matrix::store::{
    get_or_migrate_store_key, migrate_store_to_current, reset_store_for_recovery, AccountIdentity,
    KeyringStoreKeyVault, StoreKeyMaterial, StoreKeyVaultError, StoreMigrationError, StorePaths,
};
use crate::matrix::sync::{
    build_sync_service, unconfigured_snapshot, SyncReadinessSnapshot, SyncServiceConfig,
    SyncServiceOwner,
};
use crate::matrix::timeline::{
    format_forwarded_media_body, format_forwarded_plain_body, reply_draft_readback,
    should_attach_formatted_body, ComposerDraftRegistry, NativeComposerReplyDraft,
    NativeComposerReplyDraftReadback, NativeComposerReplyDraftRoomRequest,
    NativeComposerSetReplyDraftRequest, NativeReactionMutationResult, NativeTimelineActionKind,
    NativeTimelineActionReadback, NativeTimelineCallDeclineRequest, NativeTimelineCloseRequest,
    NativeTimelineDirection, NativeTimelineEditTextRequest, NativeTimelineEventReadback,
    NativeTimelineForwardMediaRequest, NativeTimelineForwardTextRequest,
    NativeTimelineJumpLatestRequest, NativeTimelineOpenReadback, NativeTimelineOpenRequest,
    NativeTimelinePinRequest, NativeTimelinePollVoteRequest, NativeTimelineReadStateReadback,
    NativeTimelineReadStateRequest, NativeTimelineRedactRequest, NativeTimelineRegistry,
    NativeTimelineReportRequest, NativeTimelineSnapshot, NativeTimelineViewPaginationRequest,
    TimelineMediaSource, NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
};
use crate::matrix::typing::{set_typing_notice, NativeTypingOwner, NativeTypingSnapshot};
use crate::matrix::verification::live::{
    NativeVerificationInbox, NativeVerificationOwner, NativeVerificationRequest,
};

const ACTIVE_SESSION_FILE: &str = "active-session.json";
const MATRIX_DATA_DIR: &str = "matrix";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLoginIdentity {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MatrixSessionSnapshot {
    LoggedOut,
    LoggedIn {
        user_id: String,
        device_id: String,
        homeserver_url: String,
        #[serde(rename = "sessionGeneration")]
        session_generation: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixCrossSigningState {
    Unavailable,
    NotSetUp,
    Partial,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCryptoStatus {
    pub session_generation: u64,
    pub encryption_enabled: bool,
    pub cross_signing_state: MatrixCrossSigningState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixAuthCommandError {
    pub code: &'static str,
    pub message: &'static str,
    pub diagnostic_id: &'static str,
}

/// V-ROOMS.R-MEMBERS-READ — live native room-member projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomMembersSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub members: Vec<ProductRoomMember>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendTextResult {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendAttachmentResult {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendStickerResult {
    pub room_id: String,
    pub event_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendPollResult {
    pub room_id: String,
    pub event_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixPollRespondResult {
    pub room_id: String,
    pub poll_event_id: String,
    pub event_id: String,
    pub status: &'static str,
}

/// V-SEND.R-AVATAR-UPLOAD — result of a native user-profile write
/// (display name or avatar URL). `status` is always `"ok"` on success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixProfileWriteResult {
    pub status: &'static str,
}

/// V-SEND.R-AVATAR-UPLOAD — result of a native media upload for a user
/// avatar. Returns the homeserver `mxc://` URI; no file bytes cross back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixUploadMediaResult {
    pub mxc: String,
}

/// V-SEND.R-MEDIA — the exact media-config result shape (`m.upload.size`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixMediaConfigResult {
    #[serde(rename = "m.upload.size")]
    pub upload_size: u64,
}

/// V-SEND.R-MEDIA — original-file bytes returned by the native media owner.
/// This DTO is intentionally not part of a versioned Matrix envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MatrixMediaDownloadResult {
    pub bytes: Vec<u8>,
}

/// V-SEND.R-MEDIA — camelCase request used by the native media owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixMediaDownloadRequest {
    pub content_uri: String,
}

/// V-ROOMS.R-POWERS-BULK — acknowledged complete state replacement.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePowerLevelWriteResult {
    pub status: &'static str,
    pub room_id: String,
    pub event_type: &'static str,
    pub state_key: &'static str,
    pub session_generation: u64,
    pub content: serde_json::Value,
}

/// JSON-friendly create-room request owned by the desktop Matrix SDK route.
/// `parent_room_id` is used for restricted join rules; the post-create space
/// child edge remains an explicit `matrix_space_child_set` operation in TS.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomCreateRequest {
    pub name: Option<String>,
    pub topic: Option<String>,
    pub room_version: Option<String>,
    pub room_alias_name: Option<String>,
    #[serde(default)]
    pub is_direct: bool,
    #[serde(default)]
    pub invite: Vec<String>,
    pub visibility: Option<MatrixRoomCreateVisibility>,
    pub preset: Option<MatrixRoomCreatePreset>,
    pub creation_content: Option<MatrixRoomCreateContent>,
    #[serde(default)]
    pub encryption: bool,
    pub join_rule: Option<String>,
    #[serde(default)]
    pub knock: bool,
    pub parent_room_id: Option<String>,
    pub power_level_content_override: Option<MatrixRoomCreatePowerLevels>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixRoomCreateVisibility {
    Private,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixRoomCreatePreset {
    #[serde(rename = "private_chat")]
    Private,
    #[serde(rename = "public_chat")]
    Public,
    #[serde(rename = "trusted_private_chat")]
    TrustedPrivate,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomCreateContent {
    #[serde(rename = "type")]
    pub room_type: Option<String>,
    #[serde(rename = "m.federate", alias = "federate")]
    pub federate: Option<bool>,
    pub additional_creators: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomCreatePowerLevels {
    pub events_default: Option<i64>,
    #[serde(default)]
    pub events: BTreeMap<String, i64>,
}

/// Soft IPC/body cap for one-shot composer attachment transfer (bytes).
const MAX_ATTACHMENT_IPC_BYTES: usize = 32 * 1024 * 1024;

/// Maximum bounded identifier accepted by the direct CallWidget media path.
const MAX_CALL_WIDGET_MEDIA_URI_BYTES: usize = 2 * 1024;

/// V-SEND.R-CALL-MEDIA response ceiling. Never truncate over-limit content.
const MAX_CALL_WIDGET_MEDIA_DOWNLOAD_BYTES: usize = MAX_ATTACHMENT_IPC_BYTES;

/// Soft IPC/body cap for one-shot user-avatar media transfer (bytes).
/// Avatars are small images; 8 MiB is generous and keeps the webview buffer
/// bounded (well under the 32 MiB attachment cap).
const MAX_AVATAR_IPC_BYTES: usize = 8 * 1024 * 1024;

/// Power-level state is a JSON state event, so it uses the normal bounded
/// Matrix IPC payload policy rather than an unbounded serde_json value.
const MAX_POWER_LEVEL_CONTENT_JSON_BYTES: usize =
    crate::matrix::ipc::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
const MAX_POWER_LEVEL_TEXT_BYTES: usize = 4 * 1024;

impl MatrixAuthCommandError {
    pub(crate) fn new(
        code: &'static str,
        message: &'static str,
        diagnostic_id: &'static str,
    ) -> Self {
        Self {
            code,
            message,
            diagnostic_id,
        }
    }

    fn invalid_input(diagnostic_id: &'static str) -> Self {
        Self::new(
            "InvalidRequest",
            "The native Matrix login request is invalid.",
            diagnostic_id,
        )
    }

    fn unavailable(diagnostic_id: &'static str) -> Self {
        Self::new(
            "Unknown",
            "Native Matrix session storage is unavailable.",
            diagnostic_id,
        )
    }
}

struct ManagedMatrixSession {
    client: Client,
    identity: MatrixLoginIdentity,
    sync: SyncServiceOwner,
    invite_avatars: InviteAvatarHandles,
    timelines: NativeTimelineRegistry,
    composer_drafts: ComposerDraftRegistry,
    sends: SendQueue,
    attachments: AttachmentSendQueue,
    verification: NativeVerificationOwner,
    _devices: NativeDeviceOwner,
    _image_packs: NativeImagePackOwner,
    typing: NativeTypingOwner,
    presence: NativePresenceOwner,
    join_rules: NativeRoomJoinRuleOwner,
    pending_device_deletion: Option<PendingDeviceDeletion>,
    next_device_delete_operation_id: u64,
    pending_cross_signing_auth_session: Option<String>,
    room_key_transfer: Arc<Mutex<RoomKeyTransferFlow>>,
    selected_room_key_import: Option<SelectedRoomKeyImport>,
    next_room_key_import_selection_id: u64,
}

/// A locally held recovery target is armed only by a failed native login.
/// It never crosses IPC; the renderer receives only an opaque, one-use
/// confirmation capability after the user opens the recovery confirmation.
#[derive(Default)]
enum StoreRecoveryState {
    #[default]
    Idle,
    Pending {
        identity: AccountIdentity,
    },
    AwaitingConfirmation {
        identity: AccountIdentity,
        confirmation_id: String,
    },
}

#[derive(Default)]
pub struct MatrixAuthState {
    session: Mutex<Option<ManagedMatrixSession>>,
    store_recovery: Mutex<StoreRecoveryState>,
    next_session_generation: AtomicU64,
}

impl MatrixAuthState {
    pub fn new() -> Self {
        Self::default()
    }

    /// A normal login supersedes any abandoned recovery affordance. This only
    /// clears a process-local capability; it does not touch files or Keychain.
    pub(super) async fn clear_store_recovery(&self) {
        *self.store_recovery.lock().await = StoreRecoveryState::Idle;
    }

    /// Remember an account only after an allowlisted failed store-open path.
    /// The identity remains in the host and is never returned by recovery IPC.
    pub(super) async fn arm_store_recovery(&self, identity: AccountIdentity) {
        *self.store_recovery.lock().await = StoreRecoveryState::Pending { identity };
    }

    pub(super) async fn prepare_store_recovery_confirmation(
        &self,
    ) -> Result<String, MatrixAuthCommandError> {
        let mut recovery = self.store_recovery.lock().await;
        let identity = match std::mem::replace(&mut *recovery, StoreRecoveryState::Idle) {
            StoreRecoveryState::Pending { identity } => identity,
            StoreRecoveryState::Idle | StoreRecoveryState::AwaitingConfirmation { .. } => {
                return Err(MatrixAuthCommandError::new(
                    "InvalidRequest",
                    "Local Matrix store recovery must be requested from a failed login.",
                    "p3.2-login-store-recovery-not-pending",
                ));
            }
        };
        let confirmation_id = new_store_recovery_confirmation_id()?;
        *recovery = StoreRecoveryState::AwaitingConfirmation {
            identity,
            confirmation_id: confirmation_id.clone(),
        };
        Ok(confirmation_id)
    }

    /// Consume the CSPRNG confirmation capability before filesystem work so it
    /// cannot be replayed after either success or failure. The fixed typed
    /// acknowledgement is a second independent host-side requirement; neither
    /// a renderer button state nor a valid opaque ID alone can authorize an
    /// archive. Wrong input leaves a pending capability untouched so the user
    /// can correct a transport/UI error without rearming recovery from a new
    /// login failure.
    pub(super) async fn take_confirmed_store_recovery(
        &self,
        confirmation_id: &str,
        confirmation_text: &str,
    ) -> Result<AccountIdentity, MatrixAuthCommandError> {
        if confirmation_text != STORE_RECOVERY_TYPED_CONFIRMATION_TEXT
            || !is_store_recovery_confirmation_id(confirmation_id)
        {
            return Err(store_recovery_confirmation_error());
        }
        let mut recovery = self.store_recovery.lock().await;
        let valid = matches!(
            &*recovery,
            StoreRecoveryState::AwaitingConfirmation {
                confirmation_id: expected,
                ..
            } if expected == confirmation_id
        );
        if !valid {
            return Err(store_recovery_confirmation_error());
        }
        match std::mem::replace(&mut *recovery, StoreRecoveryState::Idle) {
            StoreRecoveryState::AwaitingConfirmation { identity, .. } => Ok(identity),
            StoreRecoveryState::Idle | StoreRecoveryState::Pending { .. } => {
                Err(store_recovery_confirmation_error())
            }
        }
    }

    /// Resolve an opaque V-ROOMS invite-avatar capability for the native URI
    /// protocol. The handle is valid only for the live session generation and
    /// never reveals its MXC source to the webview or command IPC.
    pub async fn resolve_invite_avatar(
        &self,
        handle: &str,
    ) -> Option<(Client, crate::matrix::room_list::InviteAvatarSource)> {
        let session = self.session.lock().await;
        let active = session.as_ref()?;
        let source = active
            .invite_avatars
            .resolve(active.sync.session_generation(), handle)?;
        Some((active.client.clone(), source))
    }

    /// Resolve a stream/session-bound V-TIMELINE media capability. Neither the
    /// SDK media source nor downloaded bytes cross command IPC.
    pub async fn resolve_timeline_media(
        &self,
        handle: &str,
    ) -> Option<(Client, TimelineMediaSource)> {
        let session = self.session.lock().await;
        let active = session.as_ref()?;
        let source = active.timelines.resolve_media(handle).await?;
        Some((active.client.clone(), source))
    }
}

const STORE_RECOVERY_CONFIRMATION_ID_BYTES: usize = 32;
/// Exact acknowledgement that the host requires in addition to the opaque
/// CSPRNG confirmation capability. This is intentionally validated only in
/// the native process; renderer-side button state is not an authorization
/// boundary.
pub(super) const STORE_RECOVERY_TYPED_CONFIRMATION_TEXT: &str = "ARCHIVE";

/// Produce an opaque, CSPRNG-backed, one-use confirmation capability. It is
/// neither a Matrix credential nor a store key, and it is never logged.
fn new_store_recovery_confirmation_id() -> Result<String, MatrixAuthCommandError> {
    let mut bytes = [0_u8; STORE_RECOVERY_CONFIRMATION_ID_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "Local Matrix store recovery confirmation is unavailable.",
            "p3.2-login-store-recovery-confirmation-unavailable",
        )
    })?;
    let mut id = String::with_capacity(STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(id, "{byte:02x}");
    }
    Ok(id)
}

fn is_store_recovery_confirmation_id(value: &str) -> bool {
    value.len() == STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn store_recovery_confirmation_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "Local Matrix store recovery confirmation is invalid or has expired.",
        "p3.2-login-store-recovery-confirmation-required",
    )
}

// Shared fail-closed session guards used by the domain command modules.
fn require_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        )
    })
}

fn require_verification_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.1-verification-requires-session",
        )
    })
}

fn require_device_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.7-device-requires-session",
        )
    })
}

fn require_device_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.7-device-requires-session",
        )
    })
}

fn require_cross_signing_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-requires-session",
        )
    })
}

fn require_cross_signing_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-requires-session",
        )
    })
}

fn require_backup_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.3-backup-requires-session",
        )
    })
}

fn require_secret_storage_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.4-secret-storage-requires-session",
        )
    })
}

fn require_room_key_session(
    session: Option<&ManagedMatrixSession>,
) -> Result<&ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.5-room-keys-requires-session",
        )
    })
}

fn require_room_key_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.5-room-keys-requires-session",
        )
    })
}

async fn require_current_room_key_generation(
    state: &State<'_, MatrixAuthState>,
    generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    if require_room_key_session(session.as_ref())?
        .sync
        .session_generation()
        != generation
    {
        return Err(stale_room_key_generation_error());
    }
    Ok(())
}

fn stale_room_key_generation_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "StaleSessionGeneration",
        "The native Matrix session changed during room-key transfer.",
        "v-crypto.5-stale-session-generation",
    )
}

fn require_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        )
    })
}

#[path = "../account_data/product_commands.rs"]
mod account_data;
#[path = "product_commands.rs"]
mod auth_commands;
#[path = "../backup/product_commands.rs"]
mod backup;
#[path = "../cross_signing/product_commands.rs"]
mod cross_signing;
#[path = "../devices/product_commands.rs"]
mod devices;
#[path = "../media/product_commands.rs"]
mod media;
#[path = "../members/product_commands.rs"]
mod members;
#[path = "../presence/product_commands.rs"]
mod presence;
#[path = "../room_directory/product_commands.rs"]
mod room_directory;
#[path = "../room_keys/product_commands.rs"]
mod room_keys;
#[path = "../room_list/product_commands.rs"]
mod room_list;
#[path = "../room_ops/product_commands.rs"]
mod room_ops;
#[path = "../room_profile/product_commands.rs"]
mod room_profile;
#[path = "../secret_storage/product_commands.rs"]
mod secret_storage;
#[path = "../send/product_commands.rs"]
mod send;
#[path = "../spaces/product_commands.rs"]
mod spaces;
#[path = "../timeline/product_commands.rs"]
mod timeline;
#[path = "../typing/product_commands.rs"]
mod typing;
#[path = "../user_profile/product_commands.rs"]
mod user_profile;
#[path = "../verification/product_commands.rs"]
mod verification;
pub use account_data::*;
pub use auth_commands::*;
pub use backup::*;
pub use cross_signing::*;
pub use devices::*;
pub use media::*;
pub use members::*;
pub use presence::*;
pub use room_directory::*;
pub use room_keys::*;
pub use room_list::*;
pub use room_ops::*;
pub use room_profile::*;
pub use secret_storage::*;
pub use send::*;
pub use spaces::*;
pub use timeline::*;
pub use typing::*;
pub use user_profile::*;
pub use verification::*;

#[cfg(test)]
#[path = "product_tests.rs"]
mod tests;
