//! Crypto-store continuity and corruption handling (P8.8 harness foundation).
//!
//! Tracks open/health/continuity phases for the encrypted Matrix crypto store.
//! **Never auto-wipes.** **Never stores keys or recovery material.** Complements
//! P2.6 store failure recovery and P2.2 store paths. No dual-backend.

use super::error::CryptoStoreError;

/// Observed crypto-store health (privacy-safe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CryptoStoreHealth {
    Unknown,
    Healthy,
    Degraded,
    Locked,
    Corrupt,
    Unavailable,
    Missing,
}

impl CryptoStoreHealth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Locked => "locked",
            Self::Corrupt => "corrupt",
            Self::Unavailable => "unavailable",
            Self::Missing => "missing",
        }
    }

    pub fn is_usable(self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded | Self::Unknown)
    }
}

/// Continuity lifecycle for the crypto store handle (no SDK open here).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CryptoStorePhase {
    /// Not opened this session.
    Closed,
    /// Host is opening / migrating store.
    Opening,
    /// Open and ready for crypto operations.
    Ready,
    /// Open but health degraded (retryable).
    Degraded,
    /// Terminal failure for this generation (no auto-wipe).
    Failed,
}

impl CryptoStorePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Opening => "opening",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
        }
    }

    pub fn is_open(self) -> bool {
        matches!(self, Self::Ready | Self::Degraded | Self::Opening)
    }
}

/// Recommended operator action (never auto-wipe).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CryptoStoreAction {
    None,
    RetryOpen,
    WaitUnlock,
    /// Surface recovery UI; user must explicitly choose wipe/reauth (P2.6/P3.7).
    OfferManualRecovery,
    /// Store missing for new device — create fresh (host).
    CreateFresh,
}

impl CryptoStoreAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RetryOpen => "retry_open",
            Self::WaitUnlock => "wait_unlock",
            Self::OfferManualRecovery => "offer_manual_recovery",
            Self::CreateFresh => "create_fresh",
        }
    }

    pub fn requests_wipe(self) -> bool {
        false
    }
}

/// Session-generation-stamped crypto-store continuity tracker.
#[derive(Debug, Clone)]
pub struct CryptoStoreContinuity {
    session_generation: u64,
    phase: CryptoStorePhase,
    health: CryptoStoreHealth,
    /// How many successful open→ready cycles this generation.
    open_count: u32,
    /// How many continuity checks observed Ready after restart-like reopen.
    continuity_ok_count: u32,
    failure_diagnostic_id: Option<&'static str>,
    /// True when host reported a clean reopen with same identity (no wipe).
    last_reopen_continuous: bool,
}

impl CryptoStoreContinuity {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            phase: CryptoStorePhase::Closed,
            health: CryptoStoreHealth::Unknown,
            open_count: 0,
            continuity_ok_count: 0,
            failure_diagnostic_id: None,
            last_reopen_continuous: false,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> CryptoStorePhase {
        self.phase
    }

    pub fn health(&self) -> CryptoStoreHealth {
        self.health
    }

    pub fn open_count(&self) -> u32 {
        self.open_count
    }

