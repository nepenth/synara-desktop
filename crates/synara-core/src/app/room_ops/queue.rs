//! Room membership operation queue (P6.9 harness foundation).
//!
//! Tracks create / join / leave / invite / kick / ban / forget intents with
//! lifecycle state. **No SDK network.** No tokens. No dual-backend.

use std::collections::HashMap;

use crate::dto::{RoomId, UserId};

use super::error::RoomOpsError;

/// Soft cap on concurrent tracked ops (active + terminal until pruned).
pub const MAX_TRACKED_OPS: usize = 256;

/// Soft cap on reason / name field length (chars).
pub const MAX_REASON_CHARS: usize = 512;

/// Soft cap on create-room name length (chars).
pub const MAX_CREATE_NAME_CHARS: usize = 256;

/// Opaque local op id (not a server event id).
pub type LocalOpId = String;

/// Kind of membership / room lifecycle mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomOpKind {
    /// Create a room or DM (result room_id filled on success).
    Create,
    Join,
    Leave,
    Invite,
    Kick,
    Ban,
    Unban,
    Forget,
}

impl RoomOpKind {
    pub const ALL: &'static [RoomOpKind] = &[
        Self::Create,
        Self::Join,
        Self::Leave,
        Self::Invite,
        Self::Kick,
        Self::Ban,
        Self::Unban,
        Self::Forget,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Join => "join",
            Self::Leave => "leave",
            Self::Invite => "invite",
            Self::Kick => "kick",
            Self::Ban => "ban",
            Self::Unban => "unban",
            Self::Forget => "forget",
        }
    }
}

/// Lifecycle of one room op attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomOpState {
    Pending,
    InFlight,
    Succeeded,
    Failed,
    Cancelled,
}

impl RoomOpState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InFlight => "in_flight",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// One membership / room lifecycle operation intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomOp {
    pub local_op_id: LocalOpId,
    pub kind: RoomOpKind,
    pub session_generation: u64,
    /// Target room when known (`None` until Create succeeds).
    pub room_id: Option<RoomId>,
    /// Target user for invite/kick/ban/unban.
    pub target_user_id: Option<UserId>,
    /// Optional create-room display name (Create only).
    pub create_name: Option<String>,
    /// Optional reason (kick/ban/leave) — plain text, no secrets.
    pub reason: Option<String>,
    pub state: RoomOpState,
    /// Privacy-safe diagnostic when `state == Failed`.
    pub failure_diagnostic_id: Option<&'static str>,
}

impl RoomOp {
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

/// Session-generation-stamped room ops queue.
#[derive(Debug, Default)]
pub struct RoomOpsQueue {
    session_generation: u64,
    order: Vec<LocalOpId>,
    items: HashMap<LocalOpId, RoomOp>,
    next_seq: u64,
}

impl RoomOpsQueue {
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
            .filter(|i| matches!(i.state, RoomOpState::Pending | RoomOpState::InFlight))
            .count()
    }

    fn alloc_op_id(&mut self) -> LocalOpId {
        self.next_seq = self.next_seq.saturating_add(1);
        format!("room-op-{}", self.next_seq)
    }

