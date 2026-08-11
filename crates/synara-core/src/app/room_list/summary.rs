//! Helpers to build privacy-safe [`RoomSummary`] rows for harness tests.
//!
//! Live mapping from SDK `Room` / `RoomListItem` is intentionally deferred until
//! a sliding-sync partial-path harness lands; this module only constructs DTOs from
//! pure product fields (no matrix-sdk types).

use crate::dto::{Membership, RoomId, RoomSummary};

/// Minimal builder for harness room rows (no tokens, no media bytes).
#[derive(Debug, Clone)]
pub struct RoomSummaryBuilder {
    room_id: String,
    name: Option<String>,
    membership: Membership,
    is_direct: bool,
    is_call: bool,
    is_favorite: bool,
    is_low_priority: bool,
    folder_id: Option<String>,
    is_encrypted: bool,
    unread_count: u32,
    highlight_count: u32,
    marked_unread: bool,
    last_activity_ts: Option<u64>,
}

impl RoomSummaryBuilder {
    pub fn new(room_id: impl Into<String>) -> Self {
        Self {
            room_id: room_id.into(),
            name: None,
            membership: Membership::Join,
            is_direct: false,
            is_call: false,
            is_favorite: false,
            is_low_priority: false,
            folder_id: None,
            is_encrypted: false,
            unread_count: 0,
            highlight_count: 0,
            marked_unread: false,
            last_activity_ts: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn membership(mut self, membership: Membership) -> Self {
        self.membership = membership;
        self
    }

    pub fn direct(mut self, is_direct: bool) -> Self {
        self.is_direct = is_direct;
        self
    }

    pub fn favorite(mut self, is_favorite: bool) -> Self {
        self.is_favorite = is_favorite;
        self
    }

    pub fn call(mut self, is_call: bool) -> Self {
        self.is_call = is_call;
        self
    }

    pub fn low_priority(mut self, is_low_priority: bool) -> Self {
        self.is_low_priority = is_low_priority;
        self
    }

    pub fn folder_id(mut self, folder_id: impl Into<String>) -> Self {
        self.folder_id = Some(folder_id.into());
        self
    }

    pub fn encrypted(mut self, is_encrypted: bool) -> Self {
        self.is_encrypted = is_encrypted;
        self
    }

    pub fn unread(mut self, unread: u32, highlight: u32) -> Self {
        self.unread_count = unread;
        self.highlight_count = highlight;
        self
    }

    pub fn marked_unread(mut self, marked: bool) -> Self {
        self.marked_unread = marked;
        self
    }

    pub fn last_activity_ts(mut self, ts: u64) -> Self {
        self.last_activity_ts = Some(ts);
        self
    }

    pub fn build(self) -> Result<RoomSummary, super::error::RoomListError> {
        let room_id = self.room_id.trim().to_owned();
        if room_id.is_empty() || !room_id.starts_with('!') {
            return Err(super::error::RoomListError::Invalid {
                diagnostic_id: "p4.2-invalid-room-id",
            });
        }
        let room_id: RoomId = room_id;
        Ok(RoomSummary {
            room_id,
            name: self.name,
            canonical_alias: None,
            avatar_url: None,
            membership: self.membership,
            is_direct: self.is_direct,
            is_call: self.is_call,
            is_space: false,
            is_favorite: self.is_favorite,
            is_low_priority: self.is_low_priority,
            folder_id: self.folder_id,
            is_encrypted: self.is_encrypted,
            join_rule: None,
            unread_count: self.unread_count,
            highlight_count: self.highlight_count,
            marked_unread: self.marked_unread,
            notification_mode: None,
            last_activity_ts: self.last_activity_ts,
            heroes: None,
            tombstone_successor_room_id: None,
        })
    }
}
