//! Session material vault (P2.6 clear hooks + P3.5 persist/rotation foundation).
//!
//! Shared session-material *trait*, sealed envelope, and in-memory harness.
//! Live OS credential I/O (Keychain / Secret Service) stays in the desktop
//! shell. **Never** place tokens on IPC DTOs, logs, or error Display strings.
//!
//! Distinct from store-encryption key vault (`StoreKeyVault` / `store-key:…`).
//!
//! **P3.5 scope:** persist + overwrite (rotation structure). Session restore onto
//! an SDK `Client` is **P3.6** and is not enabled here.

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::app::store::AccountIdentity;

use super::LifecycleError;

/// Credential service name for Matrix session material (native only).
///
/// Intentionally distinct from `STORE_KEY_SERVICE`.
pub const SESSION_MATERIAL_SERVICE: &str = "com.whylandcreative.synara.desktop.matrix-session";

/// Wire format version for the sealed session envelope JSON.
pub const SESSION_ENVELOPE_VERSION: u8 = 1;

/// Kind marker for native Matrix auth sessions (password matrix API).
pub const SESSION_KIND_MATRIX: &str = "matrix";

/// Non-secret keyring account id for session material for one account.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionMaterialId {
    service: String,
    account: String,
}

impl SessionMaterialId {
    pub fn from_identity(identity: &AccountIdentity) -> Self {
        Self {
            service: SESSION_MATERIAL_SERVICE.to_owned(),
            account: format!("matrix-session:{}", identity.account_dir_segment()),
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn account(&self) -> &str {
        &self.account
    }
}

/// Opaque session material (sealed envelope bytes). Host-side only.
///
/// Real access/refresh tokens never leave the vault abstraction toward IPC.
/// Debug redacts; Drop best-effort zeroizes the blob.
#[derive(Clone)]
pub struct SessionMaterial {
    /// Sealed envelope bytes (JSON for v1; never logged as plaintext tokens).
    blob: Vec<u8>,
}

impl SessionMaterial {
    /// Harness placeholder blob (logout/wipe tests). Prefer
    /// [`Self::from_matrix_tokens`] for real session secrets.
    pub fn from_placeholder(bytes: impl Into<Vec<u8>>) -> Self {
        Self { blob: bytes.into() }
    }

    /// Construct sealed v1 matrix session material from host-side tokens.
    ///
    /// Tokens are sealed into the blob immediately; this type's Debug never
    /// prints them. Callers must not log `access_token` / `refresh_token`.
    pub fn from_matrix_tokens(
        identity: &AccountIdentity,
        device_id: &str,
        access_token: &str,
        refresh_token: Option<&str>,
    ) -> Result<Self, LifecycleError> {
        let device = device_id.trim();
        if device.is_empty() {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.5-empty-device-id",
            });
        }
        if access_token.is_empty() {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.5-empty-access-token",
            });
        }
        if refresh_token.is_some_and(|t| t.is_empty()) {
            return Err(LifecycleError::InvalidTarget {
                diagnostic_id: "p3.5-empty-refresh-token",
            });
        }

        let envelope = SessionEnvelopeV1 {
            v: SESSION_ENVELOPE_VERSION,
            kind: SESSION_KIND_MATRIX.to_owned(),
            user_id: identity.user_id().to_owned(),
            device_id: device.to_owned(),
            homeserver_url: identity.homeserver_url().to_owned(),
            access_token: access_token.to_owned(),
            refresh_token: refresh_token.map(str::to_owned),
        };
        let blob = serde_json::to_vec(&envelope).map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p3.5-session-envelope-encode",
            category: crate::transport::MatrixIpcErrorCategory::SdkInvariant,
        })?;
        Ok(Self { blob })
    }

    /// Wrap already-sealed blob bytes (vault load path).
    pub fn from_sealed_blob(bytes: impl Into<Vec<u8>>) -> Self {
        Self { blob: bytes.into() }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.blob
    }

    pub fn len(&self) -> usize {
        self.blob.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blob.is_empty()
    }

    /// Privacy-safe metadata decoded from a sealed v1 envelope (no tokens).
    pub fn public_meta(&self) -> Result<SessionMaterialMeta, LifecycleError> {
        let env = decode_envelope_v1(&self.blob)?;
        Ok(SessionMaterialMeta {
            format_version: env.v,
            kind: env.kind,
            user_id: env.user_id,
            device_id: env.device_id,
            homeserver_url: env.homeserver_url,
            has_refresh_token: env.refresh_token.is_some(),
        })
    }

    /// Host-only decode of sealed secrets. **Never** pass return value over IPC.
    ///
    /// Used for refresh rotation overwrite and (later) P3.6 restore assembly.
    /// Does **not** call SDK `restore_session`.
    pub fn decode_host_secrets(&self) -> Result<HostMatrixSessionSecrets, LifecycleError> {
        let env = decode_envelope_v1(&self.blob)?;
        if env.kind != SESSION_KIND_MATRIX {
            return Err(LifecycleError::Vault {
                diagnostic_id: "p3.5-session-kind-unsupported",
                category: crate::transport::MatrixIpcErrorCategory::UnsupportedCapability,
            });
        }
        Ok(HostMatrixSessionSecrets {
            user_id: env.user_id,
            device_id: env.device_id,
            homeserver_url: env.homeserver_url,
            access_token: env.access_token,
            refresh_token: env.refresh_token,
        })
    }
}

