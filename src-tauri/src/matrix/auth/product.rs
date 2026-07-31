//! D0.1–D0.3 product password-login, native session, sync, and timeline ownership.
//!
//! This is the only desktop product boundary for password login. The live
//! `matrix_sdk::Client` and all access/refresh tokens remain in the Rust host.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use matrix_sdk::{
    attachment::AttachmentConfig,
    encryption::CrossSigningStatus,
    room::edit::EditedContent,
    room::reply::{EnforceThread, Reply as AttachmentReply},
    ruma::{
        api::client::uiaa,
        events::{
            poll::unstable_response::UnstablePollResponseEventContent,
            relation::{Reply, Thread},
            room::{
                message::{
                    AddMentions, MessageFormat, MessageType, Relation, ReplyWithinThread,
                    RoomMessageEventContent, RoomMessageEventContentWithoutRelation,
                },
                ImageInfo,
            },
            sticker::StickerEventContent,
            AnyMessageLikeEventContent, AnySyncMessageLikeEvent, AnySyncTimelineEvent, Mentions,
        },
        EventId, MxcUri, OwnedEventId, OwnedMxcUri, OwnedRoomId, OwnedTransactionId, OwnedUserId,
        UInt,
    },
    Client, Room,
};
use mime::Mime;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;
use zeroize::Zeroize;

use super::{
    complete_password_reset, login_with_password, normalize_homeserver_url,
    password_reset_ephemeral_user_id, request_password_email_token, AuthError, LoginOptions,
    PasswordEmailTokenResult, PasswordResetOutcome,
};
use crate::matrix::account_data::{
    add_room_to_mdirect, clear_completed_later_live, complete_later_item_live,
    complete_room_todo_item_live, delete_room_note_item_live, mark_later_reminded_live,
    move_room_todo_item_live, remove_room_from_mdirect, snapshot_later, snapshot_mdirect,
    snapshot_room_notes, snooze_later_item_live, upsert_later_item, upsert_room_note_item,
    NativeLaterSnapshot, NativeMDirectMutationResult, NativeMDirectSnapshot,
    NativeRoomNotesSnapshot, RoomNoteMoveDirection, SynaraLaterItem, SynaraRoomNoteItem,
};
use crate::matrix::backup::live::{
    self as live_backup, NativeBackupOperationResult, NativeBackupStatus,
};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::cross_signing::live::{
    project_status, supported_authentication, NativeCrossSigningSetupOutcome,
    NativeCrossSigningSetupResult, NativeCrossSigningStatus, SupportedBootstrapAuthentication,
};
use crate::matrix::devices::{
    live::{snapshot as live_device_snapshot, supported_delete_authentication},
    NativeDeviceDeleteChallenge, NativeDeviceDeleteResult, NativeDeviceOwner, NativeDeviceSnapshot,
    PendingDeviceDeletion,
};
use crate::matrix::lifecycle::{
    clear_session_material, persist_session_after_login, restore_session_from_vault,
    KeyringSessionMaterialVault,
};
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
use crate::matrix::secret_storage::live::{
    self as live_secret_storage, NativeSecretStorageOperationResult, NativeSecretStorageStatus,
};
use crate::matrix::send::{
    normalize_poll, poll_response_content, poll_start_content, AttachmentEnqueue, AttachmentKind,
    AttachmentSendQueue, SendQueue,
};
use crate::matrix::spaces::{
    snapshot_space_hierarchy, snapshot_space_parents, NativeSpaceHierarchySnapshot,
    NativeSpaceParentsSnapshot,
};
use crate::matrix::store::{
    get_or_create_store_key, AccountIdentity, KeyringStoreKeyVault, StoreKeyId, StoreKeyMaterial,
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

/// Soft IPC/body cap for one-shot composer attachment transfer (bytes).
const MAX_ATTACHMENT_IPC_BYTES: usize = 32 * 1024 * 1024;

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
    typing: NativeTypingOwner,
    pending_device_deletion: Option<PendingDeviceDeletion>,
    next_device_delete_operation_id: u64,
    pending_cross_signing_auth_session: Option<String>,
    room_key_transfer: Arc<Mutex<RoomKeyTransferFlow>>,
    selected_room_key_import: Option<SelectedRoomKeyImport>,
    next_room_key_import_selection_id: u64,
}

#[derive(Default)]
pub struct MatrixAuthState {
    session: Mutex<Option<ManagedMatrixSession>>,
    next_session_generation: AtomicU64,
}

impl MatrixAuthState {
    pub fn new() -> Self {
        Self::default()
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

#[tauri::command]
pub async fn matrix_login_password(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    homeserver_url: String,
    user: String,
    password: String,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    if session.is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "d0.1-session-already-active",
        ));
    }

    let homeserver_url = normalize_homeserver_url(&homeserver_url)
        .map_err(map_auth_error)?
        .into_string();
    let requested_identity = AccountIdentity::new(&user, &homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-invalid-user-identity"))?;
    let app_data_root = app_data_root(&app)?;
    let client = build_client(&app_data_root, requested_identity.clone()).await?;

    let result = login_with_password(
        &client,
        requested_identity.user_id(),
        &password,
        &LoginOptions {
            request_refresh_token: true,
            ..LoginOptions::default()
        },
    )
    .await
    .map_err(map_auth_error)?;

    let live_identity = AccountIdentity::new(&result.user_id, &result.homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-login-identity-invalid"))?;
    if live_identity != requested_identity {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "The authenticated Matrix identity did not match the requested account.",
            "d0.1-login-identity-mismatch",
        ));
    }

    ensure_crypto_ready(&client).await?;
    let session_generation = state.next_generation();
    let verification = NativeVerificationOwner::new(&client, session_generation);
    let devices = NativeDeviceOwner::start(&client, app.clone(), session_generation)
        .await
        .map_err(map_device_error)?;
    let typing = NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?;
    let sync = start_sync_owner(&client, session_generation).await?;
    let session_vault = KeyringSessionMaterialVault::new();
    persist_session_after_login(&client, &live_identity, &session_vault)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-persist-failed"))?;

    let identity = MatrixLoginIdentity {
        user_id: result.user_id,
        device_id: result.device_id,
        homeserver_url: result.homeserver_url,
    };
    if let Err(error) = write_active_identity(&app_data_root, &identity) {
        let _ = clear_session_material(&session_vault, &live_identity);
        return Err(error);
    }

    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync,
        invite_avatars: InviteAvatarHandles::new(session_generation),
        timelines: NativeTimelineRegistry::new(session_generation),
        composer_drafts: ComposerDraftRegistry::new(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification,
        _devices: devices,
        typing,
        pending_device_deletion: None,
        next_device_delete_operation_id: 0,
        pending_cross_signing_auth_session: None,
        room_key_transfer: Arc::new(Mutex::new(RoomKeyTransferFlow::new(session_generation))),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok(identity)
}

/// V-AUTH.4a — request a password-reset email token (unauthenticated CS API).
///
/// Does not create a product login session. Never logs email, client_secret, or sid.
#[tauri::command]
pub async fn matrix_password_reset_request_email_token(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    client_secret: <SET_IN_CONFIG>
    send_attempt: u32,
) -> Result<PasswordEmailTokenResult, MatrixAuthCommandError> {
    let client_secret = zeroize::Zeroizing::new(client_secret);
    let client = build_password_reset_client(&app, &homeserver_url).await?;
    request_password_email_token(&client, &email, client_secret.as_str(), send_attempt)
        .await
        .map_err(map_password_reset_auth_error)
}

/// V-AUTH.4a — complete password reset with email-identity (+ optional password) UIAA.
///
/// Host owns the stages required by the retained desktop flow. Unsupported UIAA
/// stages fail closed. Never logs password, client_secret, or sid.
#[tauri::command]
pub async fn matrix_password_reset_complete(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    new_password: String,
    client_secret: <SET_IN_CONFIG>
    sid: String,
) -> Result<PasswordResetOutcome, MatrixAuthCommandError> {
    let new_password = zeroize::Zeroizing::new(new_password);
    let client_secret = zeroize::Zeroizing::new(client_secret);
    let client = build_password_reset_client(&app, &homeserver_url).await?;
    complete_password_reset(
        &client,
        &email,
        new_password.as_str(),
        client_secret.as_str(),
        &sid,
    )
    .await
    .map_err(map_password_reset_auth_error)
}

#[tauri::command]
pub async fn matrix_session_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    Ok(snapshot(session.as_ref()))
}

#[tauri::command]
pub async fn matrix_sync_status(
    state: State<'_, MatrixAuthState>,
) -> Result<SyncReadinessSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    Ok(match session.as_ref() {
        Some(active) => active.sync.observe(),
        None => unconfigured_snapshot(state.current_generation()),
    })
}

#[tauri::command]
pub async fn matrix_crypto_status(
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixCryptoStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let Some(active) = session.as_ref() else {
        return Ok(crypto_status(state.current_generation(), None));
    };
    let cross_signing = active.client.encryption().cross_signing_status().await;
    Ok(crypto_status(
        active.sync.session_generation(),
        cross_signing,
    ))
}

#[tauri::command]
pub async fn matrix_cross_signing_status(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_cross_signing_session(session.as_ref())?;
    live_cross_signing_status(active).await
}

#[tauri::command]
pub async fn matrix_cross_signing_setup(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_cross_signing_session_mut(session.as_mut())?;
    let before = live_cross_signing_status(active).await?;
    if before.bootstrap != crate::matrix::cross_signing::live::NativeCrossSigningBootstrap::Needed {
        active.pending_cross_signing_auth_session = None;
        return Ok(NativeCrossSigningSetupResult {
            outcome: NativeCrossSigningSetupOutcome::AlreadyConfigured,
            status: before,
        });
    }

    match active
        .client
        .encryption()
        .bootstrap_cross_signing_if_needed(None)
        .await
    {
        Ok(()) => cross_signing_setup_complete(active).await,
        Err(error) => {
            let Some(info) = error.as_uiaa_response() else {
                return Err(cross_signing_setup_error(
                    "v-crypto.2-cross-signing-bootstrap-failed",
                ));
            };
            match supported_authentication(info) {
                Some(SupportedBootstrapAuthentication::Dummy) => {
                    let mut dummy = uiaa::Dummy::new();
                    dummy.session = info.session.clone();
                    active
                        .client
                        .encryption()
                        .bootstrap_cross_signing(Some(uiaa::AuthData::Dummy(dummy)))
                        .await
                        .map_err(|_| {
                            cross_signing_setup_error(
                                "v-crypto.2-cross-signing-dummy-auth-failed",
                            )
                        })?;
                    cross_signing_setup_complete(active).await
                }
                Some(SupportedBootstrapAuthentication::Password) => {
                    let auth_session = info.session.clone().ok_or_else(|| {
                        cross_signing_setup_error(
                            "v-crypto.2-cross-signing-auth-session-missing",
                        )
                    })?;
                    active.pending_cross_signing_auth_session = Some(auth_session);
                    Ok(NativeCrossSigningSetupResult {
                        outcome: NativeCrossSigningSetupOutcome::AuthenticationRequired,
                        status: live_cross_signing_status(active).await?,
                    })
                }
                None => Err(MatrixAuthCommandError::new(
                    "Forbidden",
                    "The homeserver requires an unsupported authentication step for cross-signing setup.",
                    "v-crypto.2-cross-signing-auth-unsupported",
                )),
            }
        }
    }
}

#[tauri::command]
pub async fn matrix_cross_signing_setup_password(
    state: State<'_, MatrixAuthState>,
    mut password: String,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    let result = matrix_cross_signing_setup_password_inner(&state, &password).await;
    password.zeroize();
    result
}