    pub fn continuity_ok_count(&self) -> u32 {
        self.continuity_ok_count
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn last_reopen_continuous(&self) -> bool {
        self.last_reopen_continuous
    }

    /// Begin opening the crypto store (host performs SDK open).
    pub fn begin_open(&mut self) -> Result<(), CryptoStoreError> {
        if matches!(
            self.phase,
            CryptoStorePhase::Opening | CryptoStorePhase::Ready
        ) {
            return Err(CryptoStoreError::Invalid {
                diagnostic_id: "p8.8-already-open-or-opening",
            });
        }
        self.phase = CryptoStorePhase::Opening;
        self.failure_diagnostic_id = None;
        self.last_reopen_continuous = false;
        Ok(())
    }

    /// Mark store ready after successful open.
    pub fn mark_ready(&mut self, continuous: bool) -> Result<(), CryptoStoreError> {
        if self.phase != CryptoStorePhase::Opening && self.phase != CryptoStorePhase::Degraded {
            return Err(CryptoStoreError::Invalid {
                diagnostic_id: "p8.8-mark-ready-invalid-phase",
            });
        }
        self.phase = CryptoStorePhase::Ready;
        self.health = CryptoStoreHealth::Healthy;
        self.open_count = self.open_count.saturating_add(1);
        self.last_reopen_continuous = continuous;
        if continuous {
            self.continuity_ok_count = self.continuity_ok_count.saturating_add(1);
        }
        self.failure_diagnostic_id = None;
        Ok(())
    }

    /// Host reports degraded but usable store.
    pub fn mark_degraded(&mut self, diagnostic_id: &'static str) -> Result<(), CryptoStoreError> {
        validate_diagnostic(diagnostic_id)?;
        if !self.phase.is_open() && self.phase != CryptoStorePhase::Opening {
            return Err(CryptoStoreError::Invalid {
                diagnostic_id: "p8.8-degraded-not-open",
            });
        }
        self.phase = CryptoStorePhase::Degraded;
        self.health = CryptoStoreHealth::Degraded;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    /// Fail open/continuity check. **Never wipes.**
    pub fn fail(
        &mut self,
        health: CryptoStoreHealth,
        diagnostic_id: &'static str,
    ) -> Result<CryptoStoreAction, CryptoStoreError> {
        validate_diagnostic(diagnostic_id)?;
        if matches!(health, CryptoStoreHealth::Healthy) {
            return Err(CryptoStoreError::Invalid {
                diagnostic_id: "p8.8-fail-requires-bad-health",
            });
        }
        self.phase = CryptoStorePhase::Failed;
        self.health = health;
        self.failure_diagnostic_id = Some(diagnostic_id);
        self.last_reopen_continuous = false;
        Ok(self.recommended_action())
    }

    pub fn close(&mut self) {
        self.phase = CryptoStorePhase::Closed;
        // retain health for UI unless healthy
        if self.health == CryptoStoreHealth::Healthy {
            self.health = CryptoStoreHealth::Unknown;
        }
        self.failure_diagnostic_id = None;
    }

    /// Pure policy: what the host/UI should offer next (never wipe).
    pub fn recommended_action(&self) -> CryptoStoreAction {
        match (self.phase, self.health) {
            (CryptoStorePhase::Ready, _) => CryptoStoreAction::None,
            (CryptoStorePhase::Opening, _) => CryptoStoreAction::None,
            (CryptoStorePhase::Degraded, _) => CryptoStoreAction::RetryOpen,
            (CryptoStorePhase::Failed, CryptoStoreHealth::Locked) => CryptoStoreAction::WaitUnlock,
            (CryptoStorePhase::Failed, CryptoStoreHealth::Missing) => {
                CryptoStoreAction::CreateFresh
            }
            (CryptoStorePhase::Failed, CryptoStoreHealth::Corrupt)
            | (CryptoStorePhase::Failed, CryptoStoreHealth::Unavailable)
            | (CryptoStorePhase::Failed, CryptoStoreHealth::Degraded) => {
                CryptoStoreAction::OfferManualRecovery
            }
            (CryptoStorePhase::Closed, CryptoStoreHealth::Missing) => {
                CryptoStoreAction::CreateFresh
            }
            (CryptoStorePhase::Closed, _) => CryptoStoreAction::RetryOpen,
            _ => CryptoStoreAction::RetryOpen,
        }
    }

    /// Hard invariant used in tests and guardrails.
    pub fn never_auto_wipes(&self) -> bool {
        !self.recommended_action().requests_wipe()
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.phase = CryptoStorePhase::Closed;
        self.health = CryptoStoreHealth::Unknown;
        self.open_count = 0;
        self.continuity_ok_count = 0;
        self.failure_diagnostic_id = None;
        self.last_reopen_continuous = false;
    }
}

fn validate_diagnostic(diagnostic_id: &'static str) -> Result<(), CryptoStoreError> {
    if diagnostic_id.is_empty() {
        return Err(CryptoStoreError::Invalid {
            diagnostic_id: "p8.8-empty-diagnostic",
        });
    }
    let lower = diagnostic_id.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("session_key")
        || lower.contains("password")
        || lower.contains("private")
        || lower.contains("recovery") && lower.contains("key") && lower.contains('=')
    {
        return Err(CryptoStoreError::Invalid {
            diagnostic_id: "p8.8-forbidden-diagnostic",
        });
    }
    Ok(())
}
