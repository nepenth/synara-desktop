//! Room-key import/export flow (P8.6 harness foundation).
//!
//! Pure state machine for export/import UI. **Never stores megolm session keys,
//! passphrases, or file contents.** Host holds material only in transient secure
//! paths; this module tracks phase + counts + privacy-safe diagnostics.
//! No SDK crypto APIs, no dual-backend.

use super::error::RoomKeyError;

/// Transfer direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomKeyTransferKind {
    /// Export room keys to an encrypted file (host encrypts; we track only).
    Export,
    /// Import room keys from an encrypted file.
    Import,
}

impl RoomKeyTransferKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Import => "import",
        }
    }
}

/// Lifecycle phase for an import/export op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomKeyTransferPhase {
    Idle,
    /// Collecting passphrase / path in host UI (no secrets here).
    Preparing,
    /// Host/SDK work in flight.
    InFlight,
    Succeeded,
    Failed,
    Cancelled,
}

impl RoomKeyTransferPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Preparing => "preparing",
            Self::InFlight => "in_flight",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Preparing | Self::InFlight)
    }
}

/// Privacy-safe outcome summary (counts only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomKeyTransferOutcome {
    pub kind: RoomKeyTransferKind,
    /// How many room keys the host reported exported/imported.
    pub keys_processed: u32,
    /// How many rooms touched (optional host metric).
    pub rooms_touched: u32,
}

/// Session-generation-stamped room-key transfer coordinator.
#[derive(Debug)]
pub struct RoomKeyTransferFlow {
    session_generation: u64,
    kind: Option<RoomKeyTransferKind>,
    phase: RoomKeyTransferPhase,
    progress_percent: Option<u8>,
    keys_processed: u32,
    rooms_touched: u32,
    /// Opaque host file handle / path **label** only (basename ok); never key material.
    file_label: Option<String>,
    failure_diagnostic_id: Option<&'static str>,
    op_id: u64,
}

impl RoomKeyTransferFlow {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            kind: None,
            phase: RoomKeyTransferPhase::Idle,
            progress_percent: None,
            keys_processed: 0,
            rooms_touched: 0,
            file_label: None,
            failure_diagnostic_id: None,
            op_id: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> RoomKeyTransferPhase {
        self.phase
    }

    pub fn kind(&self) -> Option<RoomKeyTransferKind> {
        self.kind
    }

    pub fn progress_percent(&self) -> Option<u8> {
        self.progress_percent
    }

    pub fn keys_processed(&self) -> u32 {
        self.keys_processed
    }

    pub fn rooms_touched(&self) -> u32 {
        self.rooms_touched
    }