async fn matrix_cross_signing_setup_password_inner(
    state: &State<'_, MatrixAuthState>,
    password: &str,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    if password.is_empty() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "Your account password is required to finish cross-signing setup.",
            "v-crypto.2-cross-signing-password-empty",
        ));
    }

    let mut session = state.session.lock().await;
    let active = require_cross_signing_session_mut(session.as_mut())?;
    let auth_session = active
        .pending_cross_signing_auth_session
        .clone()
        .ok_or_else(|| {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "Start native cross-signing setup before authenticating it.",
                "v-crypto.2-cross-signing-auth-not-pending",
            )
        })?;
    let user_id = active.client.user_id().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-user-missing",
        )
    })?;
    let mut auth = uiaa::Password::new(user_id.to_owned().into(), password.to_owned());
    auth.session = Some(auth_session);

    if let Err(error) = active
        .client
        .encryption()
        .bootstrap_cross_signing(Some(uiaa::AuthData::Password(auth)))
        .await
    {
        if let Some(info) = error.as_uiaa_response() {
            if let Some(auth_session) = info.session.clone() {
                active.pending_cross_signing_auth_session = Some(auth_session);
            }
            return Err(MatrixAuthCommandError::new(
                "Forbidden",
                "Cross-signing setup authentication failed. Check your password and try again.",
                "v-crypto.2-cross-signing-password-rejected",
            ));
        }
        return Err(cross_signing_setup_error(
            "v-crypto.2-cross-signing-auth-failed",
        ));
    }

    cross_signing_setup_complete(active).await
}

#[tauri::command]
pub async fn matrix_backup_status(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeBackupStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::status(&active.client, active.sync.session_generation()).await
}

#[tauri::command]
pub async fn matrix_backup_setup(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let result = matrix_backup_setup_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

async fn matrix_backup_setup_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    if passphrase.is_empty() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A recovery passphrase is required to set up encryption backup.",
            "v-crypto.3-setup-passphrase-empty",
        ));
    }
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::setup(&active.client, active.sync.session_generation(), passphrase).await
}

#[tauri::command]
pub async fn matrix_backup_restore(
    state: State<'_, MatrixAuthState>,
    mut recovery_secret: String,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let result = matrix_backup_restore_inner(&state, &recovery_secret).await;
    recovery_secret.zeroize();
    result
}

async fn matrix_backup_restore_inner(
    state: &State<'_, MatrixAuthState>,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    require_recovery_secret(recovery_secret)?;
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::restore(
        &active.client,
        active.sync.session_generation(),
        recovery_secret,
    )
    .await
}

#[tauri::command]
pub async fn matrix_backup_repair(
    state: State<'_, MatrixAuthState>,
    mut recovery_secret: String,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    let result = matrix_backup_repair_inner(&state, &recovery_secret).await;
    recovery_secret.zeroize();
    result
}

async fn matrix_backup_repair_inner(
    state: &State<'_, MatrixAuthState>,
    recovery_secret: &str,
) -> Result<NativeBackupOperationResult, MatrixAuthCommandError> {
    require_recovery_secret(recovery_secret)?;
    let session = state.session.lock().await;
    let active = require_backup_session(session.as_ref())?;
    live_backup::repair(
        &active.client,
        active.sync.session_generation(),
        recovery_secret,
    )
    .await
}

#[tauri::command]
pub async fn matrix_secret_storage_status(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeSecretStorageStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::status(&active.client, active.sync.session_generation()).await
}

#[tauri::command]
pub async fn matrix_secret_storage_bootstrap(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result = matrix_secret_storage_bootstrap_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

async fn matrix_secret_storage_bootstrap_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    require_secret_storage_input(passphrase, "v-crypto.4-bootstrap-passphrase-empty")?;
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::bootstrap(&active.client, active.sync.session_generation(), passphrase)
        .await
}

#[tauri::command]
pub async fn matrix_secret_storage_unlock(
    state: State<'_, MatrixAuthState>,
    mut recovery_secret: String,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result = matrix_secret_storage_unlock_inner(&state, &recovery_secret).await;
    recovery_secret.zeroize();
    result
}

async fn matrix_secret_storage_unlock_inner(
    state: &State<'_, MatrixAuthState>,
    recovery_secret: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    require_secret_storage_input(recovery_secret, "v-crypto.4-unlock-secret-empty")?;
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::unlock(
        &active.client,
        active.sync.session_generation(),
        recovery_secret,
    )
    .await
}

#[tauri::command]
pub async fn matrix_secret_storage_reset(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    let result = matrix_secret_storage_reset_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

#[tauri::command]
pub async fn matrix_room_key_transfer_status(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeRoomKeyTransferStatus, MatrixAuthCommandError> {
    let (flow, generation) = {
        let session = state.session.lock().await;
        let active = require_room_key_session(session.as_ref())?;
        (
            Arc::clone(&active.room_key_transfer),
            active.sync.session_generation(),
        )
    };
    let flow = flow.lock().await;
    Ok(live_room_keys::project_status(generation, &flow))
}

#[tauri::command]
pub async fn matrix_room_key_export(
    state: State<'_, MatrixAuthState>,
    mut passphrase: String,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let result = matrix_room_key_export_inner(&state, &passphrase).await;
    passphrase.zeroize();
    result
}

async fn matrix_room_key_export_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    live_room_keys::require_passphrase(passphrase)?;
    let (client, generation, flow) = {
        let session = state.session.lock().await;
        let active = require_room_key_session(session.as_ref())?;
        (
            active.client.clone(),
            active.sync.session_generation(),
            Arc::clone(&active.room_key_transfer),
        )
    };
    let result = live_room_keys::export(&client, generation, &flow, passphrase).await?;
    require_current_room_key_generation(state, generation).await?;
    Ok(result)
}

#[tauri::command]
pub async fn matrix_room_key_import_select(
    state: State<'_, MatrixAuthState>,
) -> Result<Option<NativeRoomKeyFileSelection>, MatrixAuthCommandError> {
    let generation = {
        let session = state.session.lock().await;
        require_room_key_session(session.as_ref())?
            .sync
            .session_generation()
    };
    let picked = live_room_keys::pick_import_file().await;
    let Some((path, file_label)) = picked else {
        return Ok(None);
    };

    let mut session = state.session.lock().await;
    let active = require_room_key_session_mut(session.as_mut())?;
    if active.sync.session_generation() != generation {
        return Err(stale_room_key_generation_error());
    }
    active.next_room_key_import_selection_id =
        active.next_room_key_import_selection_id.saturating_add(1);
    let selection_id = active.next_room_key_import_selection_id;
    active.selected_room_key_import = Some(SelectedRoomKeyImport {
        selection_id,
        path,
        file_label: file_label.clone(),
    });
    Ok(Some(NativeRoomKeyFileSelection {
        selection_id,
        file_label,
    }))
}

#[tauri::command]
pub async fn matrix_room_key_import(
    state: State<'_, MatrixAuthState>,
    selection_id: u64,
    mut passphrase: String,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let result = matrix_room_key_import_inner(&state, selection_id, &passphrase).await;
    passphrase.zeroize();
    result
}

async fn matrix_room_key_import_inner(
    state: &State<'_, MatrixAuthState>,
    selection_id: u64,
    passphrase: &str,
) -> Result<NativeRoomKeyTransferResult, MatrixAuthCommandError> {
    let (client, generation, flow, selected) = {
        let mut session = state.session.lock().await;
        let active = require_room_key_session_mut(session.as_mut())?;
        let selected = reserve_room_key_import_selection(
            &mut active.selected_room_key_import,
            selection_id,
            passphrase,
        )?;
        (
            active.client.clone(),
            active.sync.session_generation(),
            Arc::clone(&active.room_key_transfer),
            selected,
        )
    };
    let result = live_room_keys::import(&client, generation, &flow, &selected, passphrase).await;
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            restore_room_key_import_selection(state, generation, selected).await;
            return Err(error);
        }
    };
    require_current_room_key_generation(state, generation).await?;
    Ok(result)
}

fn reserve_room_key_import_selection(
    slot: &mut Option<SelectedRoomKeyImport>,
    selection_id: u64,
    passphrase: &str,
) -> Result<SelectedRoomKeyImport, MatrixAuthCommandError> {
    live_room_keys::require_passphrase(passphrase)?;
    if slot
        .as_ref()
        .is_none_or(|selected| selected.selection_id != selection_id)
    {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "Choose an encrypted room-key file before importing.",
            "v-crypto.5-import-selection-invalid",
        ));
    }
    slot.take().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "InvalidRequest",
            "Choose an encrypted room-key file before importing.",
            "v-crypto.5-import-selection-invalid",
        )
    })
}

async fn restore_room_key_import_selection(
    state: &State<'_, MatrixAuthState>,
    generation: u64,
    selected: SelectedRoomKeyImport,
) {
    let mut session = state.session.lock().await;
    let Some(active) = session.as_mut() else {
        return;
    };
    restore_reserved_room_key_import(
        generation,
        Some(active.sync.session_generation()),
        &mut active.selected_room_key_import,
        selected,
    );
}

fn restore_reserved_room_key_import(
    expected_generation: u64,
    current_generation: Option<u64>,
    slot: &mut Option<SelectedRoomKeyImport>,
    selected: SelectedRoomKeyImport,
) -> bool {
    if current_generation != Some(expected_generation) || slot.is_some() {
        return false;
    }
    *slot = Some(selected);
    true
}

async fn matrix_secret_storage_reset_inner(
    state: &State<'_, MatrixAuthState>,
    passphrase: &str,
) -> Result<NativeSecretStorageOperationResult, MatrixAuthCommandError> {
    require_secret_storage_input(passphrase, "v-crypto.4-reset-passphrase-empty")?;
    let session = state.session.lock().await;
    let active = require_secret_storage_session(session.as_ref())?;
    live_secret_storage::reset(&active.client, active.sync.session_generation(), passphrase).await
}

fn require_secret_storage_input(
    value: &str,
    diagnostic_id: &'static str,
) -> Result<(), MatrixAuthCommandError> {
    if value.is_empty() {
        Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A recovery key or passphrase is required.",
            diagnostic_id,
        ))
    } else {
        Ok(())
    }
}

fn require_recovery_secret(recovery_secret: &str) -> Result<(), MatrixAuthCommandError> {
    if recovery_secret.is_empty() {
        Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A recovery key or passphrase is required.",
            "v-crypto.3-recovery-secret-empty",
        ))
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn matrix_verification_list(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeVerificationInbox, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    Ok(active.verification.list().await)
}

#[tauri::command]
pub async fn matrix_verification_start(
    state: State<'_, MatrixAuthState>,
    device_id: Option<String>,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.start(&active.client, device_id).await
}

