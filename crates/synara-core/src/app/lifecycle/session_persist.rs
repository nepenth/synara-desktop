//! Post-login session secret persistence (P3.5).
//!
//! After a successful password login, extract the native Matrix session
//! from the live SDK `Client` and seal it into [`SessionMaterialVault`].
//!
//! - Tokens stay host-side only (vault blob); never on login IPC results
//!   or [`crate::dto::SessionSnapshot`].
//! - Does **not** call `Client::restore_session` (P3.6).
//! - Overwrite via [`super::session_material::rotate_persisted_session_tokens`]
//!   is the refresh-rotation structure.

use matrix_sdk::authentication::AuthSession;
use matrix_sdk::Client;

use crate::app::store::AccountIdentity;
use crate::transport::MatrixIpcErrorCategory;

use super::session_material::{
    persist_session_material, SessionMaterial, SessionMaterialMeta, SessionMaterialVault,
};
use super::LifecycleError;

/// Privacy-safe outcome of persisting session secrets after login.
///
/// **Never** includes access_token, refresh_token, or raw envelope bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPersistOutcome {
    pub meta: SessionMaterialMeta,
}

/// Extract session secrets from a logged-in SDK client and persist them.
///
/// `client` must already hold a native Matrix auth session (password login).
/// `identity` must match the session user id and homeserver URL (binding check).
///
/// OAuth sessions are rejected until a dedicated P3.3/P3.5 OAuth envelope lands.
pub fn persist_session_after_login<V: SessionMaterialVault + ?Sized>(
    client: &Client,
    identity: &AccountIdentity,
    vault: &V,
) -> Result<SessionPersistOutcome, LifecycleError> {
    let session = client.session().ok_or(LifecycleError::Vault {
        diagnostic_id: "p3.5-no-session-on-client",
        category: MatrixIpcErrorCategory::AuthenticationRejected,
    })?;

    let material = session_material_from_auth_session(identity, &session)?;
    let meta = material.public_meta()?;
    persist_session_material(vault, identity, &material)?;
    Ok(SessionPersistOutcome { meta })
}

/// Build sealed [`SessionMaterial`] from an SDK [`AuthSession`] without writing.
///
/// Binding: session `user_id` must equal `identity.user_id()`. Homeserver is
/// taken from `identity` (product account key) after a basic non-empty check.
pub fn session_material_from_auth_session(
    identity: &AccountIdentity,
    session: &AuthSession,
) -> Result<SessionMaterial, LifecycleError> {
    match session {
        AuthSession::Matrix(matrix) => {
            let user_id = matrix.meta.user_id.as_str();
            if user_id != identity.user_id() {
                return Err(LifecycleError::InvalidTarget {
                    diagnostic_id: "p3.5-session-user-mismatch",
                });
            }
            let device_id = matrix.meta.device_id.as_str();
            let access = matrix.tokens.access_token.as_str();
            let refresh = matrix.tokens.refresh_token.as_deref();
            SessionMaterial::from_matrix_tokens(identity, device_id, access, refresh)
        }
        AuthSession::OAuth(_) => Err(LifecycleError::Vault {
            diagnostic_id: "p3.5-oauth-session-unsupported",
            category: MatrixIpcErrorCategory::UnsupportedCapability,
        }),
        // Non-exhaustive AuthSession — future variants.
        _ => Err(LifecycleError::Vault {
            diagnostic_id: "p3.5-session-kind-unsupported",
            category: MatrixIpcErrorCategory::UnsupportedCapability,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::lifecycle::{
        clear_session_material, load_session_material, InMemorySessionMaterialVault,
    };
    use matrix_sdk::authentication::matrix::MatrixSession;
    use matrix_sdk::ruma::UserId;
    use matrix_sdk::{SessionMeta, SessionTokens};

    fn alice() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    fn matrix_session(access: &str, refresh: Option<&str>) -> AuthSession {
        AuthSession::Matrix(MatrixSession {
            meta: SessionMeta {
                user_id: UserId::parse("@alice:example.org").expect("test mxid"),
                device_id: "DEVICEABC".into(),
            },
            tokens: SessionTokens {
                access_token: access.to_owned(),
                refresh_token: refresh.map(str::to_owned),
            },
        })
    }

    #[test]
    fn from_auth_session_and_persist_round_trip() {
        let access = "syt_persist_hook_access_token";
        let refresh = "syr_persist_hook_refresh_token";
        let session = matrix_session(access, Some(refresh));
        let material = session_material_from_auth_session(&alice(), &session).unwrap();
        let meta = material.public_meta().unwrap();
        assert_eq!(meta.user_id, "@alice:example.org");
        assert_eq!(meta.device_id, "DEVICEABC");
        assert!(meta.has_refresh_token);

        let vault = InMemorySessionMaterialVault::new();
        persist_session_material(&vault, &alice(), &material).unwrap();
        let loaded = load_session_material(&vault, &alice()).unwrap().unwrap();
        let secrets = loaded.decode_host_secrets().unwrap();
        assert_eq!(secrets.access_token, access);
        assert_eq!(secrets.refresh_token.as_deref(), Some(refresh));

        let dbg = format!("{material:?}");
        assert!(!dbg.contains(access));
        assert!(!dbg.contains(refresh));

        assert!(clear_session_material(&vault, &alice()).unwrap());
    }

    #[test]
    fn user_mismatch_is_privacy_safe() {
        let session = matrix_session("syt_mismatch_token_secret", None);
        let other = AccountIdentity::new("@bob:example.org", "https://matrix.example.org").unwrap();
        let err = session_material_from_auth_session(&other, &session).unwrap_err();
        assert!(matches!(
            err,
            LifecycleError::InvalidTarget {
                diagnostic_id: "p3.5-session-user-mismatch"
            }
        ));
        let text = err.to_string();
        assert!(!text.contains("syt_mismatch_token_secret"));
        assert!(!text.contains("access_token"));
    }

    #[test]
    fn outcome_debug_has_no_secret_substrings() {
        let secret = "syt_should_never_appear_in_outcome";
        let outcome = SessionPersistOutcome {
            meta: SessionMaterialMeta {
                format_version: 1,
                kind: "matrix".into(),
                user_id: "@alice:example.org".into(),
                device_id: "DEV".into(),
                homeserver_url: "https://matrix.example.org".into(),
                has_refresh_token: true,
            },
        };
        let dbg = format!("{outcome:?}");
        // Privacy: outcome carries only metadata flags, never token values.
        assert!(!dbg.contains(secret));
        assert!(!dbg.contains("syt_"));
        // Field names may mention refresh; values must not look like tokens.
        assert!(!dbg.contains("access_token:"));
    }
}
