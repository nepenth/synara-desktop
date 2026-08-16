//! Verification request inbox + SAS display projection (P8.3 harness foundation).
//!
//! Pure index of product verification flows. **No SAS secrets, MAC keys,
//! recovery material, or tokens.** Display-only emoji short names may be stored
//! for UI compare. No SDK crypto APIs, no dual-backend.

use std::collections::HashMap;

use crate::dto::{DeviceId, UserId};

use super::error::VerificationError;

/// Soft cap on concurrent open verification flows (UI/inbox safety).
pub const MAX_OPEN_FLOWS: usize = 32;

/// Max emoji short-names retained for SAS display (protocol uses 7).
pub const MAX_SAS_EMOJI: usize = 16;

/// Incoming vs outgoing verification request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationDirection {
    Incoming,
    Outgoing,
}

/// Product lifecycle phase for one verification flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationPhase {
    /// Request created; waiting for accept / start.
    Requested,
    /// Ready for user SAS comparison (emoji shown).
    Ready,
    /// User confirmed SAS match (host may complete crypto).
    Confirmed,
    /// User reported SAS mismatch.
    Mismatched,
    /// Cancelled by either side.
    Cancelled,
    /// Successfully completed.
    Done,
    /// Failed (network / crypto / invariant).
    Failed,
}

impl VerificationPhase {
    /// Open flows still need user or host attention.
    pub fn is_open(self) -> bool {
        matches!(
            self,
            Self::Requested | Self::Ready | Self::Confirmed | Self::Mismatched
        )
    }

    /// Pending request that should set security banner attention.
    pub fn is_pending_attention(self) -> bool {
        matches!(self, Self::Requested | Self::Ready)
    }
}

/// Product summary of one verification flow (no secrets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationFlow {
    /// Opaque host-assigned flow id (not a secret).
    pub flow_id: String,
    pub other_user_id: UserId,
    pub other_device_id: DeviceId,
    pub direction: VerificationDirection,
    pub phase: VerificationPhase,
    /// Optional start timestamp (ms), privacy-safe.
    pub started_ts: Option<u64>,
    /// Display-only SAS emoji short names (never keys/MACs).
    pub sas_emoji: Option<Vec<String>>,
    pub failure_diagnostic_id: Option<&'static str>,
}

/// Session-generation-stamped verification inbox.
#[derive(Debug, Default)]
pub struct VerificationInbox {
    session_generation: u64,
    by_id: HashMap<String, VerificationFlow>,
}

impl VerificationInbox {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_id: HashMap::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn open_count(&self) -> usize {
        self.by_id.values().filter(|f| f.phase.is_open()).count()
    }

    pub fn has_pending_attention(&self) -> bool {
        self.by_id.values().any(|f| f.phase.is_pending_attention())
    }