#[tauri::command]
pub async fn matrix_verification_accept(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.accept(&flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_begin_sas(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.begin_sas(&flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_confirm(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.confirm(&flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_mismatch(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.mismatch(&flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_cancel(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.cancel(&flow_id).await
}

#[tauri::command]
pub async fn matrix_verification_dismiss(
    state: State<'_, MatrixAuthState>,
    flow_id: String,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    active.verification.dismiss(&flow_id).await
}

#[tauri::command]
pub async fn matrix_device_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeDeviceSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_device_session(session.as_ref())?;
    live_device_snapshot(&active.client, active.sync.session_generation())
        .await
        .map_err(map_device_error)
}

#[tauri::command]
pub async fn matrix_device_rename(
    state: State<'_, MatrixAuthState>,
    device_id: String,
    display_name: String,
) -> Result<NativeDeviceSnapshot, MatrixAuthCommandError> {
    let display_name = display_name.trim();
    if display_name.is_empty() {
        return Err(map_device_error("v-crypto.7-device-rename-empty"));
    }
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    let device_id = matrix_sdk::ruma::OwnedDeviceId::from(device_id);
    active
        .client
        .rename_device(&device_id, display_name)
        .await
        .map_err(|_| map_device_error("v-crypto.7-device-rename-failed"))?;
    live_device_snapshot(&active.client, active.sync.session_generation())
        .await
        .map_err(map_device_error)
}

#[tauri::command]
pub async fn matrix_device_delete_start(
    state: State<'_, MatrixAuthState>,
    device_ids: Vec<String>,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    active.pending_device_deletion = None;
    let device_ids = validate_device_deletion(active, device_ids).await?;
    match active.client.delete_devices(&device_ids, None).await {
        Ok(_) => complete_device_deletion(active, &device_ids).await,
        Err(error) => {
            let info = error
                .as_uiaa_response()
                .ok_or_else(|| map_device_error("v-crypto.7-device-delete-start-failed"))?;
            retain_device_delete_challenge(active, device_ids, info).await
        }
    }
}

#[tauri::command]
pub async fn matrix_device_delete_password(
    state: State<'_, MatrixAuthState>,
    operation_id: u64,
    session_generation: u64,
    password: String,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let password = zeroize::Zeroizing::new(password);
    if password.is_empty() {
        return Err(map_device_error("v-crypto.7-device-delete-password-empty"));
    }
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    let pending = validate_pending_device_deletion(active, operation_id, session_generation)?;
    let user_id = active
        .client
        .user_id()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-user-missing"))?;
    let mut auth = uiaa::Password::new(
        uiaa::UserIdentifier::Matrix(uiaa::MatrixUserIdentifier::new(user_id.to_string())),
        password.to_string(),
    );
    auth.session = Some(pending.auth_session.clone());
    let device_ids = pending.device_ids.clone();
    match active
        .client
        .delete_devices(&device_ids, Some(uiaa::AuthData::Password(auth)))
        .await
    {
        Ok(_) => complete_device_deletion(active, &device_ids).await,
        Err(error) => {
            let info = error
                .as_uiaa_response()
                .ok_or_else(|| map_device_error("v-crypto.7-device-delete-password-failed"))?;
            let authentication_failed = !info.completed.contains(&uiaa::AuthType::Password);
            refresh_device_delete_challenge(active, info, authentication_failed).await
        }
    }
}

#[tauri::command]
pub async fn matrix_device_delete_cancel(
    state: State<'_, MatrixAuthState>,
    operation_id: u64,
    session_generation: u64,
) -> Result<(), MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_device_session_mut(session.as_mut())?;
    validate_pending_device_deletion(active, operation_id, session_generation)?;
    active.pending_device_deletion = None;
    Ok(())
}

#[tauri::command]
pub async fn matrix_room_list_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = session.as_ref().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.2-room-list-requires-session",
        )
    })?;
    snapshot_from_sync_owner(&active.sync)
        .await
        .map_err(map_room_list_error)
}

#[tauri::command]
pub async fn matrix_invites_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_space_parents_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeSpaceParentsSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_space_parents(&active.client, active.sync.session_generation())
        .await
        .map_err(map_space_parents_error)
}

#[tauri::command]
pub async fn matrix_space_hierarchy_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeSpaceHierarchySnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_space_hierarchy(&active.client, active.sync.session_generation(), &room_id)
        .await
        .map_err(map_space_hierarchy_error)
}

#[tauri::command]
pub async fn matrix_mdirect_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeMDirectSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_mdirect(&active.client, active.sync.session_generation())
        .await
        .map_err(map_mdirect_error)
}

#[tauri::command]
pub async fn matrix_mdirect_add(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    user_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    add_room_to_mdirect(&active.client, &room_id, &user_id)
        .await
        .map_err(map_mdirect_error)
}

#[tauri::command]
pub async fn matrix_mdirect_remove(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    remove_room_from_mdirect(&active.client, &room_id)
        .await
        .map_err(map_mdirect_error)
}

#[tauri::command]
pub async fn matrix_later_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_later(&active.client, active.sync.session_generation())
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_upsert(
    state: State<'_, MatrixAuthState>,
    item: SynaraLaterItem,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    upsert_later_item(&active.client, active.sync.session_generation(), item)
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_complete(
    state: State<'_, MatrixAuthState>,
    item_id: String,
    completed_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let completed_at = completed_at.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    });
    complete_later_item_live(
        &active.client,
        active.sync.session_generation(),
        item_id,
        completed_at,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_snooze(
    state: State<'_, MatrixAuthState>,
    item_id: String,
    due_ts: f64,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snooze_later_item_live(
        &active.client,
        active.sync.session_generation(),
        item_id,
        due_ts,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_clear_completed(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    clear_completed_later_live(&active.client, active.sync.session_generation())
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_mark_reminded(
    state: State<'_, MatrixAuthState>,
    item_id: String,
    reminded_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let reminded_at = reminded_at.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    });
    mark_later_reminded_live(
        &active.client,
        active.sync.session_generation(),
        item_id,
        reminded_at,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_room_notes(&active.client, active.sync.session_generation())
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_upsert(
    state: State<'_, MatrixAuthState>,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    upsert_room_note_item(&active.client, active.sync.session_generation(), item)
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_delete(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    delete_room_note_item_live(
        &active.client,
        active.sync.session_generation(),
        room_id,
        item_id,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_complete_todo(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    item_id: String,
    completed: bool,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    complete_room_todo_item_live(
        &active.client,
        active.sync.session_generation(),
        room_id,
        item_id,
        completed,
        now,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_move_todo(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    move_room_todo_item_live(
        &active.client,
        active.sync.session_generation(),
        room_id,
        item_id,
        direction,
        now,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_typing_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeTypingSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    Ok(active.typing.snapshot().await)
}

#[tauri::command]
pub async fn matrix_typing_set(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    typing: bool,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    set_typing_notice(&active.client, &room_id, typing)
        .await
        .map_err(map_typing_error)
}

#[tauri::command]
pub async fn matrix_invites_accept(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    let room = native_invite_room(active, &invite)?;
    room.join()
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-accept-failed"))?;
    if invite.is_direct {
        let sender_id = OwnedUserId::try_from(invite.sender_id.as_str())
            .map_err(|_| map_invite_error("v-rooms.1-invite-invalid-sender"))?;
        active
            .client
            .account()
            .mark_as_dm(room.room_id(), &[sender_id])
            .await
            .map_err(|_| map_invite_error("v-rooms.1-invite-direct-mark-failed"))?;
    }
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_invites_decline(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    native_invite_room(active, &invite)?
        .leave()
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-decline-failed"))?;
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_invites_report_spam(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    native_invite_room(active, &invite)?
        .report_room("Spam Invite".to_owned())
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-report-failed"))?;
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_invites_block_sender(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    let sender_id = OwnedUserId::try_from(invite.sender_id.as_str())
        .map_err(|_| map_invite_error("v-rooms.1-invite-invalid-sender"))?;
    active
        .client
        .account()
        .ignore_user(&sender_id)
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-block-failed"))?;
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_timeline_open(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineOpenRequest,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .open_at(app, &active.client, request)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_close(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineCloseRequest,
) -> Result<bool, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    Ok(active.timelines.close_view(request))
}

#[tauri::command]
pub async fn matrix_timeline_jump_latest(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineJumpLatestRequest,
) -> Result<NativeTimelineOpenReadback, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .jump_latest(app, &active.client, request)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeTimelineSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .snapshot(&active.client, &room_id)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_event_readback(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    event_id: String,
) -> Result<NativeTimelineEventReadback, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .event_readback(&active.client, &room_id, &event_id)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_paginate(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineViewPaginationRequest,
) -> Result<crate::matrix::timeline::TimelineViewSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .paginate(&active.client, request)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_set_read_state(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineReadStateRequest,
) -> Result<NativeTimelineReadStateReadback, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .set_read_state(&active.client, request)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_reaction_toggle(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .toggle_reaction(&active.client, &room_id, &event_id, &key)
        .await
        .map_err(map_reaction_error)
}

#[tauri::command]
pub async fn matrix_reaction_ensure(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .ensure_reaction(&active.client, &room_id, &event_id, &key)
        .await
        .map_err(map_reaction_error)
}

#[tauri::command]
pub async fn matrix_reaction_redact(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    target_event_id: String,
    reaction_event_id: String,
    key: String,
) -> Result<NativeReactionMutationResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .redact_reaction(
            &active.client,
            &room_id,
            &target_event_id,
            &reaction_event_id,
            &key,
        )
        .await
        .map_err(map_reaction_error)
}

#[tauri::command]
pub async fn matrix_composer_set_reply_draft(
    state: State<'_, MatrixAuthState>,
    request: NativeComposerSetReplyDraftRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-reply-draft-invalid-event-id")?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-reply-draft-room-not-found",
            )
        })?
    };

    let draft = load_reply_draft_preview(&room, &event_id, request.start_thread).await?;
    let room_id_string = room_id.to_string();
    {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active
            .composer_drafts
            .set(room_id_string.clone(), draft.clone());
    }

    Ok(reply_draft_readback(room_id_string, "set", Some(draft)))
}

#[tauri::command]
pub async fn matrix_composer_clear_reply_draft(
    state: State<'_, MatrixAuthState>,
    request: NativeComposerReplyDraftRoomRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let room_id_string = room_id.to_string();
    {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.composer_drafts.clear(&room_id_string);
    }
    Ok(reply_draft_readback(room_id_string, "cleared", None))
}

#[tauri::command]
pub async fn matrix_composer_get_reply_draft(
    state: State<'_, MatrixAuthState>,
    request: NativeComposerReplyDraftRoomRequest,
) -> Result<NativeComposerReplyDraftReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let room_id_string = room_id.to_string();
    let draft = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.composer_drafts.get(&room_id_string).cloned()
    };
    Ok(reply_draft_readback(
        room_id_string,
        if draft.is_some() { "set" } else { "empty" },
        draft,
    ))
}

#[tauri::command]
pub async fn matrix_timeline_edit_text(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineEditTextRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id = parse_required_event_id(&request.event_id, "v-timeline-edit-invalid-event-id")?;
    let body = request.body.trim();
    if body.is_empty() {
        return Err(map_timeline_action_error("v-timeline-edit-empty-body"));
    }
    let formatted_body = normalize_formatted_body(body, request.formatted_body.as_deref())?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-edit-room-not-found",
            )
        })?
    };

    let new_content = match formatted_body {
        Some(html) => RoomMessageEventContentWithoutRelation::text_html(body.to_owned(), html),
        None => RoomMessageEventContentWithoutRelation::text_plain(body.to_owned()),
    };
    let edit_content = room
        .make_edit_event(&event_id, EditedContent::RoomMessage(new_content))
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-edit-prepare-failed"))?;
    let response = room
        .send(edit_content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-edit-send-failed"))?;

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::EditText,
        room_id: room_id.to_string(),
        event_id: response.response.event_id.to_string(),
        status: "sent",
    })
}

#[tauri::command]
pub async fn matrix_timeline_redact(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineRedactRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-redact-invalid-event-id")?;
    let reason = request
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-redact-room-not-found",
            )
        })?
    };

    room.redact(&event_id, reason, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-redact-failed"))?;

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::Redact,
        room_id: room_id.to_string(),
        event_id: event_id.to_string(),
        status: "redacted",
    })
}

#[tauri::command]
pub async fn matrix_timeline_forward_text(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineForwardTextRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let source_room_id = parse_send_room_id(&request.source_room_id)?;
    let target_room_id = parse_send_room_id(&request.target_room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-forward-invalid-event-id")?;

    let (source_room, target_room) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let source_room = active.client.get_room(&source_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix source room is not available.",
                "v-timeline-forward-source-room-not-found",
            )
        })?;
        let target_room = active.client.get_room(&target_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix target room is not available.",
                "v-timeline-forward-target-room-not-found",
            )
        })?;
        (source_room, target_room)
    };

    let (sender_label, body) = load_forwardable_text(&source_room, &event_id).await?;
    let forwarded_body = format_forwarded_plain_body(&sender_label, &body, request.as_quote);
    let content = message_content(forwarded_body, None, None, None, false, None, None)?;
    let event_id = send_message_to_room(&target_room, content, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-send-failed"))?;

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::ForwardText,
        room_id: target_room_id.to_string(),
        event_id,
        status: "sent",
    })
}

#[tauri::command]
pub async fn matrix_timeline_forward_media(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineForwardMediaRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let source_room_id = parse_send_room_id(&request.source_room_id)?;
    let target_room_id = parse_send_room_id(&request.target_room_id)?;
    let event_id = parse_required_event_id(
        &request.event_id,
        "v-timeline-forward-media-invalid-event-id",
    )?;

    let (source_room, target_room) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let source_room = active.client.get_room(&source_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix source room is not available.",
                "v-timeline-forward-media-source-room-not-found",
            )
        })?;
        let target_room = active.client.get_room(&target_room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix target room is not available.",
                "v-timeline-forward-media-target-room-not-found",
            )
        })?;
        (source_room, target_room)
    };

    let content = load_forwardable_media(&source_room, &event_id).await?;
    let event_id = target_room
        .send(content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-media-send-failed"))?
        .response
        .event_id
        .to_string();

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::ForwardMedia,
        room_id: target_room_id.to_string(),
        event_id,
        status: "sent",
    })
}

