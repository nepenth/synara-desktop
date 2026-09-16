//! Desktop X.509 identity settings (Devices page). Human-only; no agent tools.

use std::path::{Path, PathBuf};

use synara_core::app::store::{AccountIdentity, StorePaths};
use synara_core::app::x509::{
    import_ca_pem, import_signer_pems, list_certificate_verified_user_ids, load_runtime, remove_ca,
    set_enabled, status_from_store, NativeX509IdentityStatus, X509StoreError,
};

use super::*;

fn map_x509_error(error: X509StoreError) -> MatrixAuthCommandError {
    let (code, message) = match error {
        X509StoreError::InvalidPem | X509StoreError::EmptySelection | X509StoreError::TooLarge => {
            ("InvalidRequest", "The certificate could not be imported.")
        }
        X509StoreError::CaNotFound => (
            "InvalidRequest",
            "That certificate authority is not imported.",
        ),
        X509StoreError::Io => ("Unknown", "X.509 identity settings are unavailable."),
    };
    MatrixAuthCommandError::new(code, message, error.diagnostic_id())
}

fn x509_unavailable(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "X.509 identity settings are unavailable.",
        diagnostic_id,
    )
}

fn x509_account_root(
    app: &AppHandle,
    session: &ManagedMatrixSession,
) -> Result<PathBuf, MatrixAuthCommandError> {
    let app_data_root = app
        .path()
        .app_data_dir()
        .map_err(|_| x509_unavailable("v-crypto.x509-app-data-unavailable"))?;
    let identity =
        AccountIdentity::new(&session.identity.user_id, &session.identity.homeserver_url)
            .map_err(|_| x509_unavailable("v-crypto.x509-identity-invalid"))?;
    let paths = StorePaths::derive(&app_data_root, &identity)
        .map_err(|_| x509_unavailable("v-crypto.x509-store-path-invalid"))?;
    Ok(paths.account_root().to_path_buf())
}

async fn pick_pem_file(title: &str) -> Option<PathBuf> {
    let handle = rfd::AsyncFileDialog::new()
        .set_title(title)
        .add_filter("Certificates", &["pem", "crt", "cer", "key"])
        .pick_file()
        .await?;
    Some(handle.path().to_path_buf())
}

fn read_pem_file(path: &Path, max_bytes: usize) -> Result<String, MatrixAuthCommandError> {
    let bytes = std::fs::read(path).map_err(|_| map_x509_error(X509StoreError::Io))?;
    if bytes.len() > max_bytes {
        return Err(map_x509_error(X509StoreError::TooLarge));
    }
    String::from_utf8(bytes).map_err(|_| map_x509_error(X509StoreError::InvalidPem))
}

async fn status_for_session(
    app: &AppHandle,
    state: &State<'_, MatrixAuthState>,
) -> Result<NativeX509IdentityStatus, MatrixAuthCommandError> {
    let (account_root, identities) = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref()).map_err(|_| {
            MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-crypto.x509-requires-session",
            )
        })?;
        let account_root = x509_account_root(app, active)?;
        let identities = if load_runtime(&account_root).should_inject_verifier() {
            list_certificate_verified_user_ids(&active.client).await
        } else {
            Vec::new()
        };
        (account_root, identities)
    };
    Ok(status_from_store(&account_root, identities))
}

#[tauri::command]
pub async fn matrix_x509_identity_status(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<NativeX509IdentityStatus, MatrixAuthCommandError> {
    status_for_session(&app, &state).await
}

#[tauri::command]
pub async fn matrix_x509_identity_set_enabled(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    enabled: bool,
) -> Result<NativeX509IdentityStatus, MatrixAuthCommandError> {
    let account_root = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref()).map_err(|_| {
            MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-crypto.x509-requires-session",
            )
        })?;
        x509_account_root(&app, active)?
    };
    set_enabled(&account_root, enabled).map_err(map_x509_error)?;
    status_for_session(&app, &state).await
}

#[tauri::command]
pub async fn matrix_x509_identity_import_ca(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<NativeX509IdentityStatus, MatrixAuthCommandError> {
    let account_root = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref()).map_err(|_| {
            MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-crypto.x509-requires-session",
            )
        })?;
        x509_account_root(&app, active)?
    };
    let Some(path) = pick_pem_file("Import certificate authority").await else {
        return status_for_session(&app, &state).await;
    };
    let pem = read_pem_file(&path, 256 * 1024)?;
    import_ca_pem(&account_root, &pem).map_err(map_x509_error)?;
    status_for_session(&app, &state).await
}

#[tauri::command]
pub async fn matrix_x509_identity_remove_ca(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    fingerprint: String,
) -> Result<NativeX509IdentityStatus, MatrixAuthCommandError> {
    let account_root = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref()).map_err(|_| {
            MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-crypto.x509-requires-session",
            )
        })?;
        x509_account_root(&app, active)?
    };
    let fingerprint = fingerprint.trim();
    if fingerprint.is_empty() || fingerprint.len() > 80 {
        return Err(map_x509_error(X509StoreError::CaNotFound));
    }
    remove_ca(&account_root, fingerprint).map_err(map_x509_error)?;
    status_for_session(&app, &state).await
}

#[tauri::command]
pub async fn matrix_x509_identity_import_signer(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<NativeX509IdentityStatus, MatrixAuthCommandError> {
    let account_root = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref()).map_err(|_| {
            MatrixAuthCommandError::new(
                "Forbidden",
                "No native Matrix session is active.",
                "v-crypto.x509-requires-session",
            )
        })?;
        x509_account_root(&app, active)?
    };
    let Some(cert_path) = pick_pem_file("Import client certificate").await else {
        return status_for_session(&app, &state).await;
    };
    let Some(key_path) = pick_pem_file("Import client private key").await else {
        return status_for_session(&app, &state).await;
    };
    let cert_pem = read_pem_file(&cert_path, 256 * 1024)?;
    let key_pem = read_pem_file(&key_path, 32 * 1024)?;
    import_signer_pems(&account_root, &cert_pem, &key_pem).map_err(map_x509_error)?;
    status_for_session(&app, &state).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn x509_commands_never_return_pem_on_the_wire() {
        let source = include_str!("product_commands.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production commands precede tests");
        assert!(production.contains("pub async fn matrix_x509_identity_status"));
        assert!(production.contains("pick_pem_file"));
        assert!(production.contains("v-crypto.x509-requires-session"));
        assert!(!production.contains("ring::default_provider"));
        assert!(!production.contains("BEGIN CERTIFICATE"));
        let pem_return = ["Ok(", "pem)"].concat();
        assert!(!production.contains(&pem_return));
        let build = include_str!("../../../build.rs");
        let capability = include_str!("../../../capabilities/main.json");
        for command in [
            "matrix_x509_identity_status",
            "matrix_x509_identity_set_enabled",
            "matrix_x509_identity_import_ca",
            "matrix_x509_identity_remove_ca",
            "matrix_x509_identity_import_signer",
        ] {
            assert!(production.contains(&format!("pub async fn {command}")));
            assert!(build.contains(&format!("\"{command}\"")));
            assert!(capability.contains(&format!("allow-{}", command.replace('_', "-"))));
        }
    }
}
