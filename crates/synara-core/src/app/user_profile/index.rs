//! User profile + ignore list projection (P6.6 harness foundation).
//!
//! Pure index of own profile and peer profiles for product UI. **No avatar
//! bytes** (URI/handle only). **No tokens.** No SDK profile set APIs, no
//! dual-backend.

use std::collections::{BTreeSet, HashMap};

use crate::dto::UserId;

use super::error::UserProfileError;

/// Soft cap on cached peer profiles.
pub const MAX_CACHED_PROFILES: usize = 512;

/// Soft cap on ignore list size.
pub const MAX_IGNORED_USERS: usize = 1024;

/// Soft cap on display name length (chars).
pub const MAX_DISPLAY_NAME_CHARS: usize = 256;

/// Soft cap on avatar URL length (chars).
pub const MAX_AVATAR_URL_CHARS: usize = 2048;

/// Privacy-safe user profile projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    pub user_id: UserId,
    pub display_name: Option<String>,
    /// mxc: or https avatar URI/handle only — never raw bytes.
    pub avatar_url: Option<String>,
}

/// Session-generation-stamped profile + ignore index.
#[derive(Debug, Default)]
pub struct UserProfileIndex {
    session_generation: u64,
    own_user_id: Option<UserId>,
    own_profile: Option<UserProfile>,
    peers: HashMap<UserId, UserProfile>,
    ignored: BTreeSet<UserId>,
}

impl UserProfileIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            own_user_id: None,
            own_profile: None,
            peers: HashMap::new(),
            ignored: BTreeSet::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn own_user_id(&self) -> Option<&str> {
        self.own_user_id.as_deref()
    }

    pub fn own_profile(&self) -> Option<&UserProfile> {
        self.own_profile.as_ref()
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    pub fn ignored_count(&self) -> usize {
        self.ignored.len()
    }

    pub fn is_empty(&self) -> bool {
        self.own_profile.is_none() && self.peers.is_empty() && self.ignored.is_empty()
    }

    fn validate_user(user_id: &str) -> Result<(), UserProfileError> {
        if user_id.is_empty() || !user_id.starts_with('@') {
            return Err(UserProfileError::Invalid {
                diagnostic_id: "p6.6-invalid-user-id",
            });
        }
        Ok(())
    }

    fn validate_profile_fields(profile: &UserProfile) -> Result<(), UserProfileError> {
        Self::validate_user(&profile.user_id)?;
        if let Some(ref n) = profile.display_name {
            if n.chars().count() > MAX_DISPLAY_NAME_CHARS {
                return Err(UserProfileError::Invalid {
                    diagnostic_id: "p6.6-display-name-cap",
                });
            }
        }
        if let Some(ref url) = profile.avatar_url {
            if url.chars().count() > MAX_AVATAR_URL_CHARS {
                return Err(UserProfileError::Invalid {
                    diagnostic_id: "p6.6-avatar-url-cap",
                });
            }
            let lower = url.to_ascii_lowercase();
            // Forbid data: / javascript: — media bytes must not ride profile IPC.
            if lower.starts_with("data:") || lower.starts_with("javascript:") {
                return Err(UserProfileError::Invalid {
                    diagnostic_id: "p6.6-forbidden-avatar-scheme",
                });
            }
        }
        Ok(())
    }

    /// Bind own user id for this session (clean-break account switch uses retire).
    pub fn set_own_user_id(&mut self, user_id: impl Into<String>) -> Result<(), UserProfileError> {
        let user_id = user_id.into().trim().to_owned();
        Self::validate_user(&user_id)?;
        self.own_user_id = Some(user_id);
        Ok(())
    }

    /// Set / replace own profile (host maps SDK → product shape).
    pub fn set_own_profile(&mut self, profile: UserProfile) -> Result<(), UserProfileError> {
        Self::validate_profile_fields(&profile)?;
        if let Some(ref own) = self.own_user_id {
            if &profile.user_id != own {
                return Err(UserProfileError::Invalid {
                    diagnostic_id: "p6.6-own-profile-user-mismatch",
                });
            }
        } else {
            self.own_user_id = Some(profile.user_id.clone());
        }
        self.own_profile = Some(profile);
        Ok(())
    }

    /// Upsert a peer profile cache entry.
    pub fn upsert_peer(&mut self, profile: UserProfile) -> Result<(), UserProfileError> {
        Self::validate_profile_fields(&profile)?;
        if let Some(ref own) = self.own_user_id {
            if &profile.user_id == own {
                return self.set_own_profile(profile);
            }
        }
        if !self.peers.contains_key(&profile.user_id) && self.peers.len() >= MAX_CACHED_PROFILES {
            return Err(UserProfileError::Invalid {
                diagnostic_id: "p6.6-peer-profile-cap",
            });
        }
        self.peers.insert(profile.user_id.clone(), profile);
        Ok(())
    }

    pub fn get(&self, user_id: &str) -> Option<&UserProfile> {
        if self.own_user_id.as_deref() == Some(user_id) {
            return self.own_profile.as_ref();
        }
        self.peers.get(user_id)
    }

    pub fn remove_peer(&mut self, user_id: &str) -> Result<(), UserProfileError> {
        Self::validate_user(user_id)?;
        self.peers.remove(user_id);
        Ok(())
    }

    /// Mark user ignored (idempotent).
    pub fn ignore_user(&mut self, user_id: impl Into<String>) -> Result<(), UserProfileError> {
        let user_id = user_id.into().trim().to_owned();
        Self::validate_user(&user_id)?;
        if let Some(ref own) = self.own_user_id {
            if &user_id == own {
                return Err(UserProfileError::Invalid {
                    diagnostic_id: "p6.6-cannot-ignore-self",
                });
            }
        }
        if !self.ignored.contains(&user_id) && self.ignored.len() >= MAX_IGNORED_USERS {
            return Err(UserProfileError::Invalid {
                diagnostic_id: "p6.6-ignore-list-cap",
            });
        }
        self.ignored.insert(user_id);
        Ok(())
    }

    /// Un-ignore (idempotent).
    pub fn unignore_user(&mut self, user_id: &str) -> Result<(), UserProfileError> {
        Self::validate_user(user_id)?;
        self.ignored.remove(user_id);
        Ok(())
    }

    pub fn is_ignored(&self, user_id: &str) -> bool {
        self.ignored.contains(user_id)
    }

    /// Sorted ignore list for IPC/UI.
    pub fn ignored_users(&self) -> Vec<UserId> {
        self.ignored.iter().cloned().collect()
    }

    /// Replace entire ignore list from host snapshot.
    pub fn set_ignored_users(
        &mut self,
        users: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<(), UserProfileError> {
        let mut set = BTreeSet::new();
        for u in users {
            let uid = u.into().trim().to_owned();
            Self::validate_user(&uid)?;
            if let Some(ref own) = self.own_user_id {
                if &uid == own {
                    return Err(UserProfileError::Invalid {
                        diagnostic_id: "p6.6-cannot-ignore-self",
                    });
                }
            }
            set.insert(uid);
            if set.len() > MAX_IGNORED_USERS {
                return Err(UserProfileError::Invalid {
                    diagnostic_id: "p6.6-ignore-list-cap",
                });
            }
        }
        self.ignored = set;
        Ok(())
    }

    pub fn clear_peers(&mut self) {
        self.peers.clear();
    }

    pub fn clear(&mut self) {
        self.own_user_id = None;
        self.own_profile = None;
        self.peers.clear();
        self.ignored.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}
