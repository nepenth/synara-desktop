//! Room profile / alias / directory / join-history / upgrade projection (P6.5 harness).
//!
//! Pure index of room presentation state and upgrade pointers. **No avatar
//! bytes** (URI only). **No tokens.** No SDK room state PUT, no dual-backend.

use std::collections::{BTreeSet, HashMap};

use crate::dto::RoomId;

use super::error::RoomProfileError;

/// Soft cap on cached room profiles.
pub const MAX_CACHED_ROOMS: usize = 4_096;

/// Soft cap on alternate aliases per room.
pub const MAX_ALT_ALIASES: usize = 64;

/// Soft cap on room name length (chars).
pub const MAX_NAME_CHARS: usize = 256;

/// Soft cap on topic length (chars).
pub const MAX_TOPIC_CHARS: usize = 2_048;

/// Soft cap on avatar URL length (chars).
pub const MAX_AVATAR_URL_CHARS: usize = 2_048;

/// Soft cap on alias string length (chars).
pub const MAX_ALIAS_CHARS: usize = 255;

/// Join rule projection (product enum; not an SDK type).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinRule {
    Public,
    Knock,
    Invite,
    Restricted,
    KnockRestricted,
    Private,
}

impl JoinRule {
    pub const ALL: &'static [JoinRule] = &[
        Self::Public,
        Self::Knock,
        Self::Invite,
        Self::Restricted,
        Self::KnockRestricted,
        Self::Private,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Knock => "knock",
            Self::Invite => "invite",
            Self::Restricted => "restricted",
            Self::KnockRestricted => "knock_restricted",
            Self::Private => "private",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Self::Public),
            "knock" => Some(Self::Knock),
            "invite" => Some(Self::Invite),
            "restricted" => Some(Self::Restricted),
            "knock_restricted" => Some(Self::KnockRestricted),
            "private" => Some(Self::Private),
            _ => None,
        }
    }
}

/// History visibility projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HistoryVisibility {
    Invited,
    Joined,
    Shared,
    WorldReadable,
}

impl HistoryVisibility {
    pub const ALL: &'static [HistoryVisibility] = &[
        Self::Invited,
        Self::Joined,
        Self::Shared,
        Self::WorldReadable,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Invited => "invited",
            Self::Joined => "joined",
            Self::Shared => "shared",
            Self::WorldReadable => "world_readable",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "invited" => Some(Self::Invited),
            "joined" => Some(Self::Joined),
            "shared" => Some(Self::Shared),
            "world_readable" => Some(Self::WorldReadable),
            _ => None,
        }
    }
}

/// Room directory visibility (public directory listing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectoryVisibility {
    Public,
    Private,
}

impl DirectoryVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Self::Public),
            "private" => Some(Self::Private),
            _ => None,
        }
    }
}

/// Room presentation + policy + upgrade projection for product UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomProfile {
    pub room_id: RoomId,
    pub name: Option<String>,
    pub topic: Option<String>,
    /// mxc: or https avatar URI/handle only — never raw bytes.
    pub avatar_url: Option<String>,
    pub canonical_alias: Option<String>,
    pub alt_aliases: Vec<String>,
    pub join_rule: Option<JoinRule>,
    pub history_visibility: Option<HistoryVisibility>,
    pub directory_visibility: Option<DirectoryVisibility>,
    /// Predecessor room after an upgrade chain step (this room was created from).
    pub predecessor_room_id: Option<RoomId>,
    /// Tombstone successor (this room was upgraded to).
    pub successor_room_id: Option<RoomId>,
}

/// Session-generation-stamped room profile index.
#[derive(Debug, Default)]
pub struct RoomProfileIndex {
    session_generation: u64,
    rooms: HashMap<RoomId, RoomProfile>,
    /// alias → room_id reverse lookup (canonical + alt).
    alias_index: HashMap<String, RoomId>,
}

