//! Store encryption key material (CSPRNG) and keyring key identifiers.

use super::identity::AccountIdentity;

/// Length of a Matrix store encryption key (bytes). Matches common SQLite store
/// passphrase/key expectations; product never logs these bytes.
pub const STORE_KEY_LEN: usize = 32;

/// Opaque 32-byte store encryption key.
///
/// Best-effort zeroization on drop (without depending on the `zeroize` crate as
/// a direct dep). Never implement `Debug` that prints key bytes.
pub struct StoreKeyMaterial([u8; STORE_KEY_LEN]);

impl StoreKeyMaterial {
    /// Generate a new key with the process CSPRNG (`getrandom`).
    pub fn generate() -> Result<Self, StoreKeyGenError> {
        let mut bytes = [0u8; STORE_KEY_LEN];
        getrandom::fill(&mut bytes).map_err(|_| StoreKeyGenError::EntropyUnavailable)?;
        Ok(Self(bytes))
    }

    /// Construct from exact-length bytes (tests / vault restore).
    pub fn from_bytes(bytes: [u8; STORE_KEY_LEN]) -> Self {
        Self(bytes)
    }

    /// Borrow raw bytes for store open (caller must not log).
    pub fn as_bytes(&self) -> &[u8; STORE_KEY_LEN] {
        &self.0
    }

    /// Constant-time-ish equality for tests (not cryptographic CT guarantee).
    pub fn equals(&self, other: &Self) -> bool {
        // Simple compare; keys are not used as MAC secrets here.
        self.0 == other.0
    }
}

impl Drop for StoreKeyMaterial {
    fn drop(&mut self) {
        for b in &mut self.0 {
            // Volatile write to reduce dead-store elimination of zeroization.
            unsafe {
                std::ptr::write_volatile(b, 0);
            }
        }
    }
}

impl std::fmt::Debug for StoreKeyMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StoreKeyMaterial([REDACTED])")
    }
}

/// CSPRNG failure (no secret content).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKeyGenError {
    EntropyUnavailable,
}

impl std::fmt::Display for StoreKeyGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntropyUnavailable => write!(f, "cryptographic entropy unavailable"),
        }
    }
}

impl std::error::Error for StoreKeyGenError {}

/// Non-secret keyring account identifier for a store encryption key.
///
/// Distinct from session credential account names (`matrix-session`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreKeyId {
    /// Service component for the OS credential store.
    service: String,
    /// Account component — includes stable fingerprint, never the raw key.
    account: String,
}

/// Current Keychain/Secret-Service key identifier revision.
///
/// Bump only when a key-service/account derivation contract changes. The vault
/// migrator copies an existing key forward before ever generating a new one.
pub const STORE_KEY_REVISION: u32 = 1;
/// Revision-one credential service name for Matrix store encryption keys.
pub const STORE_KEY_SERVICE_V1: &str = "com.whylandcreative.synara.desktop.matrix-store-key";
/// Current credential service name (kept as a compatibility alias).
pub const STORE_KEY_SERVICE: &str = STORE_KEY_SERVICE_V1;

impl StoreKeyId {
    /// Derive the current deterministic keyring id from account identity.
    pub fn from_identity(identity: &AccountIdentity) -> Self {
        Self::for_revision(identity, STORE_KEY_REVISION)
            .expect("current StoreKey revision must have a service mapping")
    }

    /// Derive a known historical/current credential id for an explicit revision.
    ///
    /// This intentionally returns `None` for unknown revisions: callers fail
    /// closed rather than guessing a Keychain service/account name.
    pub fn for_revision(identity: &AccountIdentity, revision: u32) -> Option<Self> {
        let service = match revision {
            1 => STORE_KEY_SERVICE_V1,
            _ => return None,
        };
        Some(Self {
            service: service.to_owned(),
            account: format!("store-key:{}", identity.account_dir_segment()),
        })
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn account(&self) -> &str {
        &self.account
    }
}