    fn validate_flow_id(flow_id: &str) -> Result<(), VerificationError> {
        if flow_id.is_empty()
            || flow_id.len() > 128
            || !flow_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':')
        {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-flow-id",
            });
        }
        Ok(())
    }

    fn validate_user_id(user_id: &str) -> Result<(), VerificationError> {
        if user_id.is_empty() || !user_id.starts_with('@') || !user_id.contains(':') {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-user-id",
            });
        }
        Ok(())
    }

    fn validate_device_id(device_id: &str) -> Result<(), VerificationError> {
        if device_id.is_empty()
            || !device_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-device-id",
            });
        }
        Ok(())
    }

    /// Upsert a flow (host maps SDK verification → product shape).
    pub fn upsert(&mut self, flow: VerificationFlow) -> Result<(), VerificationError> {
        Self::validate_flow_id(&flow.flow_id)?;
        Self::validate_user_id(&flow.other_user_id)?;
        Self::validate_device_id(&flow.other_device_id)?;
        if let Some(ref emoji) = flow.sas_emoji {
            if emoji.len() > MAX_SAS_EMOJI {
                return Err(VerificationError::Invalid {
                    diagnostic_id: "p8.3-sas-emoji-cap",
                });
            }
            for name in emoji {
                if name.is_empty() || name.len() > 32 {
                    return Err(VerificationError::Invalid {
                        diagnostic_id: "p8.3-invalid-sas-emoji",
                    });
                }
            }
        }
        let is_new = !self.by_id.contains_key(&flow.flow_id);
        if is_new && flow.phase.is_open() && self.open_count() >= MAX_OPEN_FLOWS {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-open-flow-cap",
            });
        }
        self.by_id.insert(flow.flow_id.clone(), flow);
        Ok(())
    }

    pub fn get(&self, flow_id: &str) -> Option<&VerificationFlow> {
        self.by_id.get(flow_id)
    }

    /// Open flows first (Requested/Ready before Confirmed), then by flow_id.
    pub fn list_open(&self) -> Vec<&VerificationFlow> {
        let mut v: Vec<_> = self.by_id.values().filter(|f| f.phase.is_open()).collect();
        v.sort_by(|a, b| {
            phase_rank(a.phase)
                .cmp(&phase_rank(b.phase))
                .then_with(|| a.flow_id.cmp(&b.flow_id))
        });
        v
    }

    /// Transition Requested → Ready (accept / SAS started). Optional emoji.
    pub fn mark_ready(
        &mut self,
        flow_id: &str,
        sas_emoji: Option<Vec<String>>,
    ) -> Result<(), VerificationError> {
        let flow = self
            .by_id
            .get_mut(flow_id)
            .ok_or(VerificationError::Invalid {
                diagnostic_id: "p8.3-unknown-flow-id",
            })?;
        if !matches!(
            flow.phase,
            VerificationPhase::Requested | VerificationPhase::Ready
        ) {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-phase-transition",
            });
        }
        if let Some(ref emoji) = sas_emoji {
            if emoji.len() > MAX_SAS_EMOJI {
                return Err(VerificationError::Invalid {
                    diagnostic_id: "p8.3-sas-emoji-cap",
                });
            }
            for name in emoji {
                if name.is_empty() || name.len() > 32 {
                    return Err(VerificationError::Invalid {
                        diagnostic_id: "p8.3-invalid-sas-emoji",
                    });
                }
            }
            flow.sas_emoji = Some(emoji.clone());
        }
        flow.phase = VerificationPhase::Ready;
        flow.failure_diagnostic_id = None;
        Ok(())
    }

    pub fn confirm(&mut self, flow_id: &str) -> Result<(), VerificationError> {
        let flow = self
            .by_id
            .get_mut(flow_id)
            .ok_or(VerificationError::Invalid {
                diagnostic_id: "p8.3-unknown-flow-id",
            })?;
        if flow.phase != VerificationPhase::Ready {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-phase-transition",
            });
        }
        flow.phase = VerificationPhase::Confirmed;
        Ok(())
    }

    pub fn mismatch(&mut self, flow_id: &str) -> Result<(), VerificationError> {
        let flow = self
            .by_id
            .get_mut(flow_id)
            .ok_or(VerificationError::Invalid {
                diagnostic_id: "p8.3-unknown-flow-id",
            })?;
        if flow.phase != VerificationPhase::Ready {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-phase-transition",
            });
        }
        flow.phase = VerificationPhase::Mismatched;
        Ok(())
    }

    pub fn cancel(&mut self, flow_id: &str) -> Result<(), VerificationError> {
        let flow = self
            .by_id
            .get_mut(flow_id)
            .ok_or(VerificationError::Invalid {
                diagnostic_id: "p8.3-unknown-flow-id",
            })?;
        if !flow.phase.is_open() {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-phase-transition",
            });
        }
        flow.phase = VerificationPhase::Cancelled;
        // Drop display emoji on terminal cancel (no secret, but free memory).
        flow.sas_emoji = None;
        Ok(())
    }

    pub fn complete(&mut self, flow_id: &str) -> Result<(), VerificationError> {
        let flow = self
            .by_id
            .get_mut(flow_id)
            .ok_or(VerificationError::Invalid {
                diagnostic_id: "p8.3-unknown-flow-id",
            })?;
        if !matches!(
            flow.phase,
            VerificationPhase::Confirmed | VerificationPhase::Ready
        ) {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-phase-transition",
            });
        }
        flow.phase = VerificationPhase::Done;
        flow.sas_emoji = None;
        flow.failure_diagnostic_id = None;
        Ok(())
    }

    pub fn fail(
        &mut self,
        flow_id: &str,
        diagnostic_id: &'static str,
    ) -> Result<(), VerificationError> {
        let flow = self
            .by_id
            .get_mut(flow_id)
            .ok_or(VerificationError::Invalid {
                diagnostic_id: "p8.3-unknown-flow-id",
            })?;
        if !flow.phase.is_open() {
            return Err(VerificationError::Invalid {
                diagnostic_id: "p8.3-invalid-phase-transition",
            });
        }
        flow.phase = VerificationPhase::Failed;
        flow.sas_emoji = None;
        flow.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    pub fn remove(&mut self, flow_id: &str) -> Option<VerificationFlow> {
        self.by_id.remove(flow_id)
    }

    pub fn clear(&mut self) {
        self.by_id.clear();
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}

fn phase_rank(phase: VerificationPhase) -> u8 {
    match phase {
        VerificationPhase::Requested => 0,
        VerificationPhase::Ready => 1,
        VerificationPhase::Confirmed => 2,
        VerificationPhase::Mismatched => 3,
        VerificationPhase::Cancelled => 4,
        VerificationPhase::Done => 5,
        VerificationPhase::Failed => 6,
    }
}
