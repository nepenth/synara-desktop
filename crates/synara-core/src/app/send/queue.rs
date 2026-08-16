//! Outbound text send queue + local-echo state (P6.1 harness foundation).
//!
//! Tracks user-composed plain-text sends with generation stamps and
//! [`LocalEchoState`]. Does **not** call SDK `Room::send` yet — host will
//! drive network later. No dual-backend, no tokens in errors.

use std::collections::HashMap;

use crate::dto::{LocalEchoState, RoomId};

use super::error::SendError;

/// Opaque local transaction id for one outbound attempt (not a server event id).
pub type LocalTxnId = String;

/// Privacy-safe queued outbound text message (body is user-composed plain text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundTextMessage {
    pub local_txn_id: LocalTxnId,
    pub room_id: RoomId,
    pub session_generation: u64,
    /// User-composed plain-text body preview (not ciphertext).
    pub body: String,
    pub state: LocalEchoState,
    /// Privacy-safe diagnostic when `state == Failed` (never tokens / HS bodies).
    pub failure_diagnostic_id: Option<&'static str>,
}

impl OutboundTextMessage {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            LocalEchoState::Sent | LocalEchoState::Failed | LocalEchoState::Cancelled
        )
    }
}

/// Per-session-generation send queue.
#[derive(Debug, Default)]
pub struct SendQueue {
    session_generation: u64,
    /// Insertion-ordered ids for stable listing.
    order: Vec<LocalTxnId>,
    items: HashMap<LocalTxnId, OutboundTextMessage>,
    next_seq: u64,
}

impl SendQueue {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            order: Vec::new(),
            items: HashMap::new(),
            next_seq: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn active_count(&self) -> usize {
        self.items
            .values()
            .filter(|i| matches!(i.state, LocalEchoState::Sending))
            .count()
    }

    fn alloc_txn_id(&mut self) -> LocalTxnId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("local-txn-{}", self.next_seq)
    }

    /// Enqueue a plain-text message for `room_id` in Sending state.
    pub fn enqueue_text(
        &mut self,
        room_id: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<&OutboundTextMessage, SendError> {
        let room_id = room_id.into().trim().to_owned();
        if room_id.is_empty() || !room_id.starts_with('!') {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-invalid-room-id",
            });
        }
        let body = body.into();
        if body.is_empty() {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-empty-body",
            });
        }
        if body.len() > 65_536 {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-body-too-large",
            });
        }
        let local_txn_id = self.alloc_txn_id();
        let item = OutboundTextMessage {
            local_txn_id: local_txn_id.clone(),
            room_id,
            session_generation: self.session_generation,
            body,
            state: LocalEchoState::Sending,
            failure_diagnostic_id: None,
        };
        self.order.push(local_txn_id.clone());
        self.items.insert(local_txn_id.clone(), item);
        Ok(self.items.get(&local_txn_id).expect("just inserted"))
    }

    pub fn get(&self, local_txn_id: &str) -> Option<&OutboundTextMessage> {
        self.items.get(local_txn_id)
    }

    fn get_mut_checked(
        &mut self,
        local_txn_id: &str,
    ) -> Result<&mut OutboundTextMessage, SendError> {
        let item = self
            .items
            .get_mut(local_txn_id)
            .ok_or(SendError::NotFound {
                diagnostic_id: "p6.1-send-not-found",
            })?;
        if item.session_generation != self.session_generation {
            return Err(SendError::StaleGeneration {
                diagnostic_id: "p6.1-stale-send-generation",
                expected: self.session_generation,
                observed: item.session_generation,
            });
        }
        Ok(item)
    }

    /// Mark send as successfully delivered (host obtained server event id later).
    pub fn mark_sent(&mut self, local_txn_id: &str) -> Result<&OutboundTextMessage, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if item.state != LocalEchoState::Sending {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-mark-sent-invalid-state",
            });
        }
        item.state = LocalEchoState::Sent;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    /// Mark send failed with privacy-safe diagnostic only.
    pub fn mark_failed(
        &mut self,
        local_txn_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<&OutboundTextMessage, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if item.state != LocalEchoState::Sending {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-mark-failed-invalid-state",
            });
        }
        item.state = LocalEchoState::Failed;
        item.failure_diagnostic_id = Some(diagnostic_id);
        Ok(item)
    }

    /// Cancel an in-flight or failed send (user abort).
    pub fn cancel(&mut self, local_txn_id: &str) -> Result<&OutboundTextMessage, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if matches!(item.state, LocalEchoState::Sent | LocalEchoState::Cancelled) {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-cancel-invalid-state",
            });
        }
        item.state = LocalEchoState::Cancelled;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    /// Retry a Failed item → Sending.
    pub fn retry(&mut self, local_txn_id: &str) -> Result<&OutboundTextMessage, SendError> {
        let item = self.get_mut_checked(local_txn_id)?;
        if item.state != LocalEchoState::Failed {
            return Err(SendError::Invalid {
                diagnostic_id: "p6.1-retry-not-failed",
            });
        }
        item.state = LocalEchoState::Sending;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    /// Ordered list of items (enqueue order).
    pub fn list(&self) -> Vec<&OutboundTextMessage> {
        self.order
            .iter()
            .filter_map(|id| self.items.get(id))
            .collect()
    }

    /// Items for one room in enqueue order.
    pub fn list_for_room(&self, room_id: &str) -> Vec<&OutboundTextMessage> {
        self.list()
            .into_iter()
            .filter(|i| i.room_id == room_id)
            .collect()
    }

    /// Drop terminal items (sent/failed/cancelled) to bound memory.
    pub fn prune_terminal(&mut self) -> usize {
        let before = self.items.len();
        self.order.retain(|id| {
            self.items
                .get(id)
                .map(|i| !i.is_terminal())
                .unwrap_or(false)
        });
        self.items.retain(|_, i| !i.is_terminal());
        before.saturating_sub(self.items.len())
    }

    /// Bump generation and cancel all Sending items (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        for item in self.items.values_mut() {
            if item.state == LocalEchoState::Sending {
                item.state = LocalEchoState::Cancelled;
                item.failure_diagnostic_id = Some("p6.1-stale-generation-cancelled");
            }
            item.session_generation = new_generation;
        }
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.items.clear();
    }
}
