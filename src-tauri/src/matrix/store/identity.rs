//! Non-secret stable account identity for path + key isolation.

use std::fmt;

/// Account identity used to derive store directories and keyring key ids.
///
/// Contains only non-secret identifiers (Matrix user id + homeserver URL).
/// Never holds access/refresh tokens or store encryption keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountIdentity {
    user_id: String,
    homeserver_url: String,
}

/// Identity validation failure (privacy-safe; does not echo secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountIdentityError {
    EmptyUserId,
    EmptyHomeserver,
    InvalidUserId,
    InvalidHomeserver,
}

impl fmt::Display for AccountIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyUserId => write!(f, "user id is empty"),
            Self::EmptyHomeserver => write!(f, "homeserver url is empty"),
            Self::InvalidUserId => write!(f, "user id is not a valid Matrix user id shape"),
            Self::InvalidHomeserver => write!(f, "homeserver url is invalid"),
        }
    }
}

impl std::error::Error for AccountIdentityError {}

impl AccountIdentity {
    /// Validate and normalize a product account identity.
    ///
    /// - `user_id` must look like `@localpart:server` (trimmed).
    /// - `homeserver_url` is trimmed; trailing `/` stripped; lowercased scheme/host
    ///   portion is not rewritten beyond trim/slash — full string is used for
    ///   fingerprinting after normalization.
    pub fn new(user_id: &str, homeserver_url: &str) -> Result<Self, AccountIdentityError> {
        let user_id = user_id.trim();
        let homeserver_url = homeserver_url.trim().trim_end_matches('/');

        if user_id.is_empty() {
            return Err(AccountIdentityError::EmptyUserId);
        }
        if homeserver_url.is_empty() {
            return Err(AccountIdentityError::EmptyHomeserver);
        }
        if !is_plausible_user_id(user_id) {
            return Err(AccountIdentityError::InvalidUserId);
        }
        if !is_plausible_homeserver(homeserver_url) {
            return Err(AccountIdentityError::InvalidHomeserver);
        }

        Ok(Self {
            user_id: user_id.to_owned(),
            homeserver_url: homeserver_url.to_owned(),
        })
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn homeserver_url(&self) -> &str {
        &self.homeserver_url
    }

    /// Canonical non-secret string used for fingerprinting (not a path segment).
    pub fn canonical_key(&self) -> String {
        format!("{}|{}", self.user_id, self.homeserver_url)
    }

    /// Path-safe account directory segment: versioned label + collision-resistant digest.
    ///
    /// Format: `v1_{sanitized_localpart}_{sha256_32}` where `sha256_32` is the first
    /// 32 hex digits (128 bits) of SHA-256 over `canonical_key()`. Sanitization is
    /// only for human readability; isolation relies on the digest, not FNV.
    ///
    /// **Version note:** `v1_` prefix allows future derivation changes without
    /// silent path reuse. Real on-disk stores must not migrate silently across
    /// derivation versions without an explicit store migration task.
    pub fn account_dir_segment(&self) -> String {
        let local = self
            .user_id
            .strip_prefix('@')
            .unwrap_or(self.user_id.as_str());
        let local_part = local.split(':').next().unwrap_or(local);
        let sanitized = sanitize_path_label(local_part);
        let fp = sha256_prefix_hex(&self.canonical_key(), 32);
        format!("v1_{sanitized}_{fp}")
    }
}

fn is_plausible_user_id(user_id: &str) -> bool {
    if !user_id.starts_with('@') {
        return false;
    }
    let rest = &user_id[1..];
    let Some((local, server)) = rest.split_once(':') else {
        return false;
    };
    !local.is_empty()
        && !server.is_empty()
        && !local.contains(['/', '\\', '\0'])
        && !server.contains(['/', '\\', '\0', '@'])
}

fn is_plausible_homeserver(url: &str) -> bool {
    if url.contains(['\0', ' ', '\n', '\r', '\t']) {
        return false;
    }
    // Require an explicit scheme for clarity (https://… or http://… for tests).
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && url.len() > "https://".len()
        && !url.contains("..")
}

fn sanitize_path_label(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len().min(48));
    for (i, ch) in raw.chars().enumerate() {
        if i >= 48 {
            break;
        }
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("account");
    }
    out
}

/// First `hex_chars` lowercase hex digits of SHA-256(`input`).
fn sha256_prefix_hex(input: &str, hex_chars: usize) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    let encoded: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    let n = hex_chars.min(encoded.len());
    encoded[..n].to_owned()
}

#[cfg(test)]
pub(super) fn fingerprint_for_test(canonical: &str) -> String {
    sha256_prefix_hex(canonical, 32)
}