impl RoomProfileIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            rooms: HashMap::new(),
            alias_index: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.rooms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty()
    }

    pub fn get(&self, room_id: &str) -> Option<&RoomProfile> {
        self.rooms.get(room_id)
    }

    /// Resolve a room id from a canonical or alt alias.
    pub fn room_id_for_alias(&self, alias: &str) -> Option<&str> {
        self.alias_index.get(alias).map(String::as_str)
    }

    /// Upsert full room profile (host maps SDK state → product shape).
    pub fn upsert(&mut self, profile: RoomProfile) -> Result<(), RoomProfileError> {
        validate_profile(&profile)?;
        if !self.rooms.contains_key(&profile.room_id) && self.rooms.len() >= MAX_CACHED_ROOMS {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-room-cap",
            });
        }

        // Drop prior alias mappings for this room.
        if let Some(prev) = self.rooms.get(&profile.room_id).cloned() {
            self.drop_aliases_for(&prev);
        }

        // Register new aliases; conflict if another room owns the alias.
        self.register_aliases(&profile)?;

        self.rooms.insert(profile.room_id.clone(), profile);
        Ok(())
    }

    /// Patch name only.
    pub fn set_name(
        &mut self,
        room_id: &str,
        name: Option<String>,
    ) -> Result<(), RoomProfileError> {
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        if let Some(ref n) = name {
            if n.chars().count() > MAX_NAME_CHARS {
                return Err(RoomProfileError::Invalid {
                    diagnostic_id: "p6.5-name-cap",
                });
            }
        }
        p.name = name;
        Ok(())
    }

    /// Patch topic only.
    pub fn set_topic(
        &mut self,
        room_id: &str,
        topic: Option<String>,
    ) -> Result<(), RoomProfileError> {
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        if let Some(ref t) = topic {
            if t.chars().count() > MAX_TOPIC_CHARS {
                return Err(RoomProfileError::Invalid {
                    diagnostic_id: "p6.5-topic-cap",
                });
            }
        }
        p.topic = topic;
        Ok(())
    }

    /// Set canonical + alt aliases atomically for a room.
    pub fn set_aliases(
        &mut self,
        room_id: &str,
        canonical: Option<String>,
        alt: Vec<String>,
    ) -> Result<(), RoomProfileError> {
        if !self.rooms.contains_key(room_id) {
            return Err(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            });
        }
        if alt.len() > MAX_ALT_ALIASES {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-alt-alias-cap",
            });
        }
        if let Some(ref c) = canonical {
            validate_alias(c)?;
        }
        for a in &alt {
            validate_alias(a)?;
        }

        // Build temporary profile view for alias registration.
        let room_id_owned = room_id.to_owned();
        let existing =
            self.rooms
                .get(&room_id_owned)
                .cloned()
                .ok_or(RoomProfileError::NotFound {
                    diagnostic_id: "p6.5-room-not-found",
                })?;
        self.drop_aliases_for(&existing);

        let trial = RoomProfile {
            room_id: room_id_owned.clone(),
            name: existing.name.clone(),
            topic: existing.topic.clone(),
            avatar_url: existing.avatar_url.clone(),
            canonical_alias: canonical.clone(),
            alt_aliases: alt.clone(),
            join_rule: existing.join_rule,
            history_visibility: existing.history_visibility,
            directory_visibility: existing.directory_visibility,
            predecessor_room_id: existing.predecessor_room_id.clone(),
            successor_room_id: existing.successor_room_id.clone(),
        };

        if let Err(e) = self.register_aliases(&trial) {
            // Restore previous aliases on conflict.
            let _ = self.register_aliases(&existing);
            return Err(e);
        }

        let p = self.rooms.get_mut(&room_id_owned).expect("checked");
        p.canonical_alias = canonical;
        p.alt_aliases = alt;
        Ok(())
    }

    pub fn set_join_rule(&mut self, room_id: &str, rule: JoinRule) -> Result<(), RoomProfileError> {
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        p.join_rule = Some(rule);
        Ok(())
    }

    pub fn set_history_visibility(
        &mut self,
        room_id: &str,
        visibility: HistoryVisibility,
    ) -> Result<(), RoomProfileError> {
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        p.history_visibility = Some(visibility);
        Ok(())
    }

    pub fn set_directory_visibility(
        &mut self,
        room_id: &str,
        visibility: DirectoryVisibility,
    ) -> Result<(), RoomProfileError> {
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        p.directory_visibility = Some(visibility);
        Ok(())
    }

    /// Record tombstone / upgrade successor for a room.
    pub fn set_successor(
        &mut self,
        room_id: &str,
        successor_room_id: Option<String>,
    ) -> Result<(), RoomProfileError> {
        if let Some(ref s) = successor_room_id {
            validate_room_id(s)?;
            if s == room_id {
                return Err(RoomProfileError::Invalid {
                    diagnostic_id: "p6.5-self-successor",
                });
            }
        }
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        p.successor_room_id = successor_room_id;
        Ok(())
    }

    /// Record predecessor room id (create-room with predecessor).
    pub fn set_predecessor(
        &mut self,
        room_id: &str,
        predecessor_room_id: Option<String>,
    ) -> Result<(), RoomProfileError> {
        if let Some(ref s) = predecessor_room_id {
            validate_room_id(s)?;
            if s == room_id {
                return Err(RoomProfileError::Invalid {
                    diagnostic_id: "p6.5-self-predecessor",
                });
            }
        }
        let p = self
            .rooms
            .get_mut(room_id)
            .ok_or(RoomProfileError::NotFound {
                diagnostic_id: "p6.5-room-not-found",
            })?;
        p.predecessor_room_id = predecessor_room_id;
        Ok(())
    }

    /// Follow successor chain (bounded) for upgrade UX.
    pub fn upgrade_chain(&self, room_id: &str, max_hops: usize) -> Vec<RoomId> {
        let mut out = Vec::new();
        let mut cur = room_id.to_owned();
        let mut seen = BTreeSet::new();
        seen.insert(cur.clone());
        for _ in 0..max_hops {
            let Some(p) = self.rooms.get(&cur) else {
                break;
            };
            let Some(next) = p.successor_room_id.clone() else {
                break;
            };
            if !seen.insert(next.clone()) {
                break;
            }
            out.push(next.clone());
            cur = next;
        }
        out
    }

    pub fn remove(&mut self, room_id: &str) -> bool {
        if let Some(prev) = self.rooms.remove(room_id) {
            self.drop_aliases_for(&prev);
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.rooms.clear();
        self.alias_index.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }

    fn drop_aliases_for(&mut self, profile: &RoomProfile) {
        if let Some(ref c) = profile.canonical_alias {
            if self.alias_index.get(c).map(String::as_str) == Some(profile.room_id.as_str()) {
                self.alias_index.remove(c);
            }
        }
        for a in &profile.alt_aliases {
            if self.alias_index.get(a).map(String::as_str) == Some(profile.room_id.as_str()) {
                self.alias_index.remove(a);
            }
        }
    }

    fn register_aliases(&mut self, profile: &RoomProfile) -> Result<(), RoomProfileError> {
        let mut pending: Vec<String> = Vec::new();
        if let Some(ref c) = profile.canonical_alias {
            pending.push(c.clone());
        }
        for a in &profile.alt_aliases {
            pending.push(a.clone());
        }
        // Dedup within room.
        pending.sort();
        pending.dedup();

        for alias in &pending {
            if let Some(owner) = self.alias_index.get(alias) {
                if owner != &profile.room_id {
                    return Err(RoomProfileError::Invalid {
                        diagnostic_id: "p6.5-alias-conflict",
                    });
                }
            }
        }
        for alias in pending {
            self.alias_index.insert(alias, profile.room_id.clone());
        }
        Ok(())
    }
}

