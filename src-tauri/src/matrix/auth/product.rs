//! D0.1–D0.3 product password-login, native session, sync, and timeline ownership.
//!
//! This is the only desktop product boundary for password login. The live
//! `matrix_sdk::Client` and all access/refresh tokens remain in the Rust host.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use matrix_sdk::{
    encryption::CrossSigningStatus,
    ruma::{
        api::client::uiaa,
        events::{
            relation::Reply,
            room::message::{Relation, RoomMessageEventContent},
            Mentions,
        },
        OwnedEventId, OwnedRoomId, OwnedTransactionId,
    },
    Client, Room,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;
use zeroize::Zeroize;

use super::{login_with_password, normalize_homeserver_url, AuthError, LoginOptions};
use crate::matrix::backup::live::{
    self as live_backup, NativeBackupOperationResult, NativeBackupStatus,
};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::cross_signing::live::{
    project_status, supported_authentication, NativeCrossSigningSetupOutcome,
    NativeCrossSigningSetupResult, NativeCrossSigningStatus, SupportedBootstrapAuthentication,
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
use crate::matrix::room_list::{snapshot_from_sync_owner, NativeRoomListSnapshot};
use crate::matrix::secret_storage::live::{
    self as live_secret_storage, NativeSecretStorageOperationResult, NativeSecretStorageStatus,
};
use crate::matrix::send::SendQueue;
use crate::matrix::store::{
    get_or_create_store_key, AccountIdentity, KeyringStoreKeyVault, StoreKeyId,
};
use crate::matrix::sync::{
    build_sync_service, unconfigured_snapshot, SyncReadinessSnapshot, SyncServiceConfig,
    SyncServiceOwner,
};
use crate::matrix::timeline::{
    NativeTimelineDirection, NativeTimelineRegistry, NativeTimelineSnapshot,
};
use crate::matrix::verification::live::{
    NativeDeviceVerificationStatus, NativeVerificationInbox, NativeVerificationOwner,
    NativeVerificationRequest,
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
    timelines: NativeTimelineRegistry,
    sends: SendQueue,
    verification: NativeVerificationOwner,
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
        timelines: NativeTimelineRegistry::new(session_generation),
        sends: SendQueue::new(session_generation),
        verification,
        pending_cross_signing_auth_session: None,
        room_key_transfer: Arc::new(Mutex::new(RoomKeyTransferFlow::new(session_generation))),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok(identity)
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
    live_room_keys::require_passphrase(passphrase)?;
    let (client, generation, flow, selected) = {
        let mut session = state.session.lock().await;
        let active = require_room_key_session_mut(session.as_mut())?;
        if active
            .selected_room_key_import
            .as_ref()
            .is_none_or(|selected| selected.selection_id != selection_id)
        {
            return Err(MatrixAuthCommandError::new(
                "InvalidRequest",
                "Choose an encrypted room-key file before importing.",
                "v-crypto.5-import-selection-invalid",
            ));
        }
        let selected = active.selected_room_key_import.take().ok_or_else(|| {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "Choose an encrypted room-key file before importing.",
                "v-crypto.5-import-selection-invalid",
            )
        })?;
        (
            active.client.clone(),
            active.sync.session_generation(),
            Arc::clone(&active.room_key_transfer),
            selected,
        )
    };
    let result = live_room_keys::import(&client, generation, &flow, selected, passphrase).await?;
    require_current_room_key_generation(state, generation).await?;
    Ok(result)
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
pub async fn matrix_device_verification_status(
    state: State<'_, MatrixAuthState>,
    device_id: String,
) -> Result<NativeDeviceVerificationStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_verification_session(session.as_ref())?;
    let user_id = active.client.user_id().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.1-status-requires-session",
        )
    })?;
    let device_id = matrix_sdk::ruma::OwnedDeviceId::from(device_id);
    let device = active
        .client
        .encryption()
        .get_device(user_id, &device_id)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "Device verification status is unavailable.",
                "v-crypto.1-status-query-failed",
            )
        })?;
    Ok(match device {
        Some(device) if device.is_verified() => NativeDeviceVerificationStatus::Verified,
        Some(_) => NativeDeviceVerificationStatus::Unverified,
        None => NativeDeviceVerificationStatus::Unavailable,
    })
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
pub async fn matrix_timeline_open(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeTimelineSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .open(&active.client, &room_id)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_snapshot(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeTimelineSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    active
        .timelines
        .snapshot(&room_id)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_timeline_paginate(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    dir: NativeTimelineDirection,
) -> Result<NativeTimelineSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    active
        .timelines
        .paginate(&room_id, dir)
        .await
        .map_err(map_timeline_error)
}

#[tauri::command]
pub async fn matrix_send_text(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    body: String,
    reply_to: Option<String>,
    txn_id: Option<String>,
) -> Result<MatrixSendTextResult, MatrixAuthCommandError> {
    let room_id = parse_send_room_id(&room_id)?;
    let reply_to = parse_reply_event_id(reply_to)?;
    let txn_id = parse_transaction_id(txn_id)?;

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

    let send_result = send_text_to_room(&room, body, reply_to, txn_id).await;

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
    let sync = start_sync_owner(&client, session_generation).await?;
    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync,
        timelines: NativeTimelineRegistry::new(session_generation),
        sends: SendQueue::new(session_generation),
        verification,
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

fn text_message_content(body: String, reply_to: Option<OwnedEventId>) -> RoomMessageEventContent {
    let mut content = RoomMessageEventContent::text_plain(body);
    content.mentions = Some(Mentions::new());
    if let Some(event_id) = reply_to {
        content.relates_to = Some(Relation::Reply(Reply::with_event_id(event_id)));
    }
    content
}

async fn send_text_to_room(
    room: &Room,
    body: String,
    reply_to: Option<OwnedEventId>,
    txn_id: Option<OwnedTransactionId>,
) -> matrix_sdk::Result<String> {
    let send = room.send(text_message_content(body, reply_to));
    let result = match txn_id {
        Some(txn_id) => send.with_transaction_id(txn_id).await?,
        None => send.await?,
    };
    Ok(result.response.event_id.to_string())
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
    fn text_content_sets_empty_mentions_and_optional_reply() {
        let plain = text_message_content("hello".into(), None);
        let plain_json = serde_json::to_value(plain).unwrap();
        assert_eq!(plain_json["body"], "hello");
        assert_eq!(plain_json["msgtype"], "m.text");
        assert_eq!(plain_json["m.mentions"], serde_json::json!({}));

        let reply =
            text_message_content("reply".into(), Some("$event:example.org".parse().unwrap()));
        let reply_json = serde_json::to_value(reply).unwrap();
        assert_eq!(
            reply_json["m.relates_to"]["m.in_reply_to"]["event_id"],
            "$event:example.org"
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
            parse_transaction_id(Some(String::new()))
                .unwrap_err()
                .diagnostic_id,
            "d0.4-send-invalid-transaction-id"
        );
    }
}
