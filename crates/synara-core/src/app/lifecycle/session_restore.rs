//! Session restore from host vault onto SDK `Client` (P3.6).
//!
//! Loads sealed session material, binds it to [`AccountIdentity`], and installs
//! it via `Client::restore_session` (Matrix native auth only).
//!
//! - Tokens stay host-side; [`SessionRestoreOutcome`] never carries secrets.
//! - Does **not** start sync (later phases).
//! - Does **not** invent dual-backend or JS→Rust token migration.
//! - Multi-account: restore is always for one explicit `AccountIdentity`
//!   (account switch = pick identity, build client for that store, restore).

use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::ruma::UserId;
use matrix_sdk::{Client, SessionMeta, SessionTokens};

use crate::app::store::AccountIdentity;
use crate::transport::MatrixIpcErrorCategory;

use super::session_material::{
    load_session_material, HostMatrixSessionSecrets, SessionMaterial, SessionMaterialMeta,
    SessionMaterialVault,
};
use super::LifecycleError;

/// Privacy-safe outcome of a successful session restore.
///
/// **Never** includes access_token, refresh_token, or raw envelope bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestoreOutcome {
    pub meta: SessionMaterialMeta,
}

/// Build an SDK [`MatrixSession`] from host-decoded secrets bound to `identity`.
///
/// Binding checks:
/// - `secrets.user_id` must equal `identity.user_id()`
/// - `secrets.homeserver_url` must equal `identity.homeserver_url()`
pub fn matrix_session_from_host_secrets(
    identity: &AccountIdentity,
    secrets: &HostMatrixSessionSecrets,
) -> Result<MatrixSession, LifecycleError> {
    if secrets.user_id != identity.user_id() {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p3.6-session-user-mismatch",
        });
    }
    if secrets.homeserver_url != identity.homeserver_url() {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p3.6-session-homeserver-mismatch",
        });
    }
    if secrets.device_id.trim().is_empty() {
        return Err(LifecycleError::Vault {
            diagnostic_id: "p3.6-session-device-empty",
            category: MatrixIpcErrorCategory::SdkInvariant,
        });
    }
    if secrets.access_token.is_empty() {
        return Err(LifecycleError::Vault {
            diagnostic_id: "p3.6-session-access-empty",
            category: MatrixIpcErrorCategory::SdkInvariant,
        });
    }

    let user_id = UserId::parse(secrets.user_id.as_str()).map_err(|_| LifecycleError::Vault {
        diagnostic_id: "p3.6-session-user-id-parse",
        category: MatrixIpcErrorCategory::SdkInvariant,
    })?;

    Ok(MatrixSession {
        meta: SessionMeta {
            user_id,
            device_id: secrets.device_id.as_str().into(),
        },
        tokens: SessionTokens {
            access_token: secrets.access_token.clone(),
            refresh_token: secrets.refresh_token.clone(),
        },
    })
}

/// Restore sealed material onto an **unauthenticated** SDK client.
///
/// `client` must already be opened for the same account store (P2.3) and must
/// not already hold a session. Tokens never leave the host process.
pub async fn restore_session_onto_client(
    client: &Client,
    identity: &AccountIdentity,
    material: &SessionMaterial,
) -> Result<SessionRestoreOutcome, LifecycleError> {
    if client.session().is_some() {
        return Err(LifecycleError::Vault {
            diagnostic_id: "p3.6-client-already-has-session",
            category: MatrixIpcErrorCategory::SdkInvariant,
        });
    }

    let meta = material.public_meta()?;
    let secrets = material.decode_host_secrets()?;
    let session = matrix_session_from_host_secrets(identity, &secrets)?;

    client
        .restore_session(session)
        .await
        .map_err(map_restore_sdk_error)?;

    // Confirm session landed with privacy-safe identity only.
    let live = client.session().ok_or(LifecycleError::Vault {
        diagnostic_id: "p3.6-restore-no-session-after",
        category: MatrixIpcErrorCategory::SdkInvariant,
    })?;
    match live {
        matrix_sdk::authentication::AuthSession::Matrix(m) => {
            if m.meta.user_id.as_str() != identity.user_id() {
                return Err(LifecycleError::InvalidTarget {
                    diagnostic_id: "p3.6-restored-user-mismatch",
                });
            }
        }
        _ => {
            return Err(LifecycleError::Vault {
                diagnostic_id: "p3.6-restored-session-kind-unsupported",
                category: MatrixIpcErrorCategory::UnsupportedCapability,
            });
        }
    }

    Ok(SessionRestoreOutcome { meta })
}

