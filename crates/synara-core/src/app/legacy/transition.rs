//! Legacy-session detection and transition coordinator (P3.7 harness foundation).
//!
//! Pure state machine implementing migration-ux policy:
//! - Detect legacy via **inert** signals only — never start `matrix-js-sdk`
//! - Reauth required; **no** token/device continuity into a fresh crypto store
//! - Failed transition leaves legacy data intact and offers retry
//! - No dual-backend / runtime selector
//!
//! See `docs/matrix-rust-sdk/migration-ux-decision.md` and `p3.7-legacy-transition.md`.

use super::error::LegacyError;

/// Soft cap on inert detection signal names retained for diagnostics UI.
pub const MAX_DETECTION_SIGNALS: usize = 32;

/// Known inert legacy signal kinds (product names, not secrets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LegacySignalKind {
    /// IndexedDB / web sync store name observed.
    WebSyncStore,
    /// JS crypto store name observed.
    JsCryptoStore,
    /// Fallback credential envelope / keyring layout marker.
    LegacyCredentialEnvelope,
    /// Explicit cutover-not-complete marker file.
    CutoverMarkerAbsent,
    /// Other inert host-reported signal (opaque product id).
    Other,
}

/// One inert detection signal (no tokens, no store contents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyDetectionSignal {
    pub kind: LegacySignalKind,
    /// Short product label / path basename only (never secret payload).
    pub label: String,
}

/// Transition lifecycle (clean-break reauth path).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionPhase {
    /// No transition active; may or may not have detected legacy.
    Idle,
    /// Legacy signals present; user must reauthenticate (new device).
    AwaitingReauth,
    /// User started reauth; host performing login (no dual client).
    Reauthing,
    /// Rust session + store opening after reauth.
    EstablishingRustSession,
    /// Transition completed; Rust is sole owner candidate.
    Complete,
    /// Failed; legacy data must remain intact.
    Failed,
    /// User dismissed / postponed.
    Deferred,
}

/// Product-facing copy keys (UI maps to localized strings; never secrets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionCopyKey {
    OneTimeSignInRequired,
    NewDeviceWillBeCreated,
    RecoveryMayBeRequired,
    LegacyDataPreservedOnFailure,
    DualBackendNotSupported,
}

/// Session-generation-stamped legacy transition coordinator.
#[derive(Debug)]
pub struct LegacyTransition {
    session_generation: u64,
    phase: TransitionPhase,
    signals: Vec<LegacyDetectionSignal>,
    legacy_detected: bool,
    /// True when host marked legacy data still on disk (inert).
    legacy_data_retained: bool,
    failure_diagnostic_id: Option<&'static str>,
    /// Monotonic op for stale host completion protection.
    op_id: u64,
}

impl LegacyTransition {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            phase: TransitionPhase::Idle,
            signals: Vec::new(),
            legacy_detected: false,
            legacy_data_retained: false,
            failure_diagnostic_id: None,
            op_id: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> TransitionPhase {
        self.phase
    }

    pub fn legacy_detected(&self) -> bool {
        self.legacy_detected
    }

    pub fn legacy_data_retained(&self) -> bool {
        self.legacy_data_retained
    }

