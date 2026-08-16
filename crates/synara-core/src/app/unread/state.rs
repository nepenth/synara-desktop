//! Room read state + open-position policy (P5.5 harness foundation).
//!
//! Aligns with `docs/timeline-room-state-reliability-contract.md`.
//! **No tokens, no event bodies, no dual-backend.** Ids and counts only.

use std::collections::HashMap;

use crate::dto::{EventId, Receipt, ReceiptType, RoomId};

use super::error::UnreadError;

/// Soft cap on rooms tracked for open policy.
pub const MAX_TRACKED_ROOMS: usize = 4_096;

/// Source of the effective read frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontierSource {
    FullyRead,
    PublicReceipt,
    PrivateReceipt,
    CurrentLiveBottom,
    Absent,
}

impl FrontierSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FullyRead => "fully_read",
            Self::PublicReceipt => "public_receipt",
            Self::PrivateReceipt => "private_receipt",
            Self::CurrentLiveBottom => "current_live_bottom",
            Self::Absent => "absent",
        }
    }
}

/// Receipt privacy preference for writes (projection only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReceiptPrivacy {
    Public,
    #[default]
    Private,
}

impl ReceiptPrivacy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

/// Recommended initial timeline open mode for a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenPositionPolicy {
    /// Open live end (fully read / no unread signal).
    LiveBottom,
    /// Open bounded unread context at marker (first unread area).
    UnreadContext {
        marker_event_id: EventId,
        source: FrontierSource,
    },
    /// Explicit event route wins over unread/live (host supplies event id).
    ExplicitEvent { event_id: EventId },
    /// Local viewport restore allowed (fully read + fresh anchor, no unread).
    RestoreLocalViewport,
}

impl OpenPositionPolicy {
    pub fn as_kind_str(&self) -> &'static str {
        match self {
            Self::LiveBottom => "live_bottom",
            Self::UnreadContext { .. } => "unread_context",
            Self::ExplicitEvent { .. } => "explicit_event",
            Self::RestoreLocalViewport => "restore_local_viewport",
        }
    }
}

/// Shared room read signals (event ids only).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoomReadState {
    pub room_id: RoomId,
    pub fully_read_event_id: Option<EventId>,
    pub public_receipt_event_id: Option<EventId>,
    pub private_receipt_event_id: Option<EventId>,
    /// Optional latest known live event id (host-filled from timeline tail).
    pub live_bottom_event_id: Option<EventId>,
    /// Notification / highlight unread signal from room list (host).
    pub has_unread_notification: bool,
    pub notification_count: u32,
    pub highlight_count: u32,
    pub receipt_privacy: ReceiptPrivacy,
    /// Process-local viewport: true if host has a fresh historical restore candidate.
    pub has_fresh_local_viewport: bool,
}

impl RoomReadState {
    pub fn new(room_id: impl Into<String>) -> Result<Self, UnreadError> {
        let room_id = room_id.into();
        validate_room(&room_id)?;
        Ok(Self {
            room_id,
            fully_read_event_id: None,
            public_receipt_event_id: None,
            private_receipt_event_id: None,
            live_bottom_event_id: None,
            has_unread_notification: false,
            notification_count: 0,
            highlight_count: 0,
            receipt_privacy: ReceiptPrivacy::Private,
            has_fresh_local_viewport: false,
        })
    }

    /// Effective frontier event id + source (fully_read preferred, then privacy receipt).
    pub fn effective_frontier(&self) -> (Option<&str>, FrontierSource) {
        if let Some(id) = &self.fully_read_event_id {
            return (Some(id.as_str()), FrontierSource::FullyRead);
        }
        match self.receipt_privacy {
            ReceiptPrivacy::Public => {
                if let Some(id) = &self.public_receipt_event_id {
                    return (Some(id.as_str()), FrontierSource::PublicReceipt);
                }
                if let Some(id) = &self.private_receipt_event_id {
                    return (Some(id.as_str()), FrontierSource::PrivateReceipt);
                }
            }
            ReceiptPrivacy::Private => {
                if let Some(id) = &self.private_receipt_event_id {
                    return (Some(id.as_str()), FrontierSource::PrivateReceipt);
                }
                if let Some(id) = &self.public_receipt_event_id {
                    return (Some(id.as_str()), FrontierSource::PublicReceipt);
                }
            }
        }
        if let Some(id) = &self.live_bottom_event_id {
            return (Some(id.as_str()), FrontierSource::CurrentLiveBottom);
        }
        (None, FrontierSource::Absent)
    }

    pub fn has_unread_signal(&self) -> bool {
        self.has_unread_notification || self.notification_count > 0 || self.highlight_count > 0
    }
}

/// Session-generation-stamped unread / open-position store.
#[derive(Debug, Default)]
pub struct UnreadPositionStore {
    session_generation: u64,
    by_room: HashMap<RoomId, RoomReadState>,
}

