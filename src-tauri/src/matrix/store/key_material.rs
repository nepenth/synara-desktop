//! Store encryption key material (CSPRNG) and keyring key identifiers.

use super::AccountIdentity;

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

/// Credential service name for Matrix store encryption keys (native only).
pub const STORE_KEY_SERVICE: &str = "com.whylandcreative.synara.desktop.matrix-store-key";

impl StoreKeyId {
    /// Derive a deterministic keyring id from account identity.
    pub fn from_identity(identity: &AccountIdentity) -> Self {
        let account = format!("store-key:{}", identity.account_dir_segment());
        Self {
            service: STORE_KEY_SERVICE.to_owned(),
            account,
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn account(&self) -> &str {
        &self.account
    }
}
