//! Post-login crypto bootstrap readiness coordinator (P8.9 harness).
//!
//! Tracks checklist flags for dogfood sole-owner flip readiness. **No keys,
//! recovery secrets, or tokens.** Host maps SDK crypto status → flags.
//! No dual-backend.

use super::error::CryptoBootstrapError;

/// Soft cap on pending step labels.
pub const MAX_PENDING_STEPS: usize = 16;

/// Soft cap on step label length (chars).
pub const MAX_STEP_LABEL_CHARS: usize = 64;

/// One bootstrap checklist item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BootstrapStep {
    /// Crypto store opened / healthy (not corrupted).
    StoreReady,
    /// Cross-signing identity present / trusted for own user.
    CrossSigningReady,
    /// Device list projected at least once.
    DeviceListReady,
    /// Key backup configured or intentionally skipped.
    BackupReady,
    /// Optional: initial verification inbox empty or acknowledged.
    VerificationSettled,
}

impl BootstrapStep {
    pub const ALL: &'static [BootstrapStep] = &[
        Self::StoreReady,
        Self::CrossSigningReady,
        Self::DeviceListReady,
        Self::BackupReady,
        Self::VerificationSettled,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::StoreReady => "store_ready",
            Self::CrossSigningReady => "cross_signing_ready",
            Self::DeviceListReady => "device_list_ready",
            Self::BackupReady => "backup_ready",
            Self::VerificationSettled => "verification_settled",
        }
    }
}

/// Overall bootstrap phase for UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapPhase {
    /// Not started (pre-login or cleared).
    Idle,
    /// Host is running bootstrap work.
    InProgress,
    /// All required steps satisfied.
    Ready,
    /// Failed with diagnostic (no secrets).
    Failed,
    /// User skipped optional recovery; required steps still may be ready.
    Degraded,
}

/// Session-generation-stamped crypto bootstrap coordinator.
#[derive(Debug)]
pub struct CryptoBootstrapCoordinator {
    session_generation: u64,
    phase: BootstrapPhase,
    store_ready: bool,
    cross_signing_ready: bool,
    device_list_ready: bool,
    backup_ready: bool,
    verification_settled: bool,
    /// When true, backup_ready is not required for Ready (user skip).
    backup_optional: bool,
    /// When true, verification_settled is not required for Ready.
    verification_optional: bool,
    failure_diagnostic_id: Option<&'static str>,
    /// Ordered privacy-safe pending step labels for UI.
    pending_labels: Vec<String>,
}

impl CryptoBootstrapCoordinator {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            phase: BootstrapPhase::Idle,
            store_ready: false,
            cross_signing_ready: false,
            device_list_ready: false,
            backup_ready: false,
            verification_settled: false,
            backup_optional: false,
            verification_optional: true,
            failure_diagnostic_id: None,
            pending_labels: Vec::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> BootstrapPhase {
        self.phase
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn is_step_ready(&self, step: BootstrapStep) -> bool {
        match step {
            BootstrapStep::StoreReady => self.store_ready,
            BootstrapStep::CrossSigningReady => self.cross_signing_ready,
            BootstrapStep::DeviceListReady => self.device_list_ready,
            BootstrapStep::BackupReady => self.backup_ready,
            BootstrapStep::VerificationSettled => self.verification_settled,
        }
    }

    pub fn pending_labels(&self) -> &[String] {
        &self.pending_labels
    }

    /// Begin bootstrap after login / session restore.
    pub fn begin(&mut self) -> Result<(), CryptoBootstrapError> {
        if matches!(self.phase, BootstrapPhase::InProgress) {
            return Err(CryptoBootstrapError::Invalid {
                diagnostic_id: "p8.9-already-in-progress",
            });
        }
        self.phase = BootstrapPhase::InProgress;
        self.failure_diagnostic_id = None;
        self.recompute_pending();
        Ok(())
    }

    pub fn set_step(
        &mut self,
        step: BootstrapStep,
        ready: bool,
    ) -> Result<(), CryptoBootstrapError> {
        if self.phase == BootstrapPhase::Idle {
            return Err(CryptoBootstrapError::Invalid {
                diagnostic_id: "p8.9-not-started",
            });
        }
        match step {
            BootstrapStep::StoreReady => self.store_ready = ready,
            BootstrapStep::CrossSigningReady => self.cross_signing_ready = ready,
            BootstrapStep::DeviceListReady => self.device_list_ready = ready,
            BootstrapStep::BackupReady => self.backup_ready = ready,
            BootstrapStep::VerificationSettled => self.verification_settled = ready,
        }
        self.recompute_phase();
        Ok(())
    }

    /// Mark backup as optional (user skipped recovery setup).
    pub fn set_backup_optional(&mut self, optional: bool) {
        self.backup_optional = optional;
        if self.phase != BootstrapPhase::Idle {
            self.recompute_phase();
        }
    }

    pub fn set_verification_optional(&mut self, optional: bool) {
        self.verification_optional = optional;
        if self.phase != BootstrapPhase::Idle {
            self.recompute_phase();
        }
    }

    pub fn fail(&mut self, diagnostic_id: &'static str) -> Result<(), CryptoBootstrapError> {
        if diagnostic_id.is_empty() {
            return Err(CryptoBootstrapError::Invalid {
                diagnostic_id: "p8.9-empty-diagnostic",
            });
        }
        if self.phase == BootstrapPhase::Idle {
            return Err(CryptoBootstrapError::Invalid {
                diagnostic_id: "p8.9-not-started",
            });
        }
        self.phase = BootstrapPhase::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    /// True when required steps are satisfied (Ready or Degraded).
    pub fn is_dogfood_ready(&self) -> bool {
        matches!(self.phase, BootstrapPhase::Ready | BootstrapPhase::Degraded)
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        *self = Self::new(new_generation);
    }

    fn required_ok(&self) -> bool {
        self.store_ready
            && self.cross_signing_ready
            && self.device_list_ready
            && (self.backup_ready || self.backup_optional)
            && (self.verification_settled || self.verification_optional)
    }

    fn recompute_phase(&mut self) {
        if matches!(self.phase, BootstrapPhase::Failed | BootstrapPhase::Idle) {
            self.recompute_pending();
            return;
        }
        if self.required_ok() {
            self.phase = if self.backup_optional && !self.backup_ready {
                BootstrapPhase::Degraded
            } else {
                BootstrapPhase::Ready
            };
            self.failure_diagnostic_id = None;
        } else {
            self.phase = BootstrapPhase::InProgress;
        }
        self.recompute_pending();
    }

    fn recompute_pending(&mut self) {
        self.pending_labels.clear();
        for step in BootstrapStep::ALL {
            let optional = match step {
                BootstrapStep::BackupReady => self.backup_optional,
                BootstrapStep::VerificationSettled => self.verification_optional,
                _ => false,
            };
            if !self.is_step_ready(*step) && !optional {
                let label = step.as_str().to_owned();
                if label.chars().count() <= MAX_STEP_LABEL_CHARS
                    && self.pending_labels.len() < MAX_PENDING_STEPS
                {
                    self.pending_labels.push(label);
                }
            }
        }
    }
}