    fn ensure_capacity(&self) -> Result<(), RoomOpsError> {
        if self.items.len() >= MAX_TRACKED_OPS {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-op-cap",
            });
        }
        Ok(())
    }

    fn insert(&mut self, op: RoomOp) -> Result<&RoomOp, RoomOpsError> {
        self.ensure_capacity()?;
        let id = op.local_op_id.clone();
        self.order.push(id.clone());
        self.items.insert(id.clone(), op);
        Ok(self.items.get(&id).expect("just inserted"))
    }

    /// Enqueue create-room (or DM) intent. Room id filled on success.
    pub fn enqueue_create(&mut self, create_name: Option<String>) -> Result<&RoomOp, RoomOpsError> {
        if let Some(ref n) = create_name {
            if n.chars().count() > MAX_CREATE_NAME_CHARS {
                return Err(RoomOpsError::Invalid {
                    diagnostic_id: "p6.9-create-name-cap",
                });
            }
        }
        let local_op_id = self.alloc_op_id();
        self.insert(RoomOp {
            local_op_id,
            kind: RoomOpKind::Create,
            session_generation: self.session_generation,
            room_id: None,
            target_user_id: None,
            create_name,
            reason: None,
            state: RoomOpState::Pending,
            failure_diagnostic_id: None,
        })
    }

    /// Enqueue join by room id (alias resolution is host-side).
    pub fn enqueue_join(&mut self, room_id: impl Into<String>) -> Result<&RoomOp, RoomOpsError> {
        let room_id = validate_room_id(room_id)?;
        let local_op_id = self.alloc_op_id();
        self.insert(RoomOp {
            local_op_id,
            kind: RoomOpKind::Join,
            session_generation: self.session_generation,
            room_id: Some(room_id),
            target_user_id: None,
            create_name: None,
            reason: None,
            state: RoomOpState::Pending,
            failure_diagnostic_id: None,
        })
    }

    pub fn enqueue_leave(
        &mut self,
        room_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        self.enqueue_room_only(RoomOpKind::Leave, room_id, reason)
    }

    pub fn enqueue_forget(&mut self, room_id: impl Into<String>) -> Result<&RoomOp, RoomOpsError> {
        self.enqueue_room_only(RoomOpKind::Forget, room_id, None)
    }

    pub fn enqueue_invite(
        &mut self,
        room_id: impl Into<String>,
        target_user_id: impl Into<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        self.enqueue_user_target(RoomOpKind::Invite, room_id, target_user_id, None)
    }

    pub fn enqueue_kick(
        &mut self,
        room_id: impl Into<String>,
        target_user_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        self.enqueue_user_target(RoomOpKind::Kick, room_id, target_user_id, reason)
    }

    pub fn enqueue_ban(
        &mut self,
        room_id: impl Into<String>,
        target_user_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        self.enqueue_user_target(RoomOpKind::Ban, room_id, target_user_id, reason)
    }

    pub fn enqueue_unban(
        &mut self,
        room_id: impl Into<String>,
        target_user_id: impl Into<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        self.enqueue_user_target(RoomOpKind::Unban, room_id, target_user_id, None)
    }

    fn enqueue_room_only(
        &mut self,
        kind: RoomOpKind,
        room_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        let room_id = validate_room_id(room_id)?;
        validate_reason(&reason)?;
        let local_op_id = self.alloc_op_id();
        self.insert(RoomOp {
            local_op_id,
            kind,
            session_generation: self.session_generation,
            room_id: Some(room_id),
            target_user_id: None,
            create_name: None,
            reason,
            state: RoomOpState::Pending,
            failure_diagnostic_id: None,
        })
    }

    fn enqueue_user_target(
        &mut self,
        kind: RoomOpKind,
        room_id: impl Into<String>,
        target_user_id: impl Into<String>,
        reason: Option<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        let room_id = validate_room_id(room_id)?;
        let target_user_id = validate_user_id(target_user_id)?;
        validate_reason(&reason)?;
        let local_op_id = self.alloc_op_id();
        self.insert(RoomOp {
            local_op_id,
            kind,
            session_generation: self.session_generation,
            room_id: Some(room_id),
            target_user_id: Some(target_user_id),
            create_name: None,
            reason,
            state: RoomOpState::Pending,
            failure_diagnostic_id: None,
        })
    }

    pub fn get(&self, local_op_id: &str) -> Option<&RoomOp> {
        self.items.get(local_op_id)
    }

    fn get_mut_checked(&mut self, local_op_id: &str) -> Result<&mut RoomOp, RoomOpsError> {
        let item = self
            .items
            .get_mut(local_op_id)
            .ok_or(RoomOpsError::NotFound {
                diagnostic_id: "p6.9-op-not-found",
            })?;
        if item.session_generation != self.session_generation {
            return Err(RoomOpsError::StaleGeneration {
                diagnostic_id: "p6.9-stale-generation",
                expected: self.session_generation,
                observed: item.session_generation,
            });
        }
        Ok(item)
    }

    /// Host begins network work: Pending → InFlight.
    pub fn mark_in_flight(&mut self, local_op_id: &str) -> Result<&RoomOp, RoomOpsError> {
        let item = self.get_mut_checked(local_op_id)?;
        if item.state != RoomOpState::Pending {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-mark-inflight-invalid-state",
            });
        }
        item.state = RoomOpState::InFlight;
        Ok(item)
    }

    /// Mark succeeded. For Create, `result_room_id` is required.
    pub fn mark_succeeded(
        &mut self,
        local_op_id: &str,
        result_room_id: Option<String>,
    ) -> Result<&RoomOp, RoomOpsError> {
        // Validate create result before mutating.
        let kind = self
            .items
            .get(local_op_id)
            .ok_or(RoomOpsError::NotFound {
                diagnostic_id: "p6.9-op-not-found",
            })?
            .kind;
        let resolved_room = match kind {
            RoomOpKind::Create => {
                let rid = result_room_id.ok_or(RoomOpsError::Invalid {
                    diagnostic_id: "p6.9-create-missing-room-id",
                })?;
                Some(validate_room_id(rid)?)
            }
            _ => {
                if result_room_id.is_some() {
                    return Err(RoomOpsError::Invalid {
                        diagnostic_id: "p6.9-unexpected-result-room-id",
                    });
                }
                None
            }
        };

        let item = self.get_mut_checked(local_op_id)?;
        if item.state != RoomOpState::InFlight {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-mark-succeeded-invalid-state",
            });
        }
        if let Some(rid) = resolved_room {
            item.room_id = Some(rid);
        }
        item.state = RoomOpState::Succeeded;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    pub fn mark_failed(
        &mut self,
        local_op_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<&RoomOp, RoomOpsError> {
        let item = self.get_mut_checked(local_op_id)?;
        if item.state != RoomOpState::InFlight {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-mark-failed-invalid-state",
            });
        }
        item.state = RoomOpState::Failed;
        item.failure_diagnostic_id = Some(diagnostic_id);
        Ok(item)
    }

    pub fn cancel(&mut self, local_op_id: &str) -> Result<&RoomOp, RoomOpsError> {
        let item = self.get_mut_checked(local_op_id)?;
        if matches!(item.state, RoomOpState::Succeeded | RoomOpState::Cancelled) {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-cancel-invalid-state",
            });
        }
        item.state = RoomOpState::Cancelled;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    /// Retry Failed → Pending.
    pub fn retry(&mut self, local_op_id: &str) -> Result<&RoomOp, RoomOpsError> {
        let item = self.get_mut_checked(local_op_id)?;
        if item.state != RoomOpState::Failed {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-retry-not-failed",
            });
        }
        item.state = RoomOpState::Pending;
        item.failure_diagnostic_id = None;
        Ok(item)
    }

    pub fn list(&self) -> Vec<&RoomOp> {
        self.order
            .iter()
            .filter_map(|id| self.items.get(id))
            .collect()
    }

    pub fn list_for_room(&self, room_id: &str) -> Vec<&RoomOp> {
        self.list()
            .into_iter()
            .filter(|i| i.room_id.as_deref() == Some(room_id))
            .collect()
    }

    pub fn list_active(&self) -> Vec<&RoomOp> {
        self.list()
            .into_iter()
            .filter(|i| !i.is_terminal())
            .collect()
    }

    /// Drop terminal items to bound memory.
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

    pub fn clear(&mut self) {
        self.order.clear();
        self.items.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
        self.next_seq = 0;
    }
}

fn validate_room_id(room_id: impl Into<String>) -> Result<RoomId, RoomOpsError> {
    let room_id = room_id.into().trim().to_owned();
    if room_id.is_empty() || !room_id.starts_with('!') {
        return Err(RoomOpsError::Invalid {
            diagnostic_id: "p6.9-invalid-room-id",
        });
    }
    Ok(room_id)
}

fn validate_user_id(user_id: impl Into<String>) -> Result<UserId, RoomOpsError> {
    let user_id = user_id.into().trim().to_owned();
    if user_id.is_empty() || !user_id.starts_with('@') {
        return Err(RoomOpsError::Invalid {
            diagnostic_id: "p6.9-invalid-user-id",
        });
    }
    Ok(user_id)
}

fn validate_reason(reason: &Option<String>) -> Result<(), RoomOpsError> {
    if let Some(r) = reason {
        if r.chars().count() > MAX_REASON_CHARS {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-reason-cap",
            });
        }
        let lower = r.to_ascii_lowercase();
        if lower.contains("access_token") || lower.contains("refresh_token") {
            return Err(RoomOpsError::Invalid {
                diagnostic_id: "p6.9-forbidden-reason",
            });
        }
    }
    Ok(())
}
