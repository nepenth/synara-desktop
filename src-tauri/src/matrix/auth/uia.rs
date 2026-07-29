//! Interactive Auth (UIA) session coordinator (P3.4 harness foundation).
//!
//! Pure state machine for multi-stage Matrix UIA (login registration, password
//! reset, and step-up). **Never stores passwords, tokens, captcha solutions,
//! email codes, or MSISDN secrets** — only stage kinds, opaque session ids, and
//! privacy-safe phase. No dual-backend, no production Tauri commands.

use super::error::AuthError;

/// Soft cap on stages advertised in one UIA session.
pub const MAX_UIA_STAGES: usize = 16;

/// Soft cap on opaque session / flow id length.
pub const MAX_UIA_ID_CHARS: usize = 256;

/// Which product operation needs UIA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiaFlowKind {
    /// Completing login after homeserver returns 401 + flows.
    Login,
    /// Account registration.
    Registration,
    /// Password reset / forgot-password.
    PasswordReset,
    /// Step-up re-auth for a privileged action.
    StepUp,
}

impl UiaFlowKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Registration => "registration",
            Self::PasswordReset => "password_reset",
            Self::StepUp => "step_up",
        }
    }
}

/// Stable stage kinds (not raw SDK types on the boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiaStageKind {
    Password,
    Recaptcha,
    EmailIdentity,
    Msisdn,
    Terms,
    Dummy,
    RegistrationToken,
    /// Unrecognized `m.login.*` stage.
    Unknown,
}

impl UiaStageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::Recaptcha => "recaptcha",
            Self::EmailIdentity => "email_identity",
            Self::Msisdn => "msisdn",
            Self::Terms => "terms",
            Self::Dummy => "dummy",
            Self::RegistrationToken => "registration_token",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_matrix_type(matrix_type: &str) -> Self {
        match matrix_type {
            "m.login.password" => Self::Password,
            "m.login.recaptcha" => Self::Recaptcha,
            "m.login.email.identity" => Self::EmailIdentity,
            "m.login.msisdn" => Self::Msisdn,
            "m.login.terms" => Self::Terms,
            "m.login.dummy" => Self::Dummy,
            "m.login.registration_token" => Self::RegistrationToken,
            _ => Self::Unknown,
        }
    }

    /// Stages that require a user secret — host must never persist the secret here.
    pub fn requires_secret_input(self) -> bool {
        matches!(
            self,
            Self::Password
                | Self::Recaptcha
                | Self::EmailIdentity
                | Self::Msisdn
                | Self::RegistrationToken
        )
    }
}

/// Lifecycle of one UIA attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiaPhase {
    Idle,
    /// Homeserver advertised flows; user must complete a stage.
    ChallengePending,
    /// Host is submitting a stage response (network in flight).
    Submitting,
    /// All required stages completed for this session.
    Completed,
    Failed,
    Cancelled,
}

impl UiaPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::ChallengePending => "challenge_pending",
            Self::Submitting => "submitting",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::ChallengePending | Self::Submitting)
    }
}

/// One advertised stage (no secret fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiaStage {
    pub kind: UiaStageKind,
    /// Original Matrix type string when known.
    pub matrix_type: String,
    /// Optional public params (e.g. recaptcha public key id) — never private keys.
    pub public_param_id: Option<String>,
}

/// Privacy-safe completed UIA outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiaOutcome {
    pub session_generation: u64,
    pub op_id: u64,
    pub flow_kind: UiaFlowKind,
    pub stages_completed: u32,
}

/// Session-generation-stamped UIA coordinator.
#[derive(Debug, Clone, PartialEq)]
pub struct UiaSession {
    session_generation: u64,
    phase: UiaPhase,
    flow_kind: Option<UiaFlowKind>,
    /// Opaque homeserver UIA session string (not an access token).
    uia_session_id: Option<String>,
    stages: Vec<UiaStage>,
    /// Index of current stage in `stages` (if any).
    current_stage_index: Option<usize>,
    stages_completed: u32,
    failure_diagnostic_id: Option<&'static str>,
    op_id: u64,
}

impl UiaSession {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            phase: UiaPhase::Idle,
            flow_kind: None,
            uia_session_id: None,
            stages: Vec::new(),
            current_stage_index: None,
            stages_completed: 0,
            failure_diagnostic_id: None,
            op_id: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn phase(&self) -> UiaPhase {
        self.phase
    }

    pub fn flow_kind(&self) -> Option<UiaFlowKind> {
        self.flow_kind
    }

    pub fn uia_session_id(&self) -> Option<&str> {
        self.uia_session_id.as_deref()
    }

    pub fn stages(&self) -> &[UiaStage] {
        &self.stages
    }

    pub fn current_stage(&self) -> Option<&UiaStage> {
        self.current_stage_index.and_then(|i| self.stages.get(i))
    }