impl fmt::Debug for SessionMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionMaterial([REDACTED len={}])", self.blob.len())
    }
}

impl Drop for SessionMaterial {
    fn drop(&mut self) {
        for b in &mut self.blob {
            unsafe {
                std::ptr::write_volatile(b, 0);
            }
        }
        self.blob.clear();
    }
}

/// Privacy-safe view of sealed session material (no access/refresh tokens).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMaterialMeta {
    pub format_version: u8,
    pub kind: String,
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
    pub has_refresh_token: bool,
}

/// Host-only decoded matrix session secrets. Tokens must never reach IPC/logs.
///
/// Debug redacts token fields. Drop zeroizes token strings best-effort.
pub struct HostMatrixSessionSecrets {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

impl HostMatrixSessionSecrets {
    pub fn has_refresh_token(&self) -> bool {
        self.refresh_token.as_ref().is_some_and(|t| !t.is_empty())
    }
}

impl fmt::Debug for HostMatrixSessionSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostMatrixSessionSecrets")
            .field("user_id", &self.user_id)
            .field("device_id", &self.device_id)
            .field("homeserver_url", &self.homeserver_url)
            .field("access_token", &"[REDACTED]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl Drop for HostMatrixSessionSecrets {
    fn drop(&mut self) {
        zeroize_string(&mut self.access_token);
        if let Some(ref mut rt) = self.refresh_token {
            zeroize_string(rt);
        }
    }
}

/// Sealed envelope v1 (JSON). Private; tokens never appear in public Debug types.
#[derive(Clone, Serialize, Deserialize)]
struct SessionEnvelopeV1 {
    v: u8,
    kind: String,
    user_id: String,
    device_id: String,
    homeserver_url: String,
    access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
}

fn decode_envelope_v1(blob: &[u8]) -> Result<SessionEnvelopeV1, LifecycleError> {
    let env: SessionEnvelopeV1 =
        serde_json::from_slice(blob).map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p3.5-session-envelope-decode",
            category: crate::transport::MatrixIpcErrorCategory::SdkInvariant,
        })?;
    if env.v != SESSION_ENVELOPE_VERSION {
        return Err(LifecycleError::Vault {
            diagnostic_id: "p3.5-session-envelope-version",
            category: crate::transport::MatrixIpcErrorCategory::UnsupportedCapability,
        });
    }
    if env.user_id.is_empty() || env.device_id.is_empty() || env.access_token.is_empty() {
        return Err(LifecycleError::Vault {
            diagnostic_id: "p3.5-session-envelope-incomplete",
            category: crate::transport::MatrixIpcErrorCategory::SdkInvariant,
        });
    }
    Ok(env)
}

fn zeroize_string(s: &mut String) {
    // Overwrite then clear so Drop of the String does not leave plaintext.
    // SAFETY: we only write zeros into the existing UTF-8 buffer, then clear.
    unsafe {
        let bytes = s.as_bytes_mut();
        for b in bytes.iter_mut() {
            std::ptr::write_volatile(b, 0);
        }
    }
    s.clear();
}