#[tauri::command]
pub async fn matrix_timeline_report(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineReportRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-report-invalid-event-id")?;
    let reason = request
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-report-room-not-found",
            )
        })?
    };

    room.report_content(event_id.clone(), reason)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-report-failed"))?;

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::Report,
        room_id: room_id.to_string(),
        event_id: event_id.to_string(),
        status: "reported",
    })
}

#[tauri::command]
pub async fn matrix_timeline_pin(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePinRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    pin_or_unpin_event(state, request, true).await
}

#[tauri::command]
pub async fn matrix_timeline_unpin(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePinRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    pin_or_unpin_event(state, request, false).await
}

#[tauri::command]
pub async fn matrix_timeline_poll_vote(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePollVoteRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id =
        parse_required_event_id(&request.event_id, "v-timeline-poll-vote-invalid-event-id")?;
    let answer_ids = request
        .answer_ids
        .into_iter()
        .map(|answer| answer.trim().to_owned())
        .filter(|answer| !answer.is_empty())
        .collect::<Vec<_>>();

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-poll-vote-room-not-found",
            )
        })?
    };

    let content = UnstablePollResponseEventContent::new(answer_ids, event_id.clone());
    let sent_event_id = room
        .send(content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-poll-vote-send-failed"))?
        .response
        .event_id
        .to_string();

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::PollVote,
        room_id: room_id.to_string(),
        event_id: sent_event_id,
        status: "voted",
    })
}

#[tauri::command]
pub async fn matrix_timeline_call_decline(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelineCallDeclineRequest,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id = parse_required_event_id(
        &request.event_id,
        "v-timeline-call-decline-invalid-event-id",
    )?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-timeline-call-decline-room-not-found",
            )
        })?
    };

    let content = room
        .make_decline_call_event(&event_id)
        .await
        .map_err(|error| match error {
            matrix_sdk::room::calls::CallError::DeclineOwnCall => MatrixAuthCommandError::new(
                "InvalidRequest",
                "A call started by this session cannot be declined.",
                "v-timeline-call-decline-own-call",
            ),
            matrix_sdk::room::calls::CallError::BadEventType => MatrixAuthCommandError::new(
                "InvalidRequest",
                "Only m.rtc.notification events can be declined.",
                "v-timeline-call-decline-bad-event-type",
            ),
            _ => map_timeline_action_error("v-timeline-call-decline-prepare-failed"),
        })?;
    let sent_event_id = room
        .send(content)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-call-decline-send-failed"))?
        .response
        .event_id
        .to_string();

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: NativeTimelineActionKind::CallDecline,
        room_id: room_id.to_string(),
        event_id: sent_event_id,
        status: "declined",
    })
}

async fn pin_or_unpin_event(
    state: State<'_, MatrixAuthState>,
    request: NativeTimelinePinRequest,
    pin: bool,
) -> Result<NativeTimelineActionReadback, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&request.room_id)?;
    let event_id = parse_required_event_id(
        &request.event_id,
        if pin {
            "v-timeline-pin-invalid-event-id"
        } else {
            "v-timeline-unpin-invalid-event-id"
        },
    )?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                if pin {
                    "v-timeline-pin-room-not-found"
                } else {
                    "v-timeline-unpin-room-not-found"
                },
            )
        })?
    };

    let changed = if pin {
        room.pin_event(&event_id)
            .await
            .map_err(|_| map_timeline_action_error("v-timeline-pin-failed"))?
    } else {
        room.unpin_event(&event_id)
            .await
            .map_err(|_| map_timeline_action_error("v-timeline-unpin-failed"))?
    };

    Ok(NativeTimelineActionReadback {
        schema_version: NATIVE_TIMELINE_ACTION_SCHEMA_VERSION,
        action: if pin {
            NativeTimelineActionKind::Pin
        } else {
            NativeTimelineActionKind::Unpin
        },
        room_id: room_id.to_string(),
        event_id: event_id.to_string(),
        status: if changed {
            if pin {
                "pinned"
            } else {
                "unpinned"
            }
        } else if pin {
            "already_pinned"
        } else {
            "already_unpinned"
        },
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_send_text(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: Option<bool>,
    reply_to: Option<String>,
    // Thread root (`m.thread`). With reply_to → Thread::reply (is_falling_back false).
    thread_root: Option<String>,
    txn_id: Option<String>,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let reply_to = parse_reply_event_id(reply_to)?;
    let thread_root = parse_thread_root_event_id(thread_root)?;
    let txn_id = parse_transaction_id(txn_id)?;
    let content = message_content(
        body.clone(),
        msg_type,
        formatted_body,
        mention_user_ids,
        mention_room.unwrap_or(false),
        reply_to,
        thread_root,
    )?;
    let (room, session_generation, local_txn_id) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let room = active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "d0.4-send-room-not-found",
            )
        })?;
        let session_generation = active.sends.session_generation();
        let item = active
            .sends
            .enqueue_text(room_id.to_string(), body.clone())
            .map_err(|error| map_send_error(error.diagnostic_id()))?;
        (room, session_generation, item.local_txn_id.clone())
    };

    let send_result = send_message_to_room(&room, content, txn_id).await;
    let mut session = state.session.lock().await;
    if let Some(active) = session.as_mut() {
        if active.sends.session_generation() == session_generation {
            if send_result.is_ok() {
                let _ = active.sends.mark_sent(&local_txn_id);
            } else {
                let _ = active
                    .sends
                    .mark_failed(&local_txn_id, "d0.4-send-sdk-failed");
            }
        }
    }

    let event_id = send_result.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix message could not be sent.",
            "d0.4-send-sdk-failed",
        )
    })?;
    Ok(MatrixSendTextResult {
        room_id: room_id.to_string(),
        event_id,
        local_txn_id,
        status: "sent",
    })
}

/// V-SEND.1 sole composer attachment upload+send owner. Bytes cross IPC once;
/// encrypted rooms are encrypted by the managed SDK (no JS dual-encrypt).
/// V-SEND.5 extends the same command with optional `thread_root` so native
/// sessions can start / continue threads without JS relation ownership.
#[tauri::command]
pub async fn matrix_send_attachment(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    filename: String,
    mime_type: String,
    bytes: Vec<u8>,
    reply_to: Option<String>,
    thread_root: Option<String>,
) -> Result<MatrixSendAttachmentResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let reply_to = parse_reply_event_id(reply_to)?;
    let thread_root = parse_thread_root_event_id(thread_root)?;
    let filename = validate_attachment_filename(&filename)?;
    let mime_type = validate_attachment_mime(&mime_type)?;
    if bytes.is_empty() {
        return Err(map_attachment_error("v-send.1-attachment-empty"));
    }
    if bytes.len() > MAX_ATTACHMENT_IPC_BYTES {
        return Err(map_attachment_error("v-send.1-attachment-too-large"));
    }
    let size_bytes = bytes.len() as u64;
    let kind = attachment_kind_for_mime(&mime_type);
    let media_handle_id = format!("native-staged:{filename}");

    let (room, session_generation, local_txn_id) = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        let room = active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.1-attachment-room-not-found",
            )
        })?;
        let session_generation = active.attachments.session_generation();
        let item = active
            .attachments
            .enqueue(AttachmentEnqueue {
                room_id: room_id.to_string(),
                kind,
                media_handle_id,
                file_name: Some(filename.clone()),
                caption: None,
                mime_type: Some(mime_type.to_string()),
                size_bytes: Some(size_bytes),
            })
            .map_err(|error| map_attachment_error(error.diagnostic_id()))?;
        (room, session_generation, item.local_txn_id.clone())
    };

    let send_result =
        send_attachment_to_room(&room, &filename, &mime_type, bytes, reply_to, thread_root).await;

    let mut session = state.session.lock().await;
    if let Some(active) = session.as_mut() {
        if active.attachments.session_generation() == session_generation {
            if send_result.is_ok() {
                let _ = active.attachments.mark_sent(&local_txn_id);
            } else {
                let _ = active
                    .attachments
                    .mark_failed(&local_txn_id, "v-send.1-attachment-sdk-failed");
            }
        }
    }

    let event_id = send_result.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix attachment could not be sent.",
            "v-send.1-attachment-sdk-failed",
        )
    })?;
    Ok(MatrixSendAttachmentResult {
        room_id: room_id.to_string(),
        event_id,
        local_txn_id,
        status: "sent",
    })
}

/// V-SEND sticker residual — sole `m.sticker` owner for native sessions.
/// Media is already on the homeserver as an MXC (image-pack sticker); this
/// command does not re-upload bytes. Optional info fields preserve dimensions
/// when the product already knows them; empty info is valid.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
pub async fn matrix_send_sticker(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    body: String,
    mxc: String,
    width: Option<u64>,
    height: Option<u64>,
    mimetype: Option<String>,
    size: Option<u64>,
    reply_to: Option<String>,
    thread_root: Option<String>,
) -> Result<MatrixSendStickerResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let reply_to = parse_reply_event_id(reply_to)?;
    let thread_root = parse_thread_root_event_id(thread_root)?;
    let content = sticker_content(
        body,
        mxc,
        width,
        height,
        mimetype,
        size,
        reply_to,
        thread_root,
    )?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send-sticker-room-not-found",
            )
        })?
    };

    let response = room.send(content).await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix sticker could not be sent.",
            "v-send-sticker-sdk-failed",
        )
    })?;
    Ok(MatrixSendStickerResult {
        room_id: room_id.to_string(),
        event_id: response.response.event_id.to_string(),
        status: "sent",
    })
}

/// V-SEND.3 sole poll-start owner (composer board + `/poll` command).
#[tauri::command]
pub async fn matrix_send_poll(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    question: String,
    answers: Vec<String>,
    max_selections: u32,
) -> Result<MatrixSendPollResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let normalized = normalize_poll(&question, &answers, max_selections)
        .map_err(|error| map_poll_error(error.diagnostic_id()))?;
    let content =
        poll_start_content(&normalized).map_err(|error| map_poll_error(error.diagnostic_id()))?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.3-poll-room-not-found",
            )
        })?
    };

    let response = room.send(content).await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix poll could not be sent.",
            "v-send.3-poll-sdk-failed",
        )
    })?;

    Ok(MatrixSendPollResult {
        room_id: room_id.to_string(),
        event_id: response.response.event_id.to_string(),
        status: "sent",
    })
}

/// V-SEND.3 sole poll-response (vote) owner.
#[tauri::command]
pub async fn matrix_poll_respond(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    poll_event_id: String,
    answer_ids: Vec<String>,
) -> Result<MatrixPollRespondResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let content = poll_response_content(&poll_event_id, &answer_ids)
        .map_err(|error| map_poll_error(error.diagnostic_id()))?;

    let room = {
        let mut session = state.session.lock().await;
        let active = require_send_session_mut(session.as_mut())?;
        active.client.get_room(&room_id).ok_or_else(|| {
            MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix room is not available.",
                "v-send.3-poll-room-not-found",
            )
        })?
    };

    let response = room.send(content).await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix poll response could not be sent.",
            "v-send.3-poll-response-sdk-failed",
        )
    })?;

    Ok(MatrixPollRespondResult {
        room_id: room_id.to_string(),
        poll_event_id,
        event_id: response.response.event_id.to_string(),
        status: "sent",
    })
}

fn map_poll_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-send.3-poll-invalid-question"
        | "v-send.3-poll-invalid-answers"
        | "v-send.3-poll-invalid-max-selections"
        | "v-send.3-poll-invalid-event-id"
        | "v-send.3-poll-invalid-answer-ids" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix poll request is invalid.",
            diagnostic_id,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix poll operation failed.",
            diagnostic_id,
        ),
    }
}