/// Load sealed material from the vault for `identity` and restore onto `client`.
pub async fn restore_session_from_vault<V: SessionMaterialVault + ?Sized>(
    client: &Client,
    identity: &AccountIdentity,
    vault: &V,
) -> Result<SessionRestoreOutcome, LifecycleError> {
    let material = load_session_material(vault, identity)?.ok_or(LifecycleError::Vault {
        diagnostic_id: "p3.6-session-material-missing",
        category: MatrixIpcErrorCategory::AuthenticationRejected,
    })?;
    restore_session_onto_client(client, identity, &material).await
}

/// Whether the vault has **any** material for `identity` (privacy-safe probe).
///
/// Used for account-switch UX: list identities that can be restored without
/// decoding tokens. Does not open an SDK client.
pub fn has_persisted_session<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
) -> Result<bool, LifecycleError> {
    Ok(load_session_material(vault, identity)?.is_some())
}

fn map_restore_sdk_error(err: matrix_sdk::Error) -> LifecycleError {
    // Classify from raw SDK text internally; never export raw message (paths/tokens).
    let raw = format!("{err}");
    let lower = raw.to_ascii_lowercase();
    let diagnostic_id = if lower.contains("already") && lower.contains("session") {
        "p3.6-restore-session-already-set"
    } else if lower.contains("store") || lower.contains("sqlite") || lower.contains("crypto") {
        "p3.6-restore-store-failed"
    } else if lower.contains("token") || lower.contains("auth") || lower.contains("unauthorized") {
        "p3.6-restore-auth-rejected"
    } else {
        "p3.6-restore-failed"
    };
    LifecycleError::Vault {
        diagnostic_id,
        category: MatrixIpcErrorCategory::AuthenticationRejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig};
    use crate::app::lifecycle::{
        clear_session_material, persist_session_material, InMemorySessionMaterialVault,
        SessionMaterial,
    };
    use crate::app::store::StoreKeyMaterial;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn alice() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    fn bob() -> AccountIdentity {
        AccountIdentity::new("@bob:example.org", "https://matrix.example.org").unwrap()
    }

    fn sealed_alice(access: &str, refresh: Option<&str>) -> SessionMaterial {
        SessionMaterial::from_matrix_tokens(&alice(), "DEVICEABC", access, refresh).unwrap()
    }

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-p3.6-{tag}-{nanos}"));
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
    fn matrix_session_binding_rejects_mismatches_privately() {
        let material = sealed_alice("syt_bind_access_secret", Some("syr_bind_refresh_secret"));
        let secrets = material.decode_host_secrets().unwrap();

        let err = matrix_session_from_host_secrets(&bob(), &secrets).unwrap_err();
        assert!(matches!(
            err,
            LifecycleError::InvalidTarget {
                diagnostic_id: "p3.6-session-user-mismatch"
            }
        ));
        let text = err.to_string();
        assert!(!text.contains("syt_bind_access_secret"));
        assert!(!text.contains("syr_bind_refresh_secret"));

        let wrong_hs =
            AccountIdentity::new("@alice:example.org", "https://other.example.org").unwrap();
        let err2 = matrix_session_from_host_secrets(&wrong_hs, &secrets).unwrap_err();
        assert!(matches!(
            err2,
            LifecycleError::InvalidTarget {
                diagnostic_id: "p3.6-session-homeserver-mismatch"
            }
        ));
    }

    #[test]
    fn vault_probe_and_missing_material() {
        let vault = InMemorySessionMaterialVault::new();
        assert!(!has_persisted_session(&vault, &alice()).unwrap());
        let material = sealed_alice("syt_probe_token", None);
        persist_session_material(&vault, &alice(), &material).unwrap();
        assert!(has_persisted_session(&vault, &alice()).unwrap());
        assert!(!has_persisted_session(&vault, &bob()).unwrap());
        assert!(clear_session_material(&vault, &alice()).unwrap());
        assert!(!has_persisted_session(&vault, &alice()).unwrap());
    }

    #[test]
    fn restore_from_vault_installs_session_without_token_leaks() {
        let access = "syt_restore_access_token_value";
        let refresh = "syr_restore_refresh_token_value";
        let vault = InMemorySessionMaterialVault::new();
        let material = sealed_alice(access, Some(refresh));
        persist_session_material(&vault, &alice(), &material).unwrap();

        let root = temp_root("restore");
        let key = StoreKeyMaterial::generate().unwrap();
        let cfg = ClientBuildConfig::product_default(&root, alice(), Some(key)).unwrap();

        let rt = test_runtime();
        let _enter = rt.enter();
        let client = rt
            .block_on(build_unauthenticated_client(&cfg))
            .expect("client open");
        assert!(client.session().is_none());

        let outcome = rt
            .block_on(restore_session_from_vault(&client, &alice(), &vault))
            .expect("restore");
        assert_eq!(outcome.meta.user_id, "@alice:example.org");
        assert_eq!(outcome.meta.device_id, "DEVICEABC");
        assert!(outcome.meta.has_refresh_token);

        let live = client.session().expect("session after restore");
        match live {
            matrix_sdk::authentication::AuthSession::Matrix(m) => {
                assert_eq!(m.meta.user_id.as_str(), "@alice:example.org");
                assert_eq!(m.meta.device_id.as_str(), "DEVICEABC");
                assert_eq!(m.tokens.access_token, access);
                assert_eq!(m.tokens.refresh_token.as_deref(), Some(refresh));
            }
            _ => panic!("expected matrix session"),
        }

        let dbg = format!("{outcome:?}");
        assert!(!dbg.contains(access));
        assert!(!dbg.contains(refresh));
        assert!(!dbg.contains("access_token:"));

        // Second restore on same client must fail without leaking secrets.
        let err = rt
            .block_on(restore_session_from_vault(&client, &alice(), &vault))
            .unwrap_err();
        let err_text = err.to_string();
        assert!(!err_text.contains(access));
        assert!(!err_text.contains(refresh));

        drop(client);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn account_switch_uses_identity_scoped_vault_entries() {
        let vault = InMemorySessionMaterialVault::new();
        let a =
            SessionMaterial::from_matrix_tokens(&alice(), "DEVA", "syt_alice_tok", None).unwrap();
        let b = SessionMaterial::from_matrix_tokens(&bob(), "DEVB", "syt_bob_tok", Some("syr_bob"))
            .unwrap();
        persist_session_material(&vault, &alice(), &a).unwrap();
        persist_session_material(&vault, &bob(), &b).unwrap();

        assert!(has_persisted_session(&vault, &alice()).unwrap());
        assert!(has_persisted_session(&vault, &bob()).unwrap());

        let loaded_a = load_session_material(&vault, &alice())
            .unwrap()
            .unwrap()
            .decode_host_secrets()
            .unwrap();
        assert_eq!(loaded_a.access_token, "syt_alice_tok");
        assert_eq!(loaded_a.device_id, "DEVA");

        let loaded_b = load_session_material(&vault, &bob())
            .unwrap()
            .unwrap()
            .decode_host_secrets()
            .unwrap();
        assert_eq!(loaded_b.access_token, "syt_bob_tok");
        assert_eq!(loaded_b.device_id, "DEVB");
        assert!(loaded_b.has_refresh_token());
    }

    #[test]
    fn missing_material_is_privacy_safe() {
        let vault = InMemorySessionMaterialVault::new();
        let root = temp_root("missing");
        let key = StoreKeyMaterial::generate().unwrap();
        let cfg = ClientBuildConfig::product_default(&root, alice(), Some(key)).unwrap();
        let rt = test_runtime();
        let _enter = rt.enter();
        let client = rt
            .block_on(build_unauthenticated_client(&cfg))
            .expect("client open");
        let err = rt
            .block_on(restore_session_from_vault(&client, &alice(), &vault))
            .unwrap_err();
        assert!(matches!(
            err,
            LifecycleError::Vault {
                diagnostic_id: "p3.6-session-material-missing",
                ..
            }
        ));
        assert!(!err.to_string().contains("token"));
        drop(client);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }
}