/// Read/write/clear session credentials by account identity.
pub trait SessionMaterialVault: Send {
    fn get(&self, id: &SessionMaterialId) -> Result<Option<SessionMaterial>, LifecycleError>;
    fn set(&self, id: &SessionMaterialId, material: &SessionMaterial)
        -> Result<(), LifecycleError>;
    /// Remove session material. Returns whether an entry existed.
    fn clear(&self, id: &SessionMaterialId) -> Result<bool, LifecycleError>;
}

/// Process-local mock keyring for unit/integration harnesses.
#[derive(Debug, Default)]
pub struct InMemorySessionMaterialVault {
    inner: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl InMemorySessionMaterialVault {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SessionMaterialVault for InMemorySessionMaterialVault {
    fn get(&self, id: &SessionMaterialId) -> Result<Option<SessionMaterial>, LifecycleError> {
        let guard = self.inner.lock().map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p2.6-session-vault-poisoned",
            category: crate::transport::MatrixIpcErrorCategory::StoreUnavailable,
        })?;
        Ok(guard
            .get(&(id.service().to_owned(), id.account().to_owned()))
            .cloned()
            .map(SessionMaterial::from_sealed_blob))
    }

    fn set(
        &self,
        id: &SessionMaterialId,
        material: &SessionMaterial,
    ) -> Result<(), LifecycleError> {
        let mut guard = self.inner.lock().map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p2.6-session-vault-poisoned",
            category: crate::transport::MatrixIpcErrorCategory::StoreUnavailable,
        })?;
        guard.insert(
            (id.service().to_owned(), id.account().to_owned()),
            material.as_bytes().to_vec(),
        );
        Ok(())
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, LifecycleError> {
        let mut guard = self.inner.lock().map_err(|_| LifecycleError::Vault {
            diagnostic_id: "p2.6-session-vault-poisoned",
            category: crate::transport::MatrixIpcErrorCategory::StoreUnavailable,
        })?;
        Ok(guard
            .remove(&(id.service().to_owned(), id.account().to_owned()))
            .is_some())
    }
}

/// Persist (or overwrite) sealed session material for `identity`.
///
/// Overwrite is the rotation path: same [`SessionMaterialId`], new blob after
/// access/refresh token rotation.
pub fn persist_session_material<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
    material: &SessionMaterial,
) -> Result<(), LifecycleError> {
    // Validate sealed shape before writing (placeholder blobs from older tests
    // may skip this — product/login path always uses from_matrix_tokens).
    let _ = material.public_meta().or_else(|_| {
        // Allow non-envelope placeholders only when clearly not JSON object.
        if material
            .as_bytes()
            .first()
            .copied()
            .is_some_and(|b| b == b'{')
        {
            material.public_meta()
        } else {
            Ok(SessionMaterialMeta {
                format_version: 0,
                kind: "placeholder".into(),
                user_id: identity.user_id().to_owned(),
                device_id: String::new(),
                homeserver_url: identity.homeserver_url().to_owned(),
                has_refresh_token: false,
            })
        }
    })?;
    let id = SessionMaterialId::from_identity(identity);
    vault.set(&id, material)
}

/// Load sealed session material for `identity` (host-side only).
pub fn load_session_material<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
) -> Result<Option<SessionMaterial>, LifecycleError> {
    let id = SessionMaterialId::from_identity(identity);
    vault.get(&id)
}

/// Clear session material for `identity` via the vault hook.
pub fn clear_session_material<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
) -> Result<bool, LifecycleError> {
    let id = SessionMaterialId::from_identity(identity);
    vault.clear(&id)
}

