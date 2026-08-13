//! Cross-signing / identity state projection (P8.4 harness foundation).
//!
//! Pure product view of local cross-signing setup and other-user identity trust.
//! **No private keys, public key material, recovery secrets, or tokens.**
//! Booleans / enums / opaque presence only. No SDK crypto APIs, no dual-backend.

use std::collections::HashMap;

use crate::dto::UserId;

use super::error::CrossSigningError;

/// Soft cap on tracked remote identities (UI/list safety).
pub const MAX_TRACKED_IDENTITIES: usize = 512;

/// Local cross-signing key presence (not key bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocalCrossSigningKeys {
    /// Master signing key published.
    pub has_master: bool,
    /// Self-signing key published.
    pub has_self_signing: bool,
    /// User-signing key published.
    pub has_user_signing: bool,
    /// Local private cross-signing keys available to this session (presence only).
    pub private_keys_cached: bool,
}

impl LocalCrossSigningKeys {
    /// True when all three public cross-signing keys are present.
    pub fn public_complete(self) -> bool {
        self.has_master && self.has_self_signing && self.has_user_signing
    }

    /// True when public set is complete and private keys are cached locally.
    pub fn fully_usable(self) -> bool {
        self.public_complete() && self.private_keys_cached
    }
}

/// Trust projection for another user's identity (no keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityTrust {
    /// No identity / not yet evaluated.
    Unknown,
    /// Identity known but not verified by our user-signing key.
    Unverified,
    /// Identity verified (our user-signing signed their master).
    Verified,
    /// Previously verified identity changed (pin mismatch / TOFU break).
    PinViolation,
}

/// Tracked remote user identity summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteIdentity {
    pub user_id: UserId,
    pub trust: IdentityTrust,
    /// True when master key is published for this user (presence only).
    pub has_master_key: bool,
}

/// Session-generation-stamped cross-signing / identity store.
#[derive(Debug, Default)]
pub struct CrossSigningStore {
    session_generation: u64,
    local_keys: LocalCrossSigningKeys,
    /// Own user id when known (for UI copy); not a secret.
    local_user_id: Option<UserId>,
    by_user: HashMap<UserId, RemoteIdentity>,
}

impl CrossSigningStore {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            local_keys: LocalCrossSigningKeys::default(),
            local_user_id: None,
            by_user: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn local_keys(&self) -> LocalCrossSigningKeys {
        self.local_keys
    }

    pub fn local_user_id(&self) -> Option<&str> {
        self.local_user_id.as_deref()
    }

    pub fn tracked_count(&self) -> usize {
        self.by_user.len()
    }

    pub fn is_empty(&self) -> bool {
        !self.local_keys.public_complete()
            && !self.local_keys.private_keys_cached
            && self.by_user.is_empty()
            && self.local_user_id.is_none()
    }

    fn validate_user_id(user_id: &str) -> Result<(), CrossSigningError> {
        if user_id.is_empty() || !user_id.starts_with('@') || !user_id.contains(':') {
            return Err(CrossSigningError::Invalid {
                diagnostic_id: "p8.4-invalid-user-id",
            });
        }
        Ok(())
    }

    pub fn set_local_user_id(&mut self, user_id: Option<String>) -> Result<(), CrossSigningError> {
        if let Some(ref u) = user_id {
            Self::validate_user_id(u)?;
        }
        self.local_user_id = user_id;
        Ok(())
    }

    /// Replace local cross-signing key presence (host maps SDK → product).
    pub fn set_local_keys(&mut self, keys: LocalCrossSigningKeys) {
        self.local_keys = keys;
    }

    pub fn set_private_keys_cached(&mut self, cached: bool) {
        self.local_keys.private_keys_cached = cached;
    }

    /// Upsert a remote identity trust projection.
    pub fn upsert_remote(&mut self, identity: RemoteIdentity) -> Result<(), CrossSigningError> {
        Self::validate_user_id(&identity.user_id)?;
        if !self.by_user.contains_key(&identity.user_id)
            && self.by_user.len() >= MAX_TRACKED_IDENTITIES
        {
            return Err(CrossSigningError::Invalid {
                diagnostic_id: "p8.4-identity-cap",
            });
        }
        self.by_user.insert(identity.user_id.clone(), identity);
        Ok(())
    }

    pub fn get_remote(&self, user_id: &str) -> Option<&RemoteIdentity> {
        self.by_user.get(user_id)
    }

    pub fn set_trust(
        &mut self,
        user_id: &str,
        trust: IdentityTrust,
    ) -> Result<(), CrossSigningError> {
        let id = self
            .by_user
            .get_mut(user_id)
            .ok_or(CrossSigningError::Invalid {
                diagnostic_id: "p8.4-unknown-user-id",
            })?;
        id.trust = trust;
        Ok(())
    }

    pub fn remove_remote(&mut self, user_id: &str) -> Option<RemoteIdentity> {
        self.by_user.remove(user_id)
    }

    /// Verified first, then pin-violation, then unverified, then unknown; then user_id.
    pub fn list_remote(&self) -> Vec<&RemoteIdentity> {
        let mut v: Vec<_> = self.by_user.values().collect();
        v.sort_by(|a, b| {
            trust_rank(a.trust)
                .cmp(&trust_rank(b.trust))
                .then_with(|| a.user_id.cmp(&b.user_id))
        });
        v
    }

    pub fn verified_count(&self) -> usize {
        self.by_user
            .values()
            .filter(|i| i.trust == IdentityTrust::Verified)
            .count()
    }

    pub fn pin_violation_count(&self) -> usize {
        self.by_user
            .values()
            .filter(|i| i.trust == IdentityTrust::PinViolation)
            .count()
    }

    /// Banner attention: incomplete local setup or pin violations.
    pub fn needs_attention(&self) -> bool {
        !self.local_keys.fully_usable() || self.pin_violation_count() > 0
    }

    pub fn clear_remotes(&mut self) {
        self.by_user.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        *self = Self::new(new_generation);
    }
}

fn trust_rank(trust: IdentityTrust) -> u8 {
    match trust {
        IdentityTrust::Verified => 0,
        IdentityTrust::PinViolation => 1,
        IdentityTrust::Unverified => 2,
        IdentityTrust::Unknown => 3,
    }
}