    pub fn signals(&self) -> &[LegacyDetectionSignal] {
        &self.signals
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn op_id(&self) -> u64 {
        self.op_id
    }

    /// Policy: never start JS Matrix client from this coordinator.
    pub fn forbids_js_client_start(&self) -> bool {
        true
    }

    /// Policy: never copy tokens/device into fresh crypto store.
    pub fn forbids_token_continuity(&self) -> bool {
        true
    }

    /// Policy: no dual-backend selector.
    pub fn forbids_dual_backend(&self) -> bool {
        true
    }

    pub fn copy_keys_for_phase(&self) -> Vec<TransitionCopyKey> {
        match self.phase {
            TransitionPhase::AwaitingReauth | TransitionPhase::Reauthing => vec![
                TransitionCopyKey::OneTimeSignInRequired,
                TransitionCopyKey::NewDeviceWillBeCreated,
                TransitionCopyKey::RecoveryMayBeRequired,
                TransitionCopyKey::DualBackendNotSupported,
            ],
            TransitionPhase::Failed => vec![
                TransitionCopyKey::LegacyDataPreservedOnFailure,
                TransitionCopyKey::OneTimeSignInRequired,
            ],
            TransitionPhase::EstablishingRustSession => {
                vec![TransitionCopyKey::RecoveryMayBeRequired]
            }
            _ => Vec::new(),
        }
    }

    /// Replace inert detection signals from host scan (no store contents).
    pub fn apply_detection(
        &mut self,
        signals: Vec<LegacyDetectionSignal>,
        legacy_data_retained: bool,
    ) -> Result<(), LegacyError> {
        if signals.len() > MAX_DETECTION_SIGNALS {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-signal-cap",
            });
        }
        for s in &signals {
            if s.label.is_empty() || s.label.len() > 128 {
                return Err(LegacyError::Invalid {
                    diagnostic_id: "p3.7-invalid-signal-label",
                });
            }
            // Reject labels that look like secrets (heuristic only).
            let lower = s.label.to_ascii_lowercase();
            if lower.contains("syt_")
                || lower.contains("access_token")
                || lower.contains("refresh_token")
            {
                return Err(LegacyError::Invalid {
                    diagnostic_id: "p3.7-signal-looks-like-secret",
                });
            }
        }
        self.signals = signals;
        self.legacy_detected = !self.signals.is_empty();
        self.legacy_data_retained = legacy_data_retained;
        if self.legacy_detected && matches!(self.phase, TransitionPhase::Idle) {
            self.phase = TransitionPhase::AwaitingReauth;
            self.failure_diagnostic_id = None;
        }
        if !self.legacy_detected && matches!(self.phase, TransitionPhase::AwaitingReauth) {
            self.phase = TransitionPhase::Idle;
        }
        Ok(())
    }

    /// Begin reauth (returns op_id). Does not start any Matrix client.
    pub fn begin_reauth(&mut self) -> Result<u64, LegacyError> {
        if !matches!(
            self.phase,
            TransitionPhase::AwaitingReauth | TransitionPhase::Failed | TransitionPhase::Deferred
        ) {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-invalid-phase-transition",
            });
        }
        if !self.legacy_detected {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-no-legacy-detected",
            });
        }
        self.op_id = self.op_id.saturating_add(1);
        self.phase = TransitionPhase::Reauthing;
        self.failure_diagnostic_id = None;
        Ok(self.op_id)
    }

    pub fn mark_establishing(&mut self, op_id: u64) -> Result<(), LegacyError> {
        self.require_op(op_id)?;
        if self.phase != TransitionPhase::Reauthing {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-invalid-phase-transition",
            });
        }
        self.phase = TransitionPhase::EstablishingRustSession;
        Ok(())
    }

    pub fn complete(&mut self, op_id: u64) -> Result<(), LegacyError> {
        self.require_op(op_id)?;
        if !matches!(
            self.phase,
            TransitionPhase::Reauthing | TransitionPhase::EstablishingRustSession
        ) {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-invalid-phase-transition",
            });
        }
        self.phase = TransitionPhase::Complete;
        self.failure_diagnostic_id = None;
        // Legacy may still be retained inert until explicit cleanup (P3.8/P14).
        Ok(())
    }

    /// Fail transition; **must** keep legacy_data_retained true if it was.
    pub fn fail(&mut self, op_id: u64, diagnostic_id: &'static str) -> Result<(), LegacyError> {
        self.require_op(op_id)?;
        if !matches!(
            self.phase,
            TransitionPhase::Reauthing | TransitionPhase::EstablishingRustSession
        ) {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-invalid-phase-transition",
            });
        }
        if diagnostic_id.is_empty() {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-empty-failure-id",
            });
        }
        self.phase = TransitionPhase::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        // Do not clear legacy_data_retained — D-FAILURE.
        Ok(())
    }

    pub fn defer(&mut self) -> Result<(), LegacyError> {
        if !matches!(
            self.phase,
            TransitionPhase::AwaitingReauth | TransitionPhase::Failed
        ) {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-invalid-phase-transition",
            });
        }
        self.phase = TransitionPhase::Deferred;
        Ok(())
    }

    /// After complete, host may mark inert legacy cleaned up (optional).
    pub fn mark_legacy_cleaned(&mut self) -> Result<(), LegacyError> {
        if self.phase != TransitionPhase::Complete {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-cleanup-only-after-complete",
            });
        }
        self.legacy_data_retained = false;
        self.signals.clear();
        self.legacy_detected = false;
        Ok(())
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        *self = Self::new(new_generation);
    }

    fn require_op(&self, op_id: u64) -> Result<(), LegacyError> {
        if op_id == 0 || op_id != self.op_id {
            return Err(LegacyError::Invalid {
                diagnostic_id: "p3.7-stale-op-id",
            });
        }
        Ok(())
    }
}
