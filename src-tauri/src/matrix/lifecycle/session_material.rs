//! OS credential-store vault for Matrix session secrets.
//!
//! The sealed envelope, vault trait, in-memory harness, and persist/load/clear
//! helpers live in `synara-core`. This shell file keeps only the live Keychain
//! / Secret Service adapter.

use synara_core::app::lifecycle::LifecycleError;

pub use synara_core::app::lifecycle::{
    clear_session_material, load_session_material, persist_session_material,
    rotate_persisted_session_tokens, HostMatrixSessionSecrets, InMemorySessionMaterialVault,
    SessionMaterial, SessionMaterialId, SessionMaterialMeta, SessionMaterialVault,
    SESSION_ENVELOPE_VERSION, SESSION_KIND_MATRIX, SESSION_MATERIAL_SERVICE,
};

/// Non-secret service/account refs for the keyring-backed session vault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyringSessionMaterialRefs {
    pub service: String,
    pub account: String,
}

impl KeyringSessionMaterialRefs {
    pub fn from_id(id: &SessionMaterialId) -> Self {
        Self {
            service: id.service().to_owned(),
            account: id.account().to_owned(),
        }
    }
}

/// OS credential-store vault for Matrix session secrets (P3.5).
///
/// - macOS: Keychain via `keyring` apple-native backend
/// - Linux: Secret Service / keyutils via `keyring` linux-native backends
/// - Other platforms: operations return vault-unavailable
///
/// Service name is [`SESSION_MATERIAL_SERVICE`] — distinct from store-key service.
/// Payload is the sealed envelope as a UTF-8 JSON string. Secrets never appear
/// in error messages or `Debug` output.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyringSessionMaterialVault;

impl KeyringSessionMaterialVault {
    pub fn new() -> Self {
        Self
    }

    /// True when this process build targets a supported native secret store.
    pub fn platform_supported() -> bool {
        cfg!(any(target_os = "macos", target_os = "linux"))
    }

    fn entry(id: &SessionMaterialId) -> Result<keyring::Entry, LifecycleError> {
        if !Self::platform_supported() {
            return Err(LifecycleError::Vault {
                diagnostic_id: "p3.5-keyring-unsupported-platform",
                category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
            });
        }
        keyring::Entry::new(id.service(), id.account()).map_err(map_session_keyring_error)
    }
}

impl SessionMaterialVault for KeyringSessionMaterialVault {
    fn get(&self, id: &SessionMaterialId) -> Result<Option<SessionMaterial>, LifecycleError> {
        let entry = Self::entry(id)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(SessionMaterial::from_sealed_blob(secret.into_bytes()))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(map_session_keyring_error(err)),
        }
    }

    fn set(
        &self,
        id: &SessionMaterialId,
        material: &SessionMaterial,
    ) -> Result<(), LifecycleError> {
        let entry = Self::entry(id)?;
        let payload =
            std::str::from_utf8(material.as_bytes()).map_err(|_| LifecycleError::Vault {
                diagnostic_id: "p3.5-session-blob-not-utf8",
                category: crate::matrix::ipc::MatrixIpcErrorCategory::SdkInvariant,
            })?;
        entry
            .set_password(payload)
            .map_err(map_session_keyring_error)
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, LifecycleError> {
        let entry = Self::entry(id)?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(err) => Err(map_session_keyring_error(err)),
        }
    }
}

fn map_session_keyring_error(error: keyring::Error) -> LifecycleError {
    // Privacy: never include the raw keyring message.
    let diagnostic_id = match error {
        keyring::Error::NoEntry => "p3.5-keyring-no-entry",
        keyring::Error::BadEncoding(_) | keyring::Error::TooLong(_, _) => "p3.5-keyring-encoding",
        keyring::Error::Invalid(_, _) => "p3.5-keyring-invalid",
        keyring::Error::Ambiguous(_) => "p3.5-keyring-ambiguous",
        keyring::Error::NoStorageAccess(_) => "p3.5-keyring-no-storage-access",
        keyring::Error::PlatformFailure(_) => "p3.5-keyring-platform-failure",
        _ => "p3.5-keyring-unavailable",
    };
    LifecycleError::Vault {
        diagnostic_id,
        category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
    }
}

#[cfg(test)]
mod keyring_ref_tests {
    use super::*;
    use synara_core::app::store::AccountIdentity;

    #[test]
    fn keyring_refs_use_session_service() {
        let id = SessionMaterialId::from_identity(
            &AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap(),
        );
        let refs = KeyringSessionMaterialRefs::from_id(&id);
        assert_eq!(refs.service, SESSION_MATERIAL_SERVICE);
        assert!(refs.account.starts_with("matrix-session:"));
    }
}
