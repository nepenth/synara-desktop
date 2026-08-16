//! Security / crypto status projection (P8.1 harness foundation).
//!
//! Pure host-side view of Synara [`SecurityStatus`] DTOs. **No keys, recovery
//! material, or secrets** — status enums and counts only. No dual-backend.

use crate::dto::{BackupStatus, RecoveryStatus, SecurityStatus, VerificationState};

use super::error::SecurityError;

/// Session-generation-stamped security status store.
#[derive(Debug, Clone)]
pub struct SecurityStatusStore {
    session_generation: u64,
    status: SecurityStatus,
}

impl SecurityStatusStore {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            status: SecurityStatus {
                cross_signing_active: false,
                backup_status: BackupStatus::Unknown,
                recovery_status: RecoveryStatus::Unknown,
                verification_state: VerificationState::Unavailable,
                device_count: None,
                has_pending_verification_requests: false,
            },
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn snapshot(&self) -> &SecurityStatus {
        &self.status
    }

    /// Replace the full security status projection (host maps SDK → DTO).
    pub fn apply(&mut self, status: SecurityStatus) -> Result<(), SecurityError> {
        if let Some(n) = status.device_count {
            // Soft sanity: absurd device counts rejected (not a privacy leak).
            if n > 10_000 {
                return Err(SecurityError::Invalid {
                    diagnostic_id: "p8.1-device-count-cap",
                });
            }
        }
        self.status = status;
        Ok(())
    }

    pub fn set_verification_state(&mut self, state: VerificationState) {
        self.status.verification_state = state;
    }

    pub fn set_backup_status(&mut self, status: BackupStatus) {
        self.status.backup_status = status;
    }

    pub fn set_recovery_status(&mut self, status: RecoveryStatus) {
        self.status.recovery_status = status;
    }

    pub fn set_pending_verification_requests(&mut self, pending: bool) {
        self.status.has_pending_verification_requests = pending;
    }

    pub fn set_cross_signing_active(&mut self, active: bool) {
        self.status.cross_signing_active = active;
    }

    pub fn set_device_count(&mut self, count: Option<u32>) -> Result<(), SecurityError> {
        if let Some(n) = count {
            if n > 10_000 {
                return Err(SecurityError::Invalid {
                    diagnostic_id: "p8.1-device-count-cap",
                });
            }
        }
        self.status.device_count = count;
        Ok(())
    }

    /// True when UI should show a “needs attention” security banner.
    pub fn needs_attention(&self) -> bool {
        self.status.has_pending_verification_requests
            || matches!(
                self.status.verification_state,
                VerificationState::Unverified
            )
            || matches!(
                self.status.recovery_status,
                RecoveryStatus::NotSetup | RecoveryStatus::Incomplete
            )
            || matches!(self.status.backup_status, BackupStatus::Outdated)
    }

    /// Bump generation and reset to unknown (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        *self = Self::new(new_generation);
    }
}