fn validate_room_id(room_id: &str) -> Result<(), RoomProfileError> {
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(RoomProfileError::Invalid {
            diagnostic_id: "p6.5-invalid-room-id",
        });
    }
    Ok(())
}

fn validate_alias(alias: &str) -> Result<(), RoomProfileError> {
    if alias.is_empty() || !alias.starts_with('#') {
        return Err(RoomProfileError::Invalid {
            diagnostic_id: "p6.5-invalid-alias",
        });
    }
    if alias.chars().count() > MAX_ALIAS_CHARS {
        return Err(RoomProfileError::Invalid {
            diagnostic_id: "p6.5-alias-cap",
        });
    }
    if alias.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(RoomProfileError::Invalid {
            diagnostic_id: "p6.5-invalid-alias",
        });
    }
    Ok(())
}

fn validate_profile(profile: &RoomProfile) -> Result<(), RoomProfileError> {
    validate_room_id(&profile.room_id)?;
    if let Some(ref n) = profile.name {
        if n.chars().count() > MAX_NAME_CHARS {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-name-cap",
            });
        }
    }
    if let Some(ref t) = profile.topic {
        if t.chars().count() > MAX_TOPIC_CHARS {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-topic-cap",
            });
        }
    }
    if let Some(ref url) = profile.avatar_url {
        if url.chars().count() > MAX_AVATAR_URL_CHARS {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-avatar-url-cap",
            });
        }
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("data:") || lower.starts_with("javascript:") {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-forbidden-avatar-scheme",
            });
        }
    }
    if let Some(ref c) = profile.canonical_alias {
        validate_alias(c)?;
    }
    if profile.alt_aliases.len() > MAX_ALT_ALIASES {
        return Err(RoomProfileError::Invalid {
            diagnostic_id: "p6.5-alt-alias-cap",
        });
    }
    for a in &profile.alt_aliases {
        validate_alias(a)?;
    }
    if let Some(ref p) = profile.predecessor_room_id {
        validate_room_id(p)?;
        if p == &profile.room_id {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-self-predecessor",
            });
        }
    }
    if let Some(ref s) = profile.successor_room_id {
        validate_room_id(s)?;
        if s == &profile.room_id {
            return Err(RoomProfileError::Invalid {
                diagnostic_id: "p6.5-self-successor",
            });
        }
    }
    Ok(())
}
