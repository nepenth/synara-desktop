//! Per-room receipt index (P6.2 harness foundation).
//!
//! Stores Synara [`Receipt`] DTOs only — no SDK types, no dual-backend.
//! Host applies network-mapped receipts; this module is pure projection.

use std::collections::HashMap;

use crate::dto::{Receipt, ReceiptType, RoomId, UserId};

use super::error::ReceiptError;

/// Key for the latest receipt of one (user, type, optional thread) in a room.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReceiptKey {
    user_id: UserId,
    receipt_type: ReceiptType,
    thread_id: Option<String>,
}

/// Session-generation-stamped index of latest receipts by room.
#[derive(Debug, Default)]
pub struct ReceiptIndex {
    session_generation: u64,
    /// room_id → (key → Receipt)
    by_room: HashMap<RoomId, HashMap<ReceiptKey, Receipt>>,
}

impl ReceiptIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_room: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn room_count(&self) -> usize {
        self.by_room.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_room.is_empty()
    }

    fn validate_ids(receipt: &Receipt) -> Result<(), ReceiptError> {
        if receipt.room_id.is_empty() || !receipt.room_id.starts_with('!') {
            return Err(ReceiptError::Invalid {
                diagnostic_id: "p6.2-invalid-room-id",
            });
        }
        if receipt.event_id.is_empty() || !receipt.event_id.starts_with('$') {
            return Err(ReceiptError::Invalid {
                diagnostic_id: "p6.2-invalid-event-id",
            });
        }
        if receipt.user_id.is_empty() || !receipt.user_id.starts_with('@') {
            return Err(ReceiptError::Invalid {
                diagnostic_id: "p6.2-invalid-user-id",
            });
        }
        Ok(())
    }

    fn key_of(receipt: &Receipt) -> ReceiptKey {
        ReceiptKey {
            user_id: receipt.user_id.clone(),
            receipt_type: receipt.receipt_type,
            thread_id: receipt.thread_id.clone(),
        }
    }

    /// Apply one receipt. Newer `ts` wins when both present; otherwise last write wins.
    pub fn apply(&mut self, receipt: Receipt) -> Result<(), ReceiptError> {
        Self::validate_ids(&receipt)?;
        let key = Self::key_of(&receipt);
        let room = self.by_room.entry(receipt.room_id.clone()).or_default();
        if let Some(existing) = room.get(&key) {
            match (existing.ts, receipt.ts) {
                (Some(old), Some(new)) if new < old => {
                    // Stale timestamp — keep existing.
                    return Ok(());
                }
                _ => {}
            }
        }
        room.insert(key, receipt);
        Ok(())
    }

    /// Apply many receipts (best-effort batch; first error aborts).
    pub fn apply_batch(&mut self, receipts: Vec<Receipt>) -> Result<usize, ReceiptError> {
        let mut n = 0;
        for r in receipts {
            self.apply(r)?;
            n += 1;
        }
        Ok(n)
    }

    /// All receipts currently indexed for `room_id`.
    pub fn list_room(&self, room_id: &str) -> Vec<&Receipt> {
        match self.by_room.get(room_id) {
            Some(map) => {
                let mut v: Vec<_> = map.values().collect();
                v.sort_by(|a, b| {
                    a.user_id
                        .cmp(&b.user_id)
                        .then_with(|| a.receipt_type.as_str().cmp(b.receipt_type.as_str()))
                        .then_with(|| a.event_id.cmp(&b.event_id))
                });
                v
            }
            None => Vec::new(),
        }
    }

    /// Latest public read receipt for `user_id` in `room_id` (main timeline).
    pub fn latest_read(&self, room_id: &str, user_id: &str) -> Option<&Receipt> {
        let map = self.by_room.get(room_id)?;
        map.get(&ReceiptKey {
            user_id: user_id.to_owned(),
            receipt_type: ReceiptType::Read,
            thread_id: None,
        })
    }

    /// Drop all receipts for a room.
    pub fn clear_room(&mut self, room_id: &str) {
        self.by_room.remove(room_id);
    }

    pub fn clear(&mut self) {
        self.by_room.clear();
    }

    /// Bump generation and wipe (logout / account switch — no cross-account leak).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.by_room.clear();
    }
}