#[tauri::command]
pub async fn matrix_logout(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let Some(active) = session.as_ref() else {
        return Ok(MatrixSessionSnapshot::LoggedOut);
    };

    active.client.matrix_auth().logout().await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The Matrix homeserver rejected logout.",
            "d0.1-remote-logout-failed",
        )
    })?;
    active
        .sync
        .stop()
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;

    let identity = account_identity(&active.identity)?;
    let clear_result = clear_session_material(&KeyringSessionMaterialVault::new(), &identity)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-clear-failed"));
    let remove_result = remove_active_identity(&app_data_root(&app)?);
    *session = None;
    clear_result?;
    remove_result?;
    Ok(MatrixSessionSnapshot::LoggedOut)
}

#[tauri::command]
pub async fn matrix_restore_session(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    if let Some(active) = session.as_ref() {
        return Ok(active.identity.clone());
    }

    let app_data_root = app_data_root(&app)?;
    let identity = read_active_identity(&app_data_root)?;
    let account = account_identity(&identity)?;
    let client = build_client(&app_data_root, account.clone()).await?;
    let restored =
        restore_session_from_vault(&client, &account, &KeyringSessionMaterialVault::new())
            .await
            .map_err(|_| {
                MatrixAuthCommandError::new(
                    "Forbidden",
                    "No restorable native Matrix session is available.",
                    "d0.1-session-restore-failed",
                )
            })?;

    if restored.meta.device_id != identity.device_id {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "The persisted native Matrix session identity is inconsistent.",
            "d0.1-restored-device-mismatch",
        ));
    }

    ensure_crypto_ready(&client).await?;
    let session_generation = state.next_generation();
    let verification = NativeVerificationOwner::new(&client, session_generation);
    let devices = NativeDeviceOwner::start(&client, app.clone(), session_generation)
        .await
        .map_err(map_device_error)?;
    let typing = NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?;
    let sync = start_sync_owner(&client, session_generation).await?;
    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync,
        invite_avatars: InviteAvatarHandles::new(session_generation),
        timelines: NativeTimelineRegistry::new(session_generation),
        composer_drafts: ComposerDraftRegistry::new(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification,
        _devices: devices,
        typing,
        pending_device_deletion: None,
        next_device_delete_operation_id: 0,
        pending_cross_signing_auth_session: None,
        room_key_transfer: Arc::new(Mutex::new(RoomKeyTransferFlow::new(session_generation))),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok(identity)
}

impl MatrixAuthState {
    fn next_generation(&self) -> u64 {
        self.next_session_generation
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    fn current_generation(&self) -> u64 {
        self.next_session_generation.load(Ordering::Relaxed)
    }
}

async fn start_sync_owner(
    client: &Client,
    session_generation: u64,
) -> Result<SyncServiceOwner, MatrixAuthCommandError> {
    let owner = build_sync_service(client, session_generation, SyncServiceConfig::default())
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;
    owner
        .start()
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;
    Ok(owner)
}

async fn ensure_crypto_ready(client: &Client) -> Result<(), MatrixAuthCommandError> {
    if client.encryption().cross_signing_status().await.is_none() {
        return Err(MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix encryption is unavailable.",
            "d0.5-crypto-machine-unavailable",
        ));
    }
    Ok(())
}

fn crypto_status(
    session_generation: u64,
    cross_signing: Option<CrossSigningStatus>,
) -> MatrixCryptoStatus {
    MatrixCryptoStatus {
        session_generation,
        encryption_enabled: cross_signing.is_some(),
        cross_signing_state: cross_signing_state(cross_signing.as_ref()),
    }
}

fn cross_signing_state(status: Option<&CrossSigningStatus>) -> MatrixCrossSigningState {
    match status {
        None => MatrixCrossSigningState::Unavailable,
        Some(status) if status.is_complete() => MatrixCrossSigningState::Ready,
        Some(status) if status.has_master || status.has_self_signing || status.has_user_signing => {
            MatrixCrossSigningState::Partial
        }
        Some(_) => MatrixCrossSigningState::NotSetUp,
    }
}

async fn live_cross_signing_status(
    active: &ManagedMatrixSession,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    let encryption = active.client.encryption();
    let private_status = encryption.cross_signing_status().await;
    let Some(user_id) = active.client.user_id() else {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-user-missing",
        ));
    };
    let own_identity = encryption
        .request_user_identity(user_id)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "Native cross-signing status is unavailable.",
                "v-crypto.2-cross-signing-identity-query-failed",
            )
        })?;

    Ok(project_status(
        active.sync.session_generation(),
        private_status.as_ref(),
        own_identity.is_some(),
        own_identity
            .as_ref()
            .is_some_and(|identity| identity.is_verified()),
    ))
}

async fn validate_device_deletion(
    active: &ManagedMatrixSession,
    device_ids: Vec<String>,
) -> Result<Vec<matrix_sdk::ruma::OwnedDeviceId>, MatrixAuthCommandError> {
    if device_ids.is_empty() {
        return Err(map_device_error("v-crypto.7-device-delete-selection-empty"));
    }
    let snapshot = live_device_snapshot(&active.client, active.sync.session_generation())
        .await
        .map_err(map_device_error)?;
    let current = snapshot
        .devices
        .iter()
        .find(|device| device.is_current)
        .map(|device| device.device_id.as_str())
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-current-missing"))?;
    let mut unique = std::collections::BTreeSet::new();
    for device_id in device_ids {
        if device_id.is_empty() || device_id == current || !snapshot.contains(&device_id) {
            return Err(map_device_error(
                "v-crypto.7-device-delete-selection-invalid",
            ));
        }
        unique.insert(matrix_sdk::ruma::OwnedDeviceId::from(device_id));
    }
    Ok(unique.into_iter().collect())
}

fn validate_pending_device_deletion(
    active: &ManagedMatrixSession,
    operation_id: u64,
    session_generation: u64,
) -> Result<&PendingDeviceDeletion, MatrixAuthCommandError> {
    if active.sync.session_generation() != session_generation {
        return Err(map_device_error(
            "v-crypto.7-device-delete-stale-generation",
        ));
    }
    let pending = active
        .pending_device_deletion
        .as_ref()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-not-pending"))?;
    if pending.session_generation != session_generation {
        return Err(map_device_error(
            "v-crypto.7-device-delete-stale-generation",
        ));
    }
    if pending.operation_id != operation_id {
        return Err(map_device_error(
            "v-crypto.7-device-delete-operation-mismatch",
        ));
    }
    Ok(pending)
}

async fn retain_device_delete_challenge(
    active: &mut ManagedMatrixSession,
    device_ids: Vec<matrix_sdk::ruma::OwnedDeviceId>,
    info: &uiaa::UiaaInfo,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let operation_id = active
        .next_device_delete_operation_id
        .checked_add(1)
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-operation-overflow"))?;
    active.next_device_delete_operation_id = operation_id;
    install_device_delete_challenge(active, operation_id, device_ids, info, false).await
}

async fn refresh_device_delete_challenge(
    active: &mut ManagedMatrixSession,
    info: &uiaa::UiaaInfo,
    authentication_failed: bool,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let pending = active
        .pending_device_deletion
        .take()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-not-pending"))?;
    install_device_delete_challenge(
        active,
        pending.operation_id,
        pending.device_ids,
        info,
        authentication_failed,
    )
    .await
}

async fn install_device_delete_challenge(
    active: &mut ManagedMatrixSession,
    operation_id: u64,
    device_ids: Vec<matrix_sdk::ruma::OwnedDeviceId>,
    info: &uiaa::UiaaInfo,
    authentication_failed: bool,
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let auth_session = info
        .session
        .clone()
        .ok_or_else(|| map_device_error("v-crypto.7-device-delete-auth-session-missing"))?;
    let available = supported_delete_authentication(info);
    let authentication = if available
        .contains(&crate::matrix::devices::NativeDeviceDeleteAuthentication::Password)
    {
        crate::matrix::devices::NativeDeviceDeleteAuthentication::Password
    } else {
        return Err(map_device_error(
            "v-crypto.7-device-delete-auth-unsupported",
        ));
    };
    active.pending_device_deletion = Some(PendingDeviceDeletion {
        operation_id,
        session_generation: active.sync.session_generation(),
        device_ids,
        auth_session,
    });
    Ok(NativeDeviceDeleteResult::AuthenticationRequired {
        challenge: NativeDeviceDeleteChallenge {
            operation_id,
            session_generation: active.sync.session_generation(),
            authentication,
            authentication_failed,
        },
    })
}

async fn complete_device_deletion(
    active: &mut ManagedMatrixSession,
    deleted: &[matrix_sdk::ruma::OwnedDeviceId],
) -> Result<NativeDeviceDeleteResult, MatrixAuthCommandError> {
    let snapshot = live_device_snapshot(&active.client, active.sync.session_generation())
        .await
        .map_err(map_device_error)?;
    if deleted
        .iter()
        .any(|device_id| snapshot.contains(device_id.as_str()))
    {
        return Err(map_device_error(
            "v-crypto.7-device-delete-readback-incomplete",
        ));
    }
    active.pending_device_deletion = None;
    Ok(NativeDeviceDeleteResult::Complete { snapshot })
}

async fn cross_signing_setup_complete(
    active: &mut ManagedMatrixSession,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    active.pending_cross_signing_auth_session = None;
    let status = live_cross_signing_status(active).await?;
    if status.bootstrap == crate::matrix::cross_signing::live::NativeCrossSigningBootstrap::Needed {
        return Err(cross_signing_setup_error(
            "v-crypto.2-cross-signing-bootstrap-incomplete",
        ));
    }
    Ok(NativeCrossSigningSetupResult {
        outcome: NativeCrossSigningSetupOutcome::Complete,
        status,
    })
}

fn cross_signing_setup_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native cross-signing setup could not be completed.",
        diagnostic_id,
    )
}

fn map_sync_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix sync is unavailable.",
        diagnostic_id,
    )
}

fn map_room_list_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room list is unavailable.",
        diagnostic_id,
    )
}

fn map_space_parents_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix space parent map is unavailable.",
        diagnostic_id,
    )
}

fn map_space_hierarchy_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix space hierarchy is unavailable.",
        diagnostic_id,
    )
}

fn map_mdirect_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.5-mdirect-invalid-room" | "v-rooms.5-mdirect-invalid-user" => (
            "InvalidRequest",
            "The native Matrix direct-room request is invalid.",
        ),
        _ => (
            "Unknown",
            "The native Matrix direct-room map is unavailable.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

fn map_later_notes_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-timeline-later-invalid-item" | "v-timeline-room-notes-invalid-item" => (
            "InvalidRequest",
            "The native Matrix later/notes request is invalid.",
        ),
        _ => (
            "Unknown",
            "The native Matrix later/notes account data is unavailable.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

fn map_typing_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.4-typing-invalid-room" => (
            "InvalidRequest",
            "The native Matrix typing request is invalid.",
        ),
        "v-rooms.4-typing-room-missing" | "v-rooms.4-typing-room-not-joined" => {
            ("NotFound", "The native Matrix typing room was not found.")
        }
        "v-rooms.4-typing-owner-user-missing" => {
            ("Forbidden", "No native Matrix session is active.")
        }
        _ => ("Unknown", "The native Matrix typing notice is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

fn map_invite_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.1-invite-invalid-room" | "v-rooms.1-invite-invalid-sender" => (
            "InvalidRequest",
            "The native Matrix invite request is invalid.",
        ),
        "v-rooms.1-invite-not-found" | "v-rooms.1-invite-member-missing" => (
            "NotFound",
            "The native Matrix invitation is no longer available.",
        ),
        _ => (
            "Unknown",
            "The native Matrix invite operation could not be completed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

async fn native_invite_target(
    active: &mut ManagedMatrixSession,
    room_id: &str,
) -> Result<NativeInvite, MatrixAuthCommandError> {
    let normalized_room_id = room_id.trim();
    if normalized_room_id.is_empty() {
        return Err(map_invite_error("v-rooms.1-invite-invalid-room"));
    }
    let snapshot = snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)?;
    snapshot
        .invites
        .into_iter()
        .find(|invite| invite.room_id == normalized_room_id)
        .ok_or_else(|| map_invite_error("v-rooms.1-invite-not-found"))
}

fn native_invite_room(
    active: &ManagedMatrixSession,
    invite: &NativeInvite,
) -> Result<Room, MatrixAuthCommandError> {
    let room_id = OwnedRoomId::try_from(invite.room_id.as_str())
        .map_err(|_| map_invite_error("v-rooms.1-invite-invalid-room"))?;
    active
        .client
        .get_room(&room_id)
        .ok_or_else(|| map_invite_error("v-rooms.1-invite-not-found"))
}