    pub fn stages_completed(&self) -> u32 {
        self.stages_completed
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

    /// Invariant: this module never retains user secrets after submit.
    pub fn never_stores_secrets(&self) -> bool {
        true
    }

    fn validate_id(label: &'static str, value: &str) -> Result<(), AuthError> {
        if value.is_empty() || value.chars().count() > MAX_UIA_ID_CHARS {
            return Err(AuthError::InvalidInput {
                diagnostic_id: label,
                reason: "empty_or_too_long_id",
            });
        }
        Ok(())
    }

    /// Begin UIA after host receives 401 + flows. `uia_session_id` is the
    /// homeserver session string (opaque; not an access token).
    pub fn begin(
        &mut self,
        flow_kind: UiaFlowKind,
        uia_session_id: impl Into<String>,
        stages: Vec<UiaStage>,
    ) -> Result<u64, AuthError> {
        if self.is_active() {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-uia-already-active",
            });
        }
        let uia_session_id = uia_session_id.into().trim().to_owned();
        Self::validate_id("p3.4-invalid-uia-session-id", &uia_session_id)?;
        if stages.is_empty() {
            return Err(AuthError::InvalidInput {
                diagnostic_id: "p3.4-empty-stages",
                reason: "at_least_one_stage_required",
            });
        }
        if stages.len() > MAX_UIA_STAGES {
            return Err(AuthError::InvalidInput {
                diagnostic_id: "p3.4-stage-cap",
                reason: "too_many_stages",
            });
        }
        for s in &stages {
            if s.matrix_type.is_empty() {
                return Err(AuthError::InvalidInput {
                    diagnostic_id: "p3.4-empty-stage-type",
                    reason: "stage_matrix_type_required",
                });
            }
        }

        self.op_id = self.op_id.saturating_add(1);
        self.flow_kind = Some(flow_kind);
        self.uia_session_id = Some(uia_session_id);
        self.stages = stages;
        self.current_stage_index = Some(0);
        self.stages_completed = 0;
        self.failure_diagnostic_id = None;
        self.phase = UiaPhase::ChallengePending;
        Ok(self.op_id)
    }

    /// Host starts submitting the current stage (secrets stay on host/SDK only).
    pub fn begin_submit(&mut self, op_id: u64) -> Result<UiaStageKind, AuthError> {
        if op_id != self.op_id {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-stale-uia-op",
            });
        }
        if self.phase != UiaPhase::ChallengePending {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-submit-wrong-phase",
            });
        }
        let kind = self
            .current_stage()
            .map(|s| s.kind)
            .ok_or(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-no-current-stage",
            })?;
        self.phase = UiaPhase::Submitting;
        Ok(kind)
    }

    /// Stage accepted; advance to next stage or complete.
    pub fn stage_accepted(&mut self, op_id: u64) -> Result<(), AuthError> {
        if op_id != self.op_id {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-stale-uia-op",
            });
        }
        if self.phase != UiaPhase::Submitting {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-accept-wrong-phase",
            });
        }
        self.stages_completed = self.stages_completed.saturating_add(1);
        let next = self.current_stage_index.map(|i| i + 1).unwrap_or(0);
        if next >= self.stages.len() {
            self.phase = UiaPhase::Completed;
            self.current_stage_index = None;
            // Drop opaque session id after completion — no longer needed in harness.
            self.uia_session_id = None;
        } else {
            self.current_stage_index = Some(next);
            self.phase = UiaPhase::ChallengePending;
        }
        Ok(())
    }

    /// Stage rejected (wrong password, bad code, etc.) — stay on same stage.
    pub fn stage_rejected(
        &mut self,
        op_id: u64,
        diagnostic_id: &'static str,
    ) -> Result<(), AuthError> {
        if op_id != self.op_id {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-stale-uia-op",
            });
        }
        if self.phase != UiaPhase::Submitting {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-reject-wrong-phase",
            });
        }
        self.phase = UiaPhase::ChallengePending;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    pub fn complete_success(&mut self, op_id: u64) -> Result<UiaOutcome, AuthError> {
        if op_id != self.op_id {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-stale-uia-op",
            });
        }
        if self.phase != UiaPhase::Completed {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-complete-wrong-phase",
            });
        }
        let flow_kind = self.flow_kind.ok_or(AuthError::SdkInvariant {
            diagnostic_id: "p3.4-missing-flow-kind",
        })?;
        Ok(UiaOutcome {
            session_generation: self.session_generation,
            op_id: self.op_id,
            flow_kind,
            stages_completed: self.stages_completed,
        })
    }

    pub fn cancel(&mut self, op_id: u64) -> Result<(), AuthError> {
        if op_id != self.op_id {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-stale-uia-op",
            });
        }
        if !self.is_active() {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-cancel-inactive",
            });
        }
        self.phase = UiaPhase::Cancelled;
        self.uia_session_id = None;
        self.current_stage_index = None;
        Ok(())
    }

    pub fn fail(&mut self, op_id: u64, diagnostic_id: &'static str) -> Result<(), AuthError> {
        if op_id != self.op_id {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-stale-uia-op",
            });
        }
        if !self.is_active() {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.4-fail-inactive",
            });
        }
        self.phase = UiaPhase::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        self.uia_session_id = None;
        self.current_stage_index = None;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.phase = UiaPhase::Idle;
        self.flow_kind = None;
        self.uia_session_id = None;
        self.stages.clear();
        self.current_stage_index = None;
        self.stages_completed = 0;
        self.failure_diagnostic_id = None;
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.reset();
        // keep op_id monotonic
    }
}
