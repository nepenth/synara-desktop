//! D0.1 product password-login and native session ownership.
//!
//! This is the only desktop product boundary for password login. The live
//! `matrix_sdk::Client` and all access/refresh tokens remain in the Rust host.

use std::fs;
use std::path::{Path, PathBuf};

use matrix_sdk::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Mutex;

use super::{login_with_password, normalize_homeserver_url, AuthError, LoginOptions};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::lifecycle::{
    clear_session_material, persist_session_after_login, restore_session_from_vault,
    KeyringSessionMaterialVault,
};
use crate::matrix::store::{
    get_or_create_store_key, AccountIdentity, KeyringStoreKeyVault, StoreKeyId,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixAuthCommandError {
    pub code: &'static str,
    pub message: &'static str,
    pub diagnostic_id: &'static str,
}

impl MatrixAuthCommandError {
    fn new(code: &'static str, message: &'static str, diagnostic_id: &'static str) -> Self {
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
}

#[derive(Default)]
pub struct MatrixAuthState {
    session: Mutex<Option<ManagedMatrixSession>>,
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

    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
    });
    Ok(identity)
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
}