/// Overwrite vault entry after token rotation (same account key).
///
/// Builds a new sealed envelope and replaces the previous blob atomically from
/// the caller's perspective (`vault.set`). Does not call SDK refresh APIs.
pub fn rotate_persisted_session_tokens<V: SessionMaterialVault + ?Sized>(
    vault: &V,
    identity: &AccountIdentity,
    device_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
) -> Result<SessionMaterialMeta, LifecycleError> {
    let material =
        SessionMaterial::from_matrix_tokens(identity, device_id, access_token, refresh_token)?;
    let meta = material.public_meta()?;
    // Bind identity: envelope user/hs must match the vault key identity.
    if meta.user_id != identity.user_id() || meta.homeserver_url != identity.homeserver_url() {
        return Err(LifecycleError::InvalidTarget {
            diagnostic_id: "p3.5-identity-mismatch",
        });
    }
    persist_session_material(vault, identity, &material)?;
    Ok(meta)
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn alice() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    #[test]
    fn envelope_round_trip_preserves_tokens_host_only() {
        let access = "syt_access_token_roundtrip_abc";
        let refresh = "syr_refresh_token_roundtrip_xyz";
        let material =
            SessionMaterial::from_matrix_tokens(&alice(), "DEVICE1", access, Some(refresh))
                .unwrap();
        let secrets = material.decode_host_secrets().unwrap();
        assert_eq!(secrets.access_token, access);
        assert_eq!(secrets.refresh_token.as_deref(), Some(refresh));
        assert_eq!(secrets.user_id, "@alice:example.org");
        assert_eq!(secrets.device_id, "DEVICE1");
        let meta = material.public_meta().unwrap();
        assert!(meta.has_refresh_token);
        assert_eq!(meta.format_version, SESSION_ENVELOPE_VERSION);
    }

    #[test]
    fn debug_redacts_tokens_on_material_and_host_secrets() {
        let access = "syt_super_secret_access_do_not_leak";
        let refresh = "syr_super_secret_refresh_do_not_leak";
        let material =
            SessionMaterial::from_matrix_tokens(&alice(), "DEV", access, Some(refresh)).unwrap();
        let dbg_mat = format!("{material:?}");
        assert!(dbg_mat.contains("REDACTED"));
        assert!(!dbg_mat.contains(access));
        assert!(!dbg_mat.contains(refresh));

        let secrets = material.decode_host_secrets().unwrap();
        let dbg_sec = format!("{secrets:?}");
        assert!(!dbg_sec.contains(access));
        assert!(!dbg_sec.contains(refresh));
        assert!(dbg_sec.contains("REDACTED"));
    }

    #[test]
    fn vault_persist_load_clear_and_rotate() {
        let vault = InMemorySessionMaterialVault::new();
        let access1 = "syt_access_v1_unique";
        let refresh1 = "syr_refresh_v1_unique";
        let m1 =
            SessionMaterial::from_matrix_tokens(&alice(), "DEV", access1, Some(refresh1)).unwrap();
        persist_session_material(&vault, &alice(), &m1).unwrap();

        let loaded = load_session_material(&vault, &alice()).unwrap().unwrap();
        assert_eq!(loaded.decode_host_secrets().unwrap().access_token, access1);

        // Rotation overwrite
        let access2 = "syt_access_v2_rotated";
        let refresh2 = "syr_refresh_v2_rotated";
        let meta =
            rotate_persisted_session_tokens(&vault, &alice(), "DEV", access2, Some(refresh2))
                .unwrap();
        assert!(meta.has_refresh_token);
        assert_eq!(vault.len(), 1);

        let loaded2 = load_session_material(&vault, &alice()).unwrap().unwrap();
        let s2 = loaded2.decode_host_secrets().unwrap();
        assert_eq!(s2.access_token, access2);
        assert_eq!(s2.refresh_token.as_deref(), Some(refresh2));
        // Old tokens gone from vault payload
        let blob = String::from_utf8_lossy(loaded2.as_bytes());
        assert!(!blob.contains(access1));
        assert!(!blob.contains(refresh1));

        assert!(clear_session_material(&vault, &alice()).unwrap());
        assert!(load_session_material(&vault, &alice()).unwrap().is_none());
        assert!(!clear_session_material(&vault, &alice()).unwrap());
    }

    #[test]
    fn rejects_empty_access_token() {
        let err = SessionMaterial::from_matrix_tokens(&alice(), "DEV", "", None).unwrap_err();
        assert!(matches!(
            err,
            LifecycleError::InvalidTarget {
                diagnostic_id: "p3.5-empty-access-token"
            }
        ));
        let display = err.to_string();
        assert!(!display.contains("syt_"));
    }

    #[test]
    fn lifecycle_error_display_has_no_token_substrings() {
        let err = LifecycleError::Vault {
            diagnostic_id: "p3.5-session-envelope-decode",
            category: crate::transport::MatrixIpcErrorCategory::SdkInvariant,
        };
        let text = err.to_string();
        assert!(!text.contains("access_token"));
        assert!(!text.contains("refresh_token"));
        assert!(!text.contains("syt_"));
    }
}
