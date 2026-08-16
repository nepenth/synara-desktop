//! Key backup / recovery flow projection (P8.5 harness foundation).
//!
//! Pure state machine for setup / restore / repair UI. **Never stores recovery
//! keys, secrets, or key-backup private material.** Host holds secrets only in
//! transient secure UI; this module tracks phase + privacy-safe status enums.
//! No SDK crypto APIs, no dual-backend.

use crate::dto::{BackupStatus, RecoveryStatus};

use super::error::BackupError;

/// Which product flow the user is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupFlowKind {
    /// Create / enable recovery + key backup.
    Setup,
    /// Unlock secrets from recovery key / passphrase (no key stored here).
    Restore,
    /// Repair outdated / incomplete backup or recovery.
    Repair,
}

/// Lifecycle phase for the active flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupFlowPhase {
    Idle,
    /// User entered the flow; host may prompt for input.
    InProgress,
    /// Waiting on host/SDK async work (upload, download, verify).
    AwaitingHost,
    /// Flow completed successfully.
    Succeeded,
    /// Flow failed (diagnostic id only; no secret).
    Failed,
    /// User cancelled.
    Cancelled,
}

/// Session-generation-stamped backup / recovery flow store.
#[derive(Debug)]
pub struct BackupRecoveryFlow {
    session_generation: u64,
    kind: Option<BackupFlowKind>,
    phase: BackupFlowPhase,
    backup_status: BackupStatus,
    recovery_status: RecoveryStatus,
    /// Optional progress 0–100 for UI (no secret payload).
    progress_percent: Option<u8>,
    failure_diagnostic_id: Option<&'static str>,
    /// Monotonic op id so stale host completions can be ignored.
    op_id: u64,
}

impl BackupRecoveryFlow {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            kind: None,
            phase: BackupFlowPhase::Idle,
            backup_status: BackupStatus::Unknown,
            recovery_status: RecoveryStatus::Unknown,
            progress_percent: None,
            failure_diagnostic_id: None,
            op_id: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> BackupFlowPhase {
        self.phase
    }

    pub fn kind(&self) -> Option<BackupFlowKind> {
        self.kind
    }

    pub fn backup_status(&self) -> BackupStatus {
        self.backup_status
    }

    pub fn recovery_status(&self) -> RecoveryStatus {
        self.recovery_status
    }

    pub fn progress_percent(&self) -> Option<u8> {
        self.progress_percent
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn op_id(&self) -> u64 {
        self.op_id
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.phase,
            BackupFlowPhase::InProgress | BackupFlowPhase::AwaitingHost
        )
    }

    /// Apply projected backup/recovery status from host (no secrets).
    pub fn apply_status(
        &mut self,
        backup: BackupStatus,
        recovery: RecoveryStatus,
    ) -> Result<(), BackupError> {
        self.backup_status = backup;
        self.recovery_status = recovery;
        Ok(())
    }

    /// Begin a flow. Returns op_id for host to stamp on async work.
    pub fn begin(&mut self, kind: BackupFlowKind) -> Result<u64, BackupError> {
        if self.is_active() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-flow-already-active",
            });
        }
        self.op_id = self.op_id.saturating_add(1);
        self.kind = Some(kind);
        self.phase = BackupFlowPhase::InProgress;
        self.progress_percent = Some(0);
        self.failure_diagnostic_id = None;
        Ok(self.op_id)
    }

    /// Mark host/SDK work in flight (same op).
    pub fn mark_awaiting_host(&mut self, op_id: u64) -> Result<(), BackupError> {
        self.require_op(op_id)?;
        if !matches!(
            self.phase,
            BackupFlowPhase::InProgress | BackupFlowPhase::AwaitingHost
        ) {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-invalid-phase-transition",
            });
        }
        self.phase = BackupFlowPhase::AwaitingHost;
        Ok(())
    }

    pub fn set_progress(&mut self, op_id: u64, percent: u8) -> Result<(), BackupError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-invalid-phase-transition",
            });
        }
        if percent > 100 {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-progress-range",
            });
        }
        self.progress_percent = Some(percent);
        Ok(())
    }

    /// Complete successfully; host supplies resulting status enums only.
    pub fn succeed(
        &mut self,
        op_id: u64,
        backup: BackupStatus,
        recovery: RecoveryStatus,
    ) -> Result<(), BackupError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-invalid-phase-transition",
            });
        }
        self.phase = BackupFlowPhase::Succeeded;
        self.backup_status = backup;
        self.recovery_status = recovery;
        self.progress_percent = Some(100);
        self.failure_diagnostic_id = None;
        Ok(())
    }

    pub fn fail(&mut self, op_id: u64, diagnostic_id: &'static str) -> Result<(), BackupError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-invalid-phase-transition",
            });
        }
        if diagnostic_id.is_empty() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-empty-failure-id",
            });
        }
        self.phase = BackupFlowPhase::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        self.progress_percent = None;
        Ok(())
    }

    pub fn cancel(&mut self, op_id: u64) -> Result<(), BackupError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-invalid-phase-transition",
            });
        }
        self.phase = BackupFlowPhase::Cancelled;
        self.progress_percent = None;
        Ok(())
    }

    /// Reset to idle after terminal state (or force-clear cancelled/failed/succeeded).
    pub fn reset_to_idle(&mut self) -> Result<(), BackupError> {
        if self.is_active() {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-cannot-reset-active",
            });
        }
        self.kind = None;
        self.phase = BackupFlowPhase::Idle;
        self.progress_percent = None;
        self.failure_diagnostic_id = None;
        Ok(())
    }

    /// True when UI should nudge backup/recovery attention (not mid-flow).
    pub fn needs_attention(&self) -> bool {
        if self.is_active() {
            return false;
        }
        matches!(
            self.recovery_status,
            RecoveryStatus::NotSetup | RecoveryStatus::Incomplete
        ) || matches!(
            self.backup_status,
            BackupStatus::Outdated | BackupStatus::Disabled
        )
    }

    /// Bump generation and wipe flow state (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        *self = Self::new(new_generation);
    }

    fn require_op(&self, op_id: u64) -> Result<(), BackupError> {
        if op_id == 0 || op_id != self.op_id {
            return Err(BackupError::Invalid {
                diagnostic_id: "p8.5-stale-op-id",
            });
        }
        Ok(())
    }
}
