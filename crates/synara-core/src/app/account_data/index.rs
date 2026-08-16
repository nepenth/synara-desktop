//! Account-data service / codec foundation (P6.7 harness).
//!
//! Pure index of global + room account-data events by type. Content is stored
//! as allowlisted string fields only (no raw JSON dumps, no tokens). Host maps
//! SDK account data → this shape.

use std::collections::{BTreeMap, HashMap};

use crate::dto::RoomId;

use super::error::AccountDataError;

/// Soft caps.
pub const MAX_GLOBAL_TYPES: usize = 512;
pub const MAX_ROOM_TYPES: usize = 256;
pub const MAX_ROOMS_WITH_ACCOUNT_DATA: usize = 4_096;
pub const MAX_CONTENT_FIELDS: usize = 32;
pub const MAX_KEY_LEN: usize = 128;
pub const MAX_VALUE_LEN: usize = 4_096;

/// Well-known account-data type constants (product).
pub const TYPE_FULLY_READ: &str = "m.fully_read";
pub const TYPE_PUSH_RULES: &str = "m.push_rules";
pub const TYPE_DIRECT: &str = "m.direct";
pub const TYPE_IGNORED_USER_LIST: &str = "m.ignored_user_list";
pub const TYPE_TAG: &str = "m.tag";

/// One account-data event projection (string fields only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDataEntry {
    /// Matrix account-data type (e.g. `m.fully_read`).
    pub event_type: String,
    /// `None` = global; `Some(room)` = room account data.
    pub room_id: Option<RoomId>,
    /// Allowlisted short string fields from content (already host-filtered).
    pub fields: BTreeMap<String, String>,
}

impl AccountDataEntry {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }
}

/// Session-generation-stamped account-data index.
#[derive(Debug, Default)]
pub struct AccountDataIndex {
    session_generation: u64,
    /// type → entry
    global: HashMap<String, AccountDataEntry>,
    /// (room_id, type) → entry
    room: HashMap<(RoomId, String), AccountDataEntry>,
}

impl AccountDataIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            global: HashMap::new(),
            room: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn global_len(&self) -> usize {
        self.global.len()
    }

    pub fn room_len(&self) -> usize {
        self.room.len()
    }

    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.room.is_empty()
    }

    /// Upsert one account-data entry (global or room).
    pub fn upsert(&mut self, entry: AccountDataEntry) -> Result<(), AccountDataError> {
        validate_event_type(&entry.event_type)?;
        if let Some(room) = &entry.room_id {
            validate_room(room)?;
        }
        validate_fields(&entry.fields)?;

        // Cap content fields.
        if entry.fields.len() > MAX_CONTENT_FIELDS {
            return Err(AccountDataError::Invalid {
                diagnostic_id: "p6.7-content-field-cap",
            });
        }

        match &entry.room_id {
            None => {
                if !self.global.contains_key(&entry.event_type)
                    && self.global.len() >= MAX_GLOBAL_TYPES
                {
                    return Err(AccountDataError::Invalid {
                        diagnostic_id: "p6.7-global-type-cap",
                    });
                }
                self.global.insert(entry.event_type.clone(), entry);
            }
            Some(room_id) => {
                let rooms = self
                    .room
                    .keys()
                    .map(|(r, _)| r.clone())
                    .collect::<std::collections::HashSet<_>>();
                let key = (room_id.clone(), entry.event_type.clone());
                if !self.room.contains_key(&key) {
                    let room_types = self.room.keys().filter(|(r, _)| r == room_id).count();
                    if room_types >= MAX_ROOM_TYPES {
                        return Err(AccountDataError::Invalid {
                            diagnostic_id: "p6.7-room-type-cap",
                        });
                    }
                    if !rooms.contains(room_id) && rooms.len() >= MAX_ROOMS_WITH_ACCOUNT_DATA {
                        return Err(AccountDataError::Invalid {
                            diagnostic_id: "p6.7-room-cap",
                        });
                    }
                }
                self.room.insert(key, entry);
            }
        }
        Ok(())
    }

    pub fn get_global(&self, event_type: &str) -> Option<&AccountDataEntry> {
        self.global.get(event_type)
    }

    pub fn get_room(&self, room_id: &str, event_type: &str) -> Option<&AccountDataEntry> {
        self.room.get(&(room_id.to_owned(), event_type.to_owned()))
    }

    /// Convenience: `m.fully_read` event_id field for a room.
    pub fn fully_read_event_id(&self, room_id: &str) -> Option<&str> {
        self.get_room(room_id, TYPE_FULLY_READ)
            .and_then(|e| e.get("event_id"))
    }

    /// Set fully-read marker (room account data helper).
    pub fn set_fully_read(
        &mut self,
        room_id: impl Into<String>,
        event_id: impl Into<String>,
    ) -> Result<(), AccountDataError> {
        let room_id = room_id.into();
        let event_id = event_id.into();
        validate_room(&room_id)?;
        if event_id.is_empty() || !event_id.starts_with('$') {
            return Err(AccountDataError::Invalid {
                diagnostic_id: "p6.7-invalid-event-id",
            });
        }
        let mut fields = BTreeMap::new();
        fields.insert("event_id".into(), event_id);
        self.upsert(AccountDataEntry {
            event_type: TYPE_FULLY_READ.into(),
            room_id: Some(room_id),
            fields,
        })
    }

    pub fn remove_global(&mut self, event_type: &str) -> bool {
        self.global.remove(event_type).is_some()
    }

    pub fn remove_room(&mut self, room_id: &str, event_type: &str) -> bool {
        self.room
            .remove(&(room_id.to_owned(), event_type.to_owned()))
            .is_some()
    }

    pub fn list_global_types(&self) -> Vec<&str> {
        let mut v: Vec<_> = self.global.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.global.clear();
        self.room.clear();
    }
}

fn validate_event_type(t: &str) -> Result<(), AccountDataError> {
    if t.is_empty() || t.len() > MAX_KEY_LEN {
        return Err(AccountDataError::Invalid {
            diagnostic_id: "p6.7-invalid-event-type",
        });
    }
    if t.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(AccountDataError::Invalid {
            diagnostic_id: "p6.7-invalid-event-type",
        });
    }
    Ok(())
}

fn validate_room(room_id: &str) -> Result<(), AccountDataError> {
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(AccountDataError::Invalid {
            diagnostic_id: "p6.7-invalid-room-id",
        });
    }
    Ok(())
}

fn validate_fields(fields: &BTreeMap<String, String>) -> Result<(), AccountDataError> {
    for (k, v) in fields {
        if k.is_empty() || k.len() > MAX_KEY_LEN {
            return Err(AccountDataError::Invalid {
                diagnostic_id: "p6.7-invalid-field-key",
            });
        }
        let kl = k.to_ascii_lowercase();
        if kl.contains("access_token")
            || kl.contains("refresh_token")
            || kl.contains("password")
            || kl.contains("secret")
            || kl.contains("private_key")
        {
            return Err(AccountDataError::Forbidden {
                diagnostic_id: "p6.7-forbidden-field-key",
            });
        }
        if v.len() > MAX_VALUE_LEN {
            return Err(AccountDataError::Invalid {
                diagnostic_id: "p6.7-value-too-long",
            });
        }
        let vl = v.to_ascii_lowercase();
        if vl.contains("access_token=")
            || vl.contains("refresh_token=")
            || vl.contains("-----begin")
        {
            return Err(AccountDataError::Forbidden {
                diagnostic_id: "p6.7-forbidden-field-value",
            });
        }
    }
    Ok(())
}
