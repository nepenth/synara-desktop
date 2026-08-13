//! OS credential-store vault for Matrix store encryption keys.
//!
//! Production uses [`KeyringStoreKeyVault`] (macOS Keychain / Linux Secret
//! Service through the `keyring` crate). The abstract [`StoreKeyVault`] trait,
//! in-memory harness, and get-or-create policy live in `synara-core`.
//! Missing keys and IO failures must **not** delete on-disk Matrix stores
//! (plan §8.3 — no automatic wipe).

use synara_core::app::store::{
    StoreKeyId, StoreKeyMaterial, StoreKeyVault, StoreKeyVaultError, STORE_KEY_LEN,
};

/// Non-secret service/account refs for the keyring-backed vault.
///
/// Stable naming contract for collision tests and diagnostics (never contains
/// key material). Live IO goes through [`KeyringStoreKeyVault`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyringStoreKeyRefs {
    pub service: String,
    pub account: String,
}

impl KeyringStoreKeyRefs {
    pub fn from_id(id: &StoreKeyId) -> Self {
        Self {
            service: id.service().to_owned(),
            account: id.account().to_owned(),
        }
    }
}

/// OS credential-store vault for Matrix store encryption keys (R0.4 residual).
///
/// - macOS: Keychain via `keyring` apple-native backend
/// - Linux: Secret Service / keyutils via `keyring` linux-native backends
/// - Other platforms: operations return [`StoreKeyVaultError::BackendUnavailable`]
///
/// Keys are stored as lowercase hex (64 chars for 32 bytes). Secrets never
/// appear in error messages or `Debug` output.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeyringStoreKeyVault;

impl KeyringStoreKeyVault {
    pub fn new() -> Self {
        Self
    }

    /// True when this process build targets a supported native secret store.
    pub fn platform_supported() -> bool {
        cfg!(any(target_os = "macos", target_os = "linux"))
    }

    fn entry(id: &StoreKeyId) -> Result<keyring::Entry, StoreKeyVaultError> {
        if !Self::platform_supported() {
            return Err(StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "r0.4-keyring-unsupported-platform",
            });
        }
        keyring::Entry::new(id.service(), id.account()).map_err(map_keyring_error)
    }
}

impl StoreKeyVault for KeyringStoreKeyVault {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        let entry = Self::entry(id)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(decode_store_key_payload(&secret)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(map_keyring_error(err)),
        }
    }

    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        let entry = Self::entry(id)?;
        let payload = encode_store_key_payload(key);
        entry.set_password(&payload).map_err(map_keyring_error)
    }

    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        let entry = Self::entry(id)?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(err) => Err(map_keyring_error(err)),
        }
    }
}

/// Encode key bytes as lowercase hex for credential-store string payloads.
fn encode_store_key_payload(key: &StoreKeyMaterial) -> String {
    let mut out = String::with_capacity(STORE_KEY_LEN * 2);
    for b in key.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Decode a hex payload into store key material (strict length + hex only).
fn decode_store_key_payload(secret: &str) -> Result<StoreKeyMaterial, StoreKeyVaultError> {
    let trimmed = secret.trim();
    if trimmed.len() != STORE_KEY_LEN * 2 {
        return Err(StoreKeyVaultError::CorruptPayload);
    }
    let mut bytes = [0u8; STORE_KEY_LEN];
    for (i, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(chunk).map_err(|_| StoreKeyVaultError::CorruptPayload)?;
        bytes[i] = u8::from_str_radix(hex, 16).map_err(|_| StoreKeyVaultError::CorruptPayload)?;
    }
    Ok(StoreKeyMaterial::from_bytes(bytes))
}

fn map_keyring_error(error: keyring::Error) -> StoreKeyVaultError {
    // Privacy: never include the raw keyring message (may mention paths/service).
    match error {
        keyring::Error::NoEntry => StoreKeyVaultError::NotFound,
        keyring::Error::BadEncoding(_) | keyring::Error::TooLong(_, _) => {
            StoreKeyVaultError::Encoding
        }
        keyring::Error::Invalid(_, _) => StoreKeyVaultError::Encoding,
        keyring::Error::Ambiguous(_) => StoreKeyVaultError::BackendUnavailable {
            diagnostic_id: "r0.4-keyring-ambiguous",
        },
        keyring::Error::NoStorageAccess(_) => StoreKeyVaultError::BackendUnavailable {
            diagnostic_id: "r0.4-keyring-no-storage-access",
        },
        keyring::Error::PlatformFailure(_) => StoreKeyVaultError::BackendUnavailable {
            diagnostic_id: "r0.4-keyring-platform-failure",
        },
        _ => StoreKeyVaultError::BackendUnavailable {
            diagnostic_id: "r0.4-keyring-unavailable",
        },
    }
}

#[cfg(test)]
mod key_payload_tests {
    use super::*;

    #[test]
    fn hex_payload_round_trip() {
        let key = StoreKeyMaterial::generate().unwrap();
        let encoded = encode_store_key_payload(&key);
        assert_eq!(encoded.len(), STORE_KEY_LEN * 2);
        assert!(encoded.chars().all(|c| c.is_ascii_hexdigit()));
        let decoded = decode_store_key_payload(&encoded).unwrap();
        assert!(decoded.equals(&key));
    }

    #[test]
    fn hex_payload_rejects_wrong_length_and_non_hex() {
        assert!(matches!(
            decode_store_key_payload("abcd"),
            Err(StoreKeyVaultError::CorruptPayload)
        ));
        let bad = "g".repeat(STORE_KEY_LEN * 2);
        assert!(matches!(
            decode_store_key_payload(&bad),
            Err(StoreKeyVaultError::CorruptPayload)
        ));
    }

    #[test]
    fn map_no_entry_is_not_found() {
        assert_eq!(
            map_keyring_error(keyring::Error::NoEntry),
            StoreKeyVaultError::NotFound
        );
    }
}