impl UnreadPositionStore {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_room: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_room.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_room.is_empty()
    }

    pub fn get(&self, room_id: &str) -> Option<&RoomReadState> {
        self.by_room.get(room_id)
    }

    /// Upsert full room read state (host maps receipts / room list).
    pub fn upsert(&mut self, state: RoomReadState) -> Result<(), UnreadError> {
        validate_room(&state.room_id)?;
        validate_opt_event(state.fully_read_event_id.as_deref())?;
        validate_opt_event(state.public_receipt_event_id.as_deref())?;
        validate_opt_event(state.private_receipt_event_id.as_deref())?;
        validate_opt_event(state.live_bottom_event_id.as_deref())?;
        if !self.by_room.contains_key(&state.room_id) && self.by_room.len() >= MAX_TRACKED_ROOMS {
            return Err(UnreadError::Invalid {
                diagnostic_id: "p5.5-room-cap",
            });
        }
        self.by_room.insert(state.room_id.clone(), state);
        Ok(())
    }

    /// Apply a receipt DTO into room read state (creates room if needed).
    pub fn apply_receipt(&mut self, receipt: Receipt) -> Result<(), UnreadError> {
        validate_room(&receipt.room_id)?;
        validate_event(&receipt.event_id)?;
        if receipt.user_id.is_empty() || !receipt.user_id.starts_with('@') {
            return Err(UnreadError::Invalid {
                diagnostic_id: "p5.5-invalid-user-id",
            });
        }
        let entry = self
            .by_room
            .entry(receipt.room_id.clone())
            .or_insert_with(|| RoomReadState {
                room_id: receipt.room_id.clone(),
                ..RoomReadState::new(receipt.room_id.clone()).expect("validated room")
            });
        match receipt.receipt_type {
            ReceiptType::FullyRead => {
                entry.fully_read_event_id = Some(receipt.event_id);
            }
            ReceiptType::Read => {
                entry.public_receipt_event_id = Some(receipt.event_id);
            }
            ReceiptType::ReadPrivate => {
                entry.private_receipt_event_id = Some(receipt.event_id);
            }
        }
        Ok(())
    }

    pub fn set_notification_counts(
        &mut self,
        room_id: &str,
        notification_count: u32,
        highlight_count: u32,
    ) -> Result<(), UnreadError> {
        let entry = self.by_room.get_mut(room_id).ok_or(UnreadError::NotFound {
            diagnostic_id: "p5.5-room-not-found",
        })?;
        entry.notification_count = notification_count;
        entry.highlight_count = highlight_count;
        entry.has_unread_notification = notification_count > 0 || highlight_count > 0;
        Ok(())
    }

    pub fn set_live_bottom(
        &mut self,
        room_id: &str,
        event_id: Option<EventId>,
    ) -> Result<(), UnreadError> {
        validate_opt_event(event_id.as_deref())?;
        let entry = self.by_room.get_mut(room_id).ok_or(UnreadError::NotFound {
            diagnostic_id: "p5.5-room-not-found",
        })?;
        entry.live_bottom_event_id = event_id;
        Ok(())
    }

    pub fn set_fresh_local_viewport(
        &mut self,
        room_id: &str,
        fresh: bool,
    ) -> Result<(), UnreadError> {
        let entry = self.by_room.get_mut(room_id).ok_or(UnreadError::NotFound {
            diagnostic_id: "p5.5-room-not-found",
        })?;
        entry.has_fresh_local_viewport = fresh;
        Ok(())
    }

    /// Decide initial open position. Explicit route event wins when provided.
    pub fn decide_open(
        &self,
        room_id: &str,
        explicit_event_id: Option<&str>,
    ) -> Result<OpenPositionPolicy, UnreadError> {
        if let Some(eid) = explicit_event_id {
            validate_event(eid)?;
            return Ok(OpenPositionPolicy::ExplicitEvent {
                event_id: eid.to_owned(),
            });
        }
        let state = self.by_room.get(room_id).ok_or(UnreadError::NotFound {
            diagnostic_id: "p5.5-room-not-found",
        })?;
        if state.has_unread_signal() {
            let (frontier, source) = state.effective_frontier();
            if let Some(marker) = frontier {
                // Live bottom alone is not an unread marker for context open.
                if source != FrontierSource::CurrentLiveBottom && source != FrontierSource::Absent {
                    return Ok(OpenPositionPolicy::UnreadContext {
                        marker_event_id: marker.to_owned(),
                        source,
                    });
                }
            }
            // Unread signal but no marker — conservative live bottom + jump affordance.
            return Ok(OpenPositionPolicy::LiveBottom);
        }
        if state.has_fresh_local_viewport {
            return Ok(OpenPositionPolicy::RestoreLocalViewport);
        }
        Ok(OpenPositionPolicy::LiveBottom)
    }

    /// Whether jump-to-unread affordance should show (marker outside confident live end).
    pub fn should_show_jump_to_unread(&self, room_id: &str) -> Result<bool, UnreadError> {
        let state = self.by_room.get(room_id).ok_or(UnreadError::NotFound {
            diagnostic_id: "p5.5-room-not-found",
        })?;
        if !state.has_unread_signal() {
            return Ok(false);
        }
        let (frontier, source) = state.effective_frontier();
        let Some(marker) = frontier else {
            return Ok(true);
        };
        if source == FrontierSource::CurrentLiveBottom || source == FrontierSource::Absent {
            return Ok(true);
        }
        if let Some(live) = &state.live_bottom_event_id {
            return Ok(live.as_str() != marker);
        }
        Ok(true)
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_room.clear();
    }
}

fn validate_room(room_id: &str) -> Result<(), UnreadError> {
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(UnreadError::Invalid {
            diagnostic_id: "p5.5-invalid-room-id",
        });
    }
    Ok(())
}

fn validate_event(event_id: &str) -> Result<(), UnreadError> {
    if event_id.is_empty() || !event_id.starts_with('$') {
        return Err(UnreadError::Invalid {
            diagnostic_id: "p5.5-invalid-event-id",
        });
    }
    Ok(())
}

fn validate_opt_event(event_id: Option<&str>) -> Result<(), UnreadError> {
    match event_id {
        None => Ok(()),
        Some(e) => validate_event(e),
    }
}