fn map_device_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-crypto.7-device-rename-empty"
        | "v-crypto.7-device-delete-selection-empty"
        | "v-crypto.7-device-delete-selection-invalid"
        | "v-crypto.7-device-delete-not-pending"
        | "v-crypto.7-device-delete-operation-mismatch" => (
            "InvalidRequest",
            "The native Matrix device request is invalid.",
        ),
        "v-crypto.7-device-delete-stale-generation" => (
            "StaleSessionGeneration",
            "The native Matrix session changed during device logout.",
        ),
        "v-crypto.7-device-delete-auth-unsupported" => (
            "Forbidden",
            "The homeserver requires an unsupported authentication step for device logout.",
        ),
        _ => ("Unknown", "Native Matrix device management is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

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

fn map_timeline_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" => (
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
        ),
        "d0.3-timeline-room-not-found" | "d0.3-timeline-not-open" => {
            ("NotFound", "The native Matrix timeline is not available.")
        }
        _ => ("Unknown", "The native Matrix timeline is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

fn map_reaction_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let code = if diagnostic_id.contains("invalid") {
        "InvalidRequest"
    } else {
        "Unknown"
    };
    MatrixAuthCommandError::new(
        code,
        "The native Matrix reaction operation could not be completed.",
        diagnostic_id,
    )
}

fn require_send_session_mut(
    session: Option<&mut ManagedMatrixSession>,
) -> Result<&mut ManagedMatrixSession, MatrixAuthCommandError> {
    session.ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        )
    })
}

fn map_send_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "The native Matrix send request is invalid.",
        diagnostic_id,
    )
}

fn parse_send_room_id(room_id: &str) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .parse()
        .map_err(|_| map_send_error("d0.4-send-invalid-room-id"))
}

fn parse_reply_event_id(
    reply_to: Option<String>,
) -> Result<Option<OwnedEventId>, MatrixAuthCommandError> {
    reply_to
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| map_send_error("d0.4-send-invalid-reply-event-id"))
        })
        .transpose()
}

fn parse_required_event_id(
    event_id: &str,
    diagnostic_id: &'static str,
) -> Result<OwnedEventId, MatrixAuthCommandError> {
    event_id
        .trim()
        .parse()
        .map_err(|_| map_timeline_action_error(diagnostic_id))
}

fn map_timeline_action_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "InvalidRequest",
        "The native Matrix timeline action request is invalid.",
        diagnostic_id,
    )
}

async fn load_forwardable_text(
    room: &Room,
    event_id: &EventId,
) -> Result<(String, String), MatrixAuthCommandError> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-event-unavailable"))?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| map_timeline_action_error("v-timeline-forward-event-decode-failed"))?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message
                .as_original()
                .ok_or_else(|| map_timeline_action_error("v-timeline-forward-event-redacted"))?;
            Ok((
                original.sender.to_string(),
                original.content.body().to_owned(),
            ))
        }
        _ => Err(map_timeline_action_error(
            "v-timeline-forward-unsupported-event",
        )),
    }
}

async fn load_forwardable_media(
    room: &Room,
    event_id: &EventId,
) -> Result<AnyMessageLikeEventContent, MatrixAuthCommandError> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-forward-media-event-unavailable"))?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| map_timeline_action_error("v-timeline-forward-media-event-decode-failed"))?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message.as_original().ok_or_else(|| {
                map_timeline_action_error("v-timeline-forward-media-event-redacted")
            })?;
            let sender = original.sender.to_string();
            let mut msgtype = original.content.msgtype.clone();
            match &mut msgtype {
                MessageType::Image(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::File(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::Audio(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                MessageType::Video(content) => {
                    content.body = format_forwarded_media_body(&sender, &content.body);
                }
                _ => {
                    return Err(map_timeline_action_error(
                        "v-timeline-forward-media-unsupported-event",
                    ));
                }
            }
            Ok(AnyMessageLikeEventContent::RoomMessage(
                RoomMessageEventContent::new(msgtype),
            ))
        }
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::Sticker(sticker)) => {
            let original = sticker.as_original().ok_or_else(|| {
                map_timeline_action_error("v-timeline-forward-media-event-redacted")
            })?;
            let sender = original.sender.to_string();
            Ok(AnyMessageLikeEventContent::Sticker(
                StickerEventContent::with_source(
                    format_forwarded_media_body(&sender, &original.content.body),
                    original.content.info.clone(),
                    original.content.source.clone(),
                ),
            ))
        }
        _ => Err(map_timeline_action_error(
            "v-timeline-forward-media-unsupported-event",
        )),
    }
}

async fn load_reply_draft_preview(
    room: &Room,
    event_id: &EventId,
    start_thread: bool,
) -> Result<NativeComposerReplyDraft, MatrixAuthCommandError> {
    let timeline_event = room
        .load_or_fetch_event(event_id, None)
        .await
        .map_err(|_| map_timeline_action_error("v-timeline-reply-draft-event-unavailable"))?;
    let sync_event = timeline_event
        .raw()
        .deserialize()
        .map_err(|_| map_timeline_action_error("v-timeline-reply-draft-event-decode-failed"))?;
    match sync_event {
        AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) => {
            let original = message.as_original().ok_or_else(|| {
                map_timeline_action_error("v-timeline-reply-draft-event-redacted")
            })?;
            let body = original.content.body().to_owned();
            let formatted_body = match original.content.msgtype {
                MessageType::Text(ref content) => content.formatted.as_ref(),
                MessageType::Notice(ref content) => content.formatted.as_ref(),
                MessageType::Emote(ref content) => content.formatted.as_ref(),
                _ => None,
            }
            .filter(|formatted| formatted.format == MessageFormat::Html)
            .map(|formatted| formatted.body.trim().to_owned())
            .filter(|html| !html.is_empty() && html != body.trim());
            let existing_thread_root = match &original.content.relates_to {
                Some(Relation::Thread(thread)) => Some(thread.event_id.to_string()),
                _ => None,
            };
            let thread_root_event_id = if start_thread {
                Some(event_id.to_string())
            } else {
                existing_thread_root
            };
            Ok(NativeComposerReplyDraft {
                event_id: event_id.to_string(),
                sender_id: original.sender.to_string(),
                body,
                formatted_body,
                thread_root_event_id,
            })
        }
        _ => Err(map_timeline_action_error(
            "v-timeline-reply-draft-unsupported-event",
        )),
    }
}

fn parse_thread_root_event_id(
    thread_root: Option<String>,
) -> Result<Option<OwnedEventId>, MatrixAuthCommandError> {
    thread_root
        .map(|event_id| {
            event_id
                .parse()
                .map_err(|_| map_send_error("v-send.5-invalid-thread-root-event-id"))
        })
        .transpose()
}

fn parse_transaction_id(
    txn_id: Option<String>,
) -> Result<Option<OwnedTransactionId>, MatrixAuthCommandError> {
    txn_id
        .map(|txn_id| {
            if txn_id.is_empty() || txn_id.len() > 255 {
                return Err(map_send_error("d0.4-send-invalid-transaction-id"));
            }
            Ok(OwnedTransactionId::from(txn_id))
        })
        .transpose()
}

fn normalize_formatted_body(
    body: &str,
    formatted_body: Option<&str>,
) -> Result<Option<String>, MatrixAuthCommandError> {
    let Some(html) = formatted_body
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    if html.len() > 65_536 {
        return Err(map_send_error("d0.4-send-formatted-body-too-large"));
    }
    if !should_attach_formatted_body(body, Some(html)) {
        return Ok(None);
    }
    Ok(Some(html.to_owned()))
}

/// Build validated `m.sticker` content for the native sticker owner.
///
/// Relation rules match text/attachment (V-SEND.5):
/// - `thread_root` + `reply_to` → `m.thread` with genuine in-thread reply
/// - `thread_root` only → `m.thread` without in-reply fallback
/// - `reply_to` only → classic `m.in_reply_to` reply
#[allow(clippy::too_many_arguments)]
pub(crate) fn sticker_content(
    body: String,
    mxc: String,
    width: Option<u64>,
    height: Option<u64>,
    mimetype: Option<String>,
    size: Option<u64>,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> Result<StickerEventContent, MatrixAuthCommandError> {
    let body = body.trim();
    if body.is_empty() || body.len() > 1024 {
        return Err(map_send_error("v-send-sticker-invalid-body"));
    }
    let mxc = mxc.trim();
    if mxc.is_empty() || mxc.len() > 1024 {
        return Err(map_send_error("v-send-sticker-invalid-mxc"));
    }
    let mxc_ref: &MxcUri = mxc.into();
    if !mxc_ref.is_valid() {
        return Err(map_send_error("v-send-sticker-invalid-mxc"));
    }
    let url: OwnedMxcUri = mxc_ref.to_owned();

    let mut info = ImageInfo::new();
    info.width = width.and_then(UInt::new);
    info.height = height.and_then(UInt::new);
    info.size = size.and_then(UInt::new);
    if let Some(mimetype) = mimetype {
        let mime = mimetype.trim();
        if !mime.is_empty() {
            if mime.len() > 255 || !mime.chars().all(|c| c.is_ascii_graphic()) {
                return Err(map_send_error("v-send-sticker-invalid-mimetype"));
            }
            info.mimetype = Some(mime.to_owned());
        }
    }

    let mut content = StickerEventContent::new(body.to_owned(), info, url);
    content.relates_to = match (thread_root, reply_to) {
        (Some(root), Some(reply)) => Some(Relation::Thread(Thread::reply(root, reply))),
        (Some(root), None) => Some(Relation::Thread(Thread::without_fallback(root))),
        (None, Some(reply)) => Some(Relation::Reply(Reply::with_event_id(reply))),
        (None, None) => None,
    };
    Ok(content)
}

/// Build validated room-message content for the native composer owner.
///
/// Relation rules (V-SEND.4 + V-SEND.5):
/// - `thread_root` + `reply_to` → `m.thread` with genuine in-thread reply
///   (`is_falling_back: false`); root and reply ids may be equal when starting
///   a thread from the root event.
/// - `thread_root` only → `m.thread` without in-reply fallback.
/// - `reply_to` only → classic `m.in_reply_to` reply (no thread).
pub(crate) fn message_content(
    body: String,
    msg_type: Option<String>,
    formatted_body: Option<String>,
    mention_user_ids: Option<Vec<String>>,
    mention_room: bool,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> Result<RoomMessageEventContent, MatrixAuthCommandError> {
    let mut content = match (msg_type.as_deref().unwrap_or("m.text"), formatted_body) {
        ("m.text", Some(html)) => RoomMessageEventContent::text_html(body, html),
        ("m.text", None) => RoomMessageEventContent::text_plain(body),
        ("m.emote", Some(html)) => RoomMessageEventContent::emote_html(body, html),
        ("m.emote", None) => RoomMessageEventContent::emote_plain(body),
        ("m.notice", Some(html)) => RoomMessageEventContent::notice_html(body, html),
        ("m.notice", None) => RoomMessageEventContent::notice_plain(body),
        _ => {
            return Err(MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix message type is invalid.",
                "v-send.4-invalid-message-type",
            ));
        }
    };
    let user_ids = mention_user_ids
        .unwrap_or_default()
        .into_iter()
        .map(|user_id| {
            user_id.parse::<OwnedUserId>().map_err(|_| {
                MatrixAuthCommandError::new(
                    "InvalidRequest",
                    "A native Matrix mention user ID is invalid.",
                    "v-send.4-invalid-mention-user-id",
                )
            })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut mentions = Mentions::new();
    mentions.user_ids = user_ids;
    mentions.room = mention_room;
    content.mentions = Some(mentions);
    content.relates_to = match (thread_root, reply_to) {
        (Some(root), Some(reply)) => Some(Relation::Thread(Thread::reply(root, reply))),
        (Some(root), None) => Some(Relation::Thread(Thread::without_fallback(root))),
        (None, Some(reply)) => Some(Relation::Reply(Reply::with_event_id(reply))),
        (None, None) => None,
    };
    Ok(content)
}

async fn send_message_to_room(
    room: &Room,
    content: RoomMessageEventContent,
    txn_id: Option<OwnedTransactionId>,
) -> matrix_sdk::Result<String> {
    let send = room.send(content);
    let result = match txn_id {
        Some(txn_id) => send.with_transaction_id(txn_id).await?,
        None => send.await?,
    };
    Ok(result.response.event_id.to_string())
}

fn map_attachment_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-send.1-attachment-empty"
        | "v-send.1-attachment-invalid-filename"
        | "v-send.1-attachment-invalid-mime"
        | "p7.4-invalid-room-id"
        | "p7.4-empty-media-handle"
        | "p7.4-file-name-cap"
        | "p7.4-file-too-large"
        | "p7.4-forbidden-handle-scheme"
        | "p7.4-forbidden-handle" => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix attachment request is invalid.",
            diagnostic_id,
        ),
        "v-send.1-attachment-too-large" | "p7.4-active-attachment-cap" => {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix attachment exceeds the allowed size or concurrency.",
                diagnostic_id,
            )
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix attachment could not be sent.",
            diagnostic_id,
        ),
    }
}