    pub fn file_label(&self) -> Option<&str> {
        self.file_label.as_deref()
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    pub fn op_id(&self) -> u64 {
        self.op_id
    }

    pub fn is_active(&self) -> bool {
        self.phase.is_active()
    }

    /// Begin transfer. `file_label` is a display basename only (no directory secrets).
    pub fn begin(
        &mut self,
        kind: RoomKeyTransferKind,
        file_label: Option<String>,
    ) -> Result<u64, RoomKeyError> {
        if self.is_active() {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-transfer-already-active",
            });
        }
        if let Some(ref label) = file_label {
            validate_file_label(label)?;
        }
        self.op_id = self.op_id.saturating_add(1);
        self.kind = Some(kind);
        self.phase = RoomKeyTransferPhase::Preparing;
        self.progress_percent = Some(0);
        self.keys_processed = 0;
        self.rooms_touched = 0;
        self.file_label = file_label;
        self.failure_diagnostic_id = None;
        Ok(self.op_id)
    }

    pub fn mark_in_flight(&mut self, op_id: u64) -> Result<(), RoomKeyError> {
        self.require_op(op_id)?;
        if !matches!(
            self.phase,
            RoomKeyTransferPhase::Preparing | RoomKeyTransferPhase::InFlight
        ) {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-invalid-phase",
            });
        }
        self.phase = RoomKeyTransferPhase::InFlight;
        Ok(())
    }

    pub fn set_progress(&mut self, op_id: u64, percent: u8) -> Result<(), RoomKeyError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-invalid-phase",
            });
        }
        if percent > 100 {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-progress-range",
            });
        }
        self.progress_percent = Some(percent);
        Ok(())
    }

    pub fn succeed(
        &mut self,
        op_id: u64,
        outcome: RoomKeyTransferOutcome,
    ) -> Result<(), RoomKeyError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-invalid-phase",
            });
        }
        if Some(outcome.kind) != self.kind {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-kind-mismatch",
            });
        }
        self.phase = RoomKeyTransferPhase::Succeeded;
        self.keys_processed = outcome.keys_processed;
        self.rooms_touched = outcome.rooms_touched;
        self.progress_percent = Some(100);
        Ok(())
    }

    pub fn fail(&mut self, op_id: u64, diagnostic_id: &'static str) -> Result<(), RoomKeyError> {
        self.require_op(op_id)?;
        validate_diagnostic(diagnostic_id)?;
        if !self.is_active() {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-invalid-phase",
            });
        }
        self.phase = RoomKeyTransferPhase::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    pub fn cancel(&mut self, op_id: u64) -> Result<(), RoomKeyError> {
        self.require_op(op_id)?;
        if !self.is_active() {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-invalid-phase",
            });
        }
        self.phase = RoomKeyTransferPhase::Cancelled;
        Ok(())
    }

    pub fn reset_to_idle(&mut self) -> Result<(), RoomKeyError> {
        if self.is_active() {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-still-active",
            });
        }
        self.kind = None;
        self.phase = RoomKeyTransferPhase::Idle;
        self.progress_percent = None;
        self.keys_processed = 0;
        self.rooms_touched = 0;
        self.file_label = None;
        self.failure_diagnostic_id = None;
        Ok(())
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        let was_active = self.is_active();
        self.session_generation = new_generation;
        self.kind = None;
        self.progress_percent = None;
        self.keys_processed = 0;
        self.rooms_touched = 0;
        self.file_label = None;
        self.op_id = 0;
        if was_active {
            self.phase = RoomKeyTransferPhase::Failed;
            self.failure_diagnostic_id = Some("p8.6-stale-generation-cancelled");
        } else {
            self.phase = RoomKeyTransferPhase::Idle;
            self.failure_diagnostic_id = None;
        }
    }

    fn require_op(&self, op_id: u64) -> Result<(), RoomKeyError> {
        if op_id != self.op_id || op_id == 0 {
            return Err(RoomKeyError::Invalid {
                diagnostic_id: "p8.6-stale-op-id",
            });
        }
        Ok(())
    }
}

fn validate_file_label(label: &str) -> Result<(), RoomKeyError> {
    if label.is_empty() || label.len() > 256 {
        return Err(RoomKeyError::Invalid {
            diagnostic_id: "p8.6-invalid-file-label",
        });
    }
    // Basename only — reject path separators and secret-looking strings.
    if label.contains('/') || label.contains('\\') || label.contains("..") {
        return Err(RoomKeyError::Invalid {
            diagnostic_id: "p8.6-file-label-not-basename",
        });
    }
    let lower = label.to_ascii_lowercase();
    if lower.contains("access_token") || lower.contains("session_key") {
        return Err(RoomKeyError::Invalid {
            diagnostic_id: "p8.6-forbidden-file-label",
        });
    }
    Ok(())
}

fn validate_diagnostic(diagnostic_id: &'static str) -> Result<(), RoomKeyError> {
    if diagnostic_id.is_empty() {
        return Err(RoomKeyError::Invalid {
            diagnostic_id: "p8.6-empty-diagnostic",
        });
    }
    let lower = diagnostic_id.to_ascii_lowercase();
    if lower.contains("access_token")
        || lower.contains("session_key")
        || lower.contains("password")
        || lower.contains("private")
        || lower.contains("megolm")
    {
        return Err(RoomKeyError::Invalid {
            diagnostic_id: "p8.6-forbidden-diagnostic",
        });
    }
    Ok(())
}