fn validate_attachment_filename(filename: &str) -> Result<String, MatrixAuthCommandError> {
    let filename = filename.trim();
    if filename.is_empty() || filename.chars().count() > 255 {
        return Err(map_attachment_error("v-send.1-attachment-invalid-filename"));
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains('\0') {
        return Err(map_attachment_error("v-send.1-attachment-invalid-filename"));
    }
    Ok(filename.to_owned())
}

fn validate_attachment_mime(mime_type: &str) -> Result<Mime, MatrixAuthCommandError> {
    let mime_type = mime_type.trim();
    if mime_type.is_empty() || mime_type.len() > 255 {
        return Err(map_attachment_error("v-send.1-attachment-invalid-mime"));
    }
    mime_type
        .parse::<Mime>()
        .map_err(|_| map_attachment_error("v-send.1-attachment-invalid-mime"))
}

fn attachment_kind_for_mime(mime: &Mime) -> AttachmentKind {
    match mime.type_() {
        mime::IMAGE => AttachmentKind::Image,
        mime::VIDEO => AttachmentKind::Video,
        mime::AUDIO => AttachmentKind::Audio,
        _ => AttachmentKind::File,
    }
}

async fn send_attachment_to_room(
    room: &Room,
    filename: &str,
    mime_type: &Mime,
    data: Vec<u8>,
    reply_to: Option<OwnedEventId>,
    thread_root: Option<OwnedEventId>,
) -> matrix_sdk::Result<String> {
    let mut config = AttachmentConfig::new();
    if let Some(event_id) = reply_to {
        // Explicit thread root from the product draft forces a thread relation
        // (start thread / reply in thread). Otherwise preserve the prior
        // MaybeThreaded behavior so existing non-thread replies keep working.
        let enforce_thread = if thread_root.is_some() {
            EnforceThread::Threaded(ReplyWithinThread::Yes)
        } else {
            EnforceThread::MaybeThreaded
        };
        config = config.reply(Some(AttachmentReply {
            event_id,
            enforce_thread,
            add_mentions: AddMentions::Yes,
        }));
    }
    let response = room
        .send_attachment(filename, mime_type, data, config)
        .await?;
    Ok(response.event_id.to_string())
}

fn snapshot(session: Option<&ManagedMatrixSession>) -> MatrixSessionSnapshot {
    match session {
        None => MatrixSessionSnapshot::LoggedOut,
        Some(active) => MatrixSessionSnapshot::LoggedIn {
            user_id: active.identity.user_id.clone(),
            device_id: active.identity.device_id.clone(),
            homeserver_url: active.identity.homeserver_url.clone(),
        },
    }
}

async fn build_client(
    app_data_root: &Path,
    identity: AccountIdentity,
) -> Result<Client, MatrixAuthCommandError> {
    let store_key = get_or_create_store_key(
        &KeyringStoreKeyVault::new(),
        &StoreKeyId::from_identity(&identity),
    )
    .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-store-key-unavailable"))?;
    let config = ClientBuildConfig::product_default(app_data_root, identity, Some(store_key))
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-client-config-failed"))?;
    build_unauthenticated_client(&config)
        .await
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-client-build-failed"))
}

/// Ephemeral unauthenticated client for password-reset (no product session, no keyring key).
async fn build_password_reset_client(
    app: &AppHandle,
    homeserver_url: &str,
) -> Result<Client, MatrixAuthCommandError> {
    let homeserver_url = normalize_homeserver_url(homeserver_url)
        .map_err(map_password_reset_auth_error)?
        .into_string();
    let user_id =
        password_reset_ephemeral_user_id(&homeserver_url).map_err(map_password_reset_auth_error)?;
    let identity = AccountIdentity::new(&user_id, &homeserver_url).map_err(|_| {
        MatrixAuthCommandError::invalid_input("v-auth.4-password-reset-identity-invalid")
    })?;
    let app_data_root = app_data_root(app)?;
    // Process-local store key — never persisted to the OS credential store.
    let store_key = StoreKeyMaterial::generate().map_err(|_| {
        MatrixAuthCommandError::unavailable("v-auth.4-password-reset-store-key-unavailable")
    })?;
    let config = ClientBuildConfig::product_default(&app_data_root, identity, Some(store_key))
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("v-auth.4-password-reset-client-config-failed")
        })?;
    build_unauthenticated_client(&config).await.map_err(|_| {
        MatrixAuthCommandError::unavailable("v-auth.4-password-reset-client-build-failed")
    })
}

fn map_password_reset_auth_error(error: AuthError) -> MatrixAuthCommandError {
    let code = match error {
        AuthError::AuthenticationRejected { .. } => "Forbidden",
        AuthError::UserDeactivated { .. } => "UserDeactivated",
        AuthError::RateLimited { .. } => "RateLimited",
        AuthError::InvalidInput { .. } => "InvalidRequest",
        AuthError::Connectivity { .. }
        | AuthError::HomeserverUnavailable { .. }
        | AuthError::WellKnownNotFound { .. } => "InvalidServer",
        AuthError::UnsupportedCapability { .. } => "Unsupported",
        AuthError::InteractiveAuthRequired { .. } => "Unauthorized",
        _ => "Unknown",
    };
    let message = match code {
        "Forbidden" => "The password reset request was rejected.",
        "UserDeactivated" => "The Matrix account is deactivated.",
        "RateLimited" => "The password reset request was rate limited.",
        "InvalidRequest" => "The password reset request is invalid.",
        "InvalidServer" => "The Matrix homeserver is unavailable.",
        "Unsupported" => "The homeserver requires an unsupported authentication stage.",
        "Unauthorized" => "Additional authentication is required to reset the password.",
        _ => "Native password reset failed.",
    };
    MatrixAuthCommandError::new(code, message, error.diagnostic_id())
}

fn account_identity(
    identity: &MatrixLoginIdentity,
) -> Result<AccountIdentity, MatrixAuthCommandError> {
    AccountIdentity::new(&identity.user_id, &identity.homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-persisted-identity-invalid"))
}

fn app_data_root(app: &AppHandle) -> Result<PathBuf, MatrixAuthCommandError> {
    app.path()
        .app_data_dir()
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-app-data-dir-unavailable"))
}

fn active_identity_path(app_data_root: &Path) -> PathBuf {
    app_data_root
        .join(MATRIX_DATA_DIR)
        .join(ACTIVE_SESSION_FILE)
}

fn write_active_identity(
    app_data_root: &Path,
    identity: &MatrixLoginIdentity,
) -> Result<(), MatrixAuthCommandError> {
    let path = active_identity_path(app_data_root);
    let parent = path
        .parent()
        .ok_or_else(|| MatrixAuthCommandError::unavailable("d0.1-active-session-path-invalid"))?;
    fs::create_dir_all(parent)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-dir-failed"))?;
    let bytes = serde_json::to_vec(identity)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-encode-failed"))?;
    fs::write(path, bytes)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-write-failed"))
}

fn read_active_identity(
    app_data_root: &Path,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let path = active_identity_path(app_data_root);
    if !path.is_file() {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "No persisted native Matrix session was found.",
            "d0.1-active-session-missing",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-read-failed"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-invalid"))
}

fn remove_active_identity(app_data_root: &Path) -> Result<(), MatrixAuthCommandError> {
    let path = active_identity_path(app_data_root);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(MatrixAuthCommandError::unavailable(
            "d0.1-active-session-remove-failed",
        )),
    }
}

fn map_auth_error(error: AuthError) -> MatrixAuthCommandError {
    let code = match error {
        AuthError::AuthenticationRejected { .. } => "Forbidden",
        AuthError::UserDeactivated { .. } => "UserDeactivated",
        AuthError::RateLimited { .. } => "RateLimited",
        AuthError::InvalidInput { .. } => "InvalidRequest",
        AuthError::Connectivity { .. }
        | AuthError::HomeserverUnavailable { .. }
        | AuthError::WellKnownNotFound { .. } => "InvalidServer",
        _ => "Unknown",
    };
    let message = match code {
        "Forbidden" => "The Matrix login credentials were rejected.",
        "UserDeactivated" => "The Matrix account is deactivated.",
        "RateLimited" => "The Matrix login request was rate limited.",
        "InvalidRequest" => "The native Matrix login request is invalid.",
        "InvalidServer" => "The Matrix homeserver is unavailable.",
        _ => "Native Matrix login failed.",
    };
    MatrixAuthCommandError::new(code, message, error.diagnostic_id())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room_key_selection(selection_id: u64, label: &str) -> SelectedRoomKeyImport {
        SelectedRoomKeyImport {
            selection_id,
            path: PathBuf::from(format!("/opaque/{label}")),
            file_label: label.to_owned(),
        }
    }

    #[test]
    fn room_key_import_reservation_is_consumed_on_success_path() {
        let mut slot = Some(room_key_selection(4, "backup.keys"));
        let reserved = reserve_room_key_import_selection(&mut slot, 4, "passphrase").unwrap();
        assert_eq!(reserved.selection_id, 4);
        assert!(slot.is_none());
    }

    #[test]
    fn failed_room_key_import_restores_same_generation_empty_slot() {
        let mut slot = None;
        let restored = restore_reserved_room_key_import(
            7,
            Some(7),
            &mut slot,
            room_key_selection(4, "backup.keys"),
        );
        assert!(restored);
        assert_eq!(slot.as_ref().unwrap().selection_id, 4);
    }

    #[test]
    fn failed_room_key_import_never_overwrites_newer_selection() {
        let mut slot = Some(room_key_selection(5, "newer.keys"));
        let restored = restore_reserved_room_key_import(
            7,
            Some(7),
            &mut slot,
            room_key_selection(4, "older.keys"),
        );
        assert!(!restored);
        assert_eq!(slot.as_ref().unwrap().selection_id, 5);
    }

    #[test]
    fn failed_room_key_import_is_discarded_after_generation_change_or_logout() {
        let mut replacement_generation_slot = None;
        assert!(!restore_reserved_room_key_import(
            7,
            Some(8),
            &mut replacement_generation_slot,
            room_key_selection(4, "old.keys"),
        ));
        assert!(replacement_generation_slot.is_none());

        let mut logged_out_slot = None;
        assert!(!restore_reserved_room_key_import(
            7,
            None,
            &mut logged_out_slot,
            room_key_selection(4, "logged-out.keys"),
        ));
        assert!(logged_out_slot.is_none());
    }

    #[test]
    fn empty_passphrase_and_invalid_room_key_selection_leave_slot_untouched() {
        let mut slot = Some(room_key_selection(4, "backup.keys"));
        let empty = reserve_room_key_import_selection(&mut slot, 4, "").unwrap_err();
        assert_eq!(empty.diagnostic_id, "v-crypto.5-passphrase-empty");
        assert_eq!(slot.as_ref().unwrap().selection_id, 4);

        let invalid = reserve_room_key_import_selection(&mut slot, 99, "passphrase").unwrap_err();
        assert_eq!(invalid.diagnostic_id, "v-crypto.5-import-selection-invalid");
        assert_eq!(slot.as_ref().unwrap().selection_id, 4);
    }

    #[test]
    fn login_identity_and_snapshot_serialization_never_have_token_fields() {
        let identity = MatrixLoginIdentity {
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://matrix.example.org".into(),
        };
        let identity_json = serde_json::to_string(&identity).unwrap();
        let snapshot_json = serde_json::to_string(&MatrixSessionSnapshot::LoggedIn {
            user_id: identity.user_id.clone(),
            device_id: identity.device_id.clone(),
            homeserver_url: identity.homeserver_url.clone(),
        })
        .unwrap();
        for json in [identity_json, snapshot_json] {
            assert!(!json.contains("accessToken"));
            assert!(!json.contains("access_token"));
            assert!(!json.contains("refreshToken"));
            assert!(!json.contains("refresh_token"));
            assert!(!json.contains("password"));
        }
    }

    #[test]
    fn v_auth_2_product_has_no_token_login_command_or_login_token_sdk_call() {
        // Desktop product does not retain m.login.token (V-AUTH.2). Password
        // login remains the only production Tauri login command.
        //
        // Read only production sections (exclude this tests module) so the
        // negative assertions below do not match their own string literals.
        let product_src = include_str!("product.rs");
        let product_prod = product_src
            .split("#[cfg(test)]")
            .next()
            .expect("product production section");
        let login_src = include_str!("login.rs");
        let login_prod = login_src
            .split("#[cfg(test)]")
            .next()
            .expect("login production section");
        let lib_src = include_str!("../../lib.rs");

        assert!(
            product_prod.contains("pub async fn matrix_login_password"),
            "password login product command must remain registered"
        );
        assert!(
            !product_prod.contains("pub async fn matrix_login_token"),
            "token login must not be a product Tauri command"
        );
        assert!(
            !lib_src.contains("matrix_login_token"),
            "token login must not be registered in the invoke handler"
        );
        assert!(
            !login_prod.contains("fn login_with_token"),
            "login_with_token foundation must not remain after V-AUTH.2 non-retention"
        );
        assert!(
            !login_prod.contains(".login_token("),
            "SDK login_token must not be called from the desktop login module"
        );

        let login_fn = product_prod
            .split("pub async fn matrix_login_password")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn ").next())
            .expect("matrix_login_password body");
        // Secrets may be requested (request_refresh_token) but must not be read
        // from/returned on the product DTO path or printed.
        for forbidden in [
            "access_token",
            "login_token",
            "println!",
            "log::",
            "tracing::",
        ] {
            assert!(
                !login_fn.contains(forbidden),
                "password login product path must not reference {forbidden}"
            );
        }
        assert!(
            login_fn.contains("request_refresh_token: true"),
            "password login should request refresh tokens host-side without exposing them"
        );
        assert!(
            login_fn.contains("MatrixLoginIdentity"),
            "password login must return only privacy-safe identity fields"
        );
    }

    #[test]
    fn crypto_status_projection_is_privacy_safe_and_reports_cross_signing_shape() {
        let status = crypto_status(
            7,
            Some(CrossSigningStatus {
                has_master: true,
                has_self_signing: false,
                has_user_signing: false,
            }),
        );
        assert!(status.encryption_enabled);
        assert_eq!(status.cross_signing_state, MatrixCrossSigningState::Partial);

        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            json,
            r#"{"sessionGeneration":7,"encryptionEnabled":true,"crossSigningState":"partial"}"#
        );
        for forbidden in ["token", "key", "ciphertext", "passphrase"] {
            assert!(!json.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn crypto_status_distinguishes_unavailable_unset_and_ready() {
        assert_eq!(
            cross_signing_state(None),
            MatrixCrossSigningState::Unavailable
        );
        assert_eq!(
            cross_signing_state(Some(&CrossSigningStatus {
                has_master: false,
                has_self_signing: false,
                has_user_signing: false,
            })),
            MatrixCrossSigningState::NotSetUp
        );
        assert_eq!(
            cross_signing_state(Some(&CrossSigningStatus {
                has_master: true,
                has_self_signing: true,
                has_user_signing: true,
            })),
            MatrixCrossSigningState::Ready
        );
    }

    #[test]
    fn missing_active_identity_has_clear_restore_error() {
        let root = std::env::temp_dir().join(format!("synara-d0.1-missing-{}", std::process::id()));
        let error = read_active_identity(&root).unwrap_err();
        assert_eq!(error.code, "Forbidden");
        assert_eq!(error.diagnostic_id, "d0.1-active-session-missing");
        assert!(error.message.contains("No persisted native Matrix session"));
    }

    #[test]
    fn active_identity_round_trip_contains_only_identity() {
        let root =
            std::env::temp_dir().join(format!("synara-d0.1-identity-{}", std::process::id()));
        let identity = MatrixLoginIdentity {
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://matrix.example.org".into(),
        };
        write_active_identity(&root, &identity).unwrap();
        assert_eq!(read_active_identity(&root).unwrap(), identity);
        remove_active_identity(&root).unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn session_generations_are_monotonic() {
        let state = MatrixAuthState::new();
        assert_eq!(state.current_generation(), 0);
        assert_eq!(state.next_generation(), 1);
        assert_eq!(state.next_generation(), 2);
        assert_eq!(state.current_generation(), 2);
    }

    #[test]
    fn send_result_serialization_is_privacy_safe() {
        let result = MatrixSendTextResult {
            room_id: "!room:example.org".into(),
            event_id: "$event:example.org".into(),
            local_txn_id: "local-txn-1".into(),
            status: "sent",
        };
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(
            json,
            r#"{"roomId":"!room:example.org","eventId":"$event:example.org","localTxnId":"local-txn-1","status":"sent"}"#
        );
        assert!(!json.contains("token"));
        assert!(!json.contains("ciphertext"));
    }

    #[test]
    fn sticker_content_preserves_mxc_info_and_reply() {
        let content = sticker_content(
            "cat".into(),
            "mxc://example.org/sticker1".into(),
            Some(128),
            Some(128),
            Some("image/png".into()),
            Some(2048),
            Some("$event:example.org".parse().unwrap()),
            None,
        )
        .unwrap();
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["body"], "cat");
        assert_eq!(json["url"], "mxc://example.org/sticker1");
        assert_eq!(json["info"]["w"], 128);
        assert_eq!(json["info"]["h"], 128);
        assert_eq!(json["info"]["mimetype"], "image/png");
        assert_eq!(json["info"]["size"], 2048);
        assert_eq!(
            json["m.relates_to"]["m.in_reply_to"]["event_id"],
            "$event:example.org"
        );
        assert!(json.get("msgtype").is_none());
        assert!(!json.to_string().contains("token"));
    }

    #[test]
    fn sticker_content_emits_thread_relation() {
        let threaded = sticker_content(
            "thread sticker".into(),
            "mxc://example.org/s".into(),
            None,
            None,
            None,
            None,
            Some("$child:example.org".parse().unwrap()),
            Some("$root:example.org".parse().unwrap()),
        )
        .unwrap();
        let json = serde_json::to_value(&threaded).unwrap();
        assert_eq!(json["m.relates_to"]["rel_type"], "m.thread");
        assert_eq!(json["m.relates_to"]["event_id"], "$root:example.org");
        assert_eq!(
            json["m.relates_to"]["m.in_reply_to"]["event_id"],
            "$child:example.org"
        );
    }

    #[test]
    fn sticker_content_rejects_invalid_body_and_mxc() {
        let empty = sticker_content(
            "  ".into(),
            "mxc://example.org/s".into(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(empty.diagnostic_id, "v-send-sticker-invalid-body");

        let bad_mxc = sticker_content(
            "ok".into(),
            "https://evil.example/img.png".into(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(bad_mxc.diagnostic_id, "v-send-sticker-invalid-mxc");
    }

    #[test]
    fn message_content_preserves_type_html_mentions_and_reply() {
        let plain = message_content("hello".into(), None, None, None, false, None, None).unwrap();
        let plain_json = serde_json::to_value(plain).unwrap();
        assert_eq!(plain_json["body"], "hello");
        assert_eq!(plain_json["msgtype"], "m.text");
        assert_eq!(plain_json["m.mentions"], serde_json::json!({}));
        assert!(plain_json.get("formatted_body").is_none());

        let reply = message_content(
            "reply".into(),
            Some("m.emote".into()),
            Some("<strong>reply</strong>".into()),
            Some(vec!["@alice:example.org".into()]),
            true,
            Some("$event:example.org".parse().unwrap()),
            None,
        )
        .unwrap();
        let reply_json = serde_json::to_value(reply).unwrap();
        assert_eq!(reply_json["msgtype"], "m.emote");
        assert_eq!(reply_json["format"], "org.matrix.custom.html");
        assert_eq!(reply_json["formatted_body"], "<strong>reply</strong>");
        assert_eq!(
            reply_json["m.mentions"],
            serde_json::json!({
                "user_ids": ["@alice:example.org"],
                "room": true
            })
        );
        assert_eq!(
            reply_json["m.relates_to"]["m.in_reply_to"]["event_id"],
            "$event:example.org"
        );
        assert!(reply_json["m.relates_to"].get("rel_type").is_none());

        let notice = message_content(
            "notice".into(),
            Some("m.notice".into()),
            None,
            None,
            false,
            None,
            None,
        )
        .unwrap();
        assert_eq!(serde_json::to_value(notice).unwrap()["msgtype"], "m.notice");
    }

    #[test]
    fn message_content_emits_thread_relation_for_in_thread_reply() {
        let threaded = message_content(
            "in thread".into(),
            Some("m.text".into()),
            None,
            None,
            false,
            Some("$child:example.org".parse().unwrap()),
            Some("$root:example.org".parse().unwrap()),
        )
        .unwrap();
        let json = serde_json::to_value(threaded).unwrap();
        assert_eq!(json["m.relates_to"]["rel_type"], "m.thread");
        assert_eq!(json["m.relates_to"]["event_id"], "$root:example.org");
        assert_eq!(
            json["m.relates_to"]["m.in_reply_to"]["event_id"],
            "$child:example.org"
        );
        // Genuine reply in thread — is_falling_back omitted/false (serde skip default).
        assert!(
            json["m.relates_to"]
                .get("is_falling_back")
                .map(|v| v == false)
                .unwrap_or(true),
            "is_falling_back must be false/absent for Thread::reply"
        );
    }

    #[test]
    fn message_content_emits_thread_without_fallback_when_only_root() {
        let root_only = message_content(
            "start".into(),
            None,
            None,
            None,
            false,
            None,
            Some("$root:example.org".parse().unwrap()),
        )
        .unwrap();
        let json = serde_json::to_value(root_only).unwrap();
        assert_eq!(json["m.relates_to"]["rel_type"], "m.thread");
        assert_eq!(json["m.relates_to"]["event_id"], "$root:example.org");
        assert!(json["m.relates_to"].get("m.in_reply_to").is_none());
    }

    #[test]
    fn message_content_rejects_invalid_type_and_mentions() {
        let invalid_type = message_content(
            "body".into(),
            Some("m.image".into()),
            None,
            None,
            false,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(invalid_type.diagnostic_id, "v-send.4-invalid-message-type");

        let invalid_mention = message_content(
            "body".into(),
            None,
            None,
            Some(vec!["not-a-user".into()]),
            false,
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(
            invalid_mention.diagnostic_id,
            "v-send.4-invalid-mention-user-id"
        );
    }

    #[test]
    fn send_input_parsers_reject_invalid_ids() {
        assert_eq!(
            parse_send_room_id("not-a-room").unwrap_err().diagnostic_id,
            "d0.4-send-invalid-room-id"
        );
        assert_eq!(
            parse_reply_event_id(Some("not-an-event".into()))
                .unwrap_err()
                .diagnostic_id,
            "d0.4-send-invalid-reply-event-id"
        );
        assert_eq!(
            parse_thread_root_event_id(Some("not-an-event".into()))
                .unwrap_err()
                .diagnostic_id,
            "v-send.5-invalid-thread-root-event-id"
        );
        assert_eq!(
            parse_transaction_id(Some(String::new()))
                .unwrap_err()
                .diagnostic_id,
            "d0.4-send-invalid-transaction-id"
        );
    }
}
