//! One-use local store recovery capability.
//!
//! A failed normal login whose diagnostic proves the encrypted store needs a
//! reset arms recovery for that account. The shell then asks for an opaque,
//! CSPRNG-backed confirmation id and must present it together with a fixed
//! typed acknowledgement. Neither the account nor any key ever crosses the
//! shell boundary; only the opaque id does.

use super::{SessionFault, SessionFaultKind};
use crate::app::store::AccountIdentity;

const CONFIRMATION_ID_BYTES: usize = 32;

/// Exact acknowledgement the host requires in addition to the confirmation id.
/// Validated only natively; renderer button state is not an authorization.
pub const STORE_RECOVERY_TYPED_CONFIRMATION_TEXT: &str = "ARCHIVE";

const NOT_PENDING: SessionFault = SessionFault::new(
    SessionFaultKind::InvalidRequest,
    "Local Matrix store recovery must be requested from a failed login.",
    "p3.2-login-store-recovery-not-pending",
);

const CONFIRMATION_UNAVAILABLE: SessionFault = SessionFault::new(
    SessionFaultKind::Unavailable,
    "Local Matrix store recovery confirmation is unavailable.",
    "p3.2-login-store-recovery-confirmation-unavailable",
);

/// Wrong, expired or replayed confirmation.
pub const STORE_RECOVERY_CONFIRMATION_REQUIRED: SessionFault = SessionFault::new(
    SessionFaultKind::InvalidRequest,
    "Local Matrix store recovery confirmation is invalid or has expired.",
    "p3.2-login-store-recovery-confirmation-required",
);

/// The only normal-login failures that may arm the explicit archive action.
/// Other local failures (locked keychain, generic store open, I/O) stay
/// fail-closed and retry/support-only rather than being treated as corruption.
pub fn is_recoverable_store_login_diagnostic(diagnostic_id: &str) -> bool {
    matches!(
        diagnostic_id,
        "p3.2-login-store-reset-required" | "p3.2-login-store-migration-required"
    )
}

/// Process-local recovery target. Armed only by a failed native login.
#[derive(Debug, Default)]
pub enum StoreRecoveryState {
    #[default]
    Idle,
    Pending {
        identity: AccountIdentity,
    },
    AwaitingConfirmation {
        identity: AccountIdentity,
        confirmation_id: String,
    },
}

impl StoreRecoveryState {
    /// A normal login or restore supersedes any abandoned recovery affordance.
    pub fn clear(&mut self) {
        *self = Self::Idle;
    }

    /// Remember an account only after an allowlisted failed store-open path.
    pub fn arm(&mut self, identity: AccountIdentity) {
        *self = Self::Pending { identity };
    }

    /// Issue the opaque one-use confirmation id for a pending recovery.
    pub fn prepare_confirmation(&mut self) -> Result<String, SessionFault> {
        let identity = match std::mem::take(self) {
            Self::Pending { identity } => identity,
            Self::Idle | Self::AwaitingConfirmation { .. } => return Err(NOT_PENDING),
        };
        let confirmation_id = new_confirmation_id()?;
        *self = Self::AwaitingConfirmation {
            identity,
            confirmation_id: confirmation_id.clone(),
        };
        Ok(confirmation_id)
    }

    /// Consume the capability before filesystem work so it cannot be replayed.
    /// Wrong input leaves a pending capability untouched, so a transport or UI
    /// error can be corrected without re-arming from a new login failure.
    pub fn take_confirmed(
        &mut self,
        confirmation_id: &str,
        confirmation_text: &str,
    ) -> Result<AccountIdentity, SessionFault> {
        if confirmation_text != STORE_RECOVERY_TYPED_CONFIRMATION_TEXT
            || !is_confirmation_id(confirmation_id)
        {
            return Err(STORE_RECOVERY_CONFIRMATION_REQUIRED);
        }
        let valid = matches!(
            self,
            Self::AwaitingConfirmation { confirmation_id: expected, .. }
                if expected == confirmation_id
        );
        if !valid {
            return Err(STORE_RECOVERY_CONFIRMATION_REQUIRED);
        }
        match std::mem::take(self) {
            Self::AwaitingConfirmation { identity, .. } => Ok(identity),
            Self::Idle | Self::Pending { .. } => Err(STORE_RECOVERY_CONFIRMATION_REQUIRED),
        }
    }
}

/// Opaque CSPRNG capability. Neither a Matrix credential nor a store key, and
/// never logged.
fn new_confirmation_id() -> Result<String, SessionFault> {
    let mut bytes = [0_u8; CONFIRMATION_ID_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| CONFIRMATION_UNAVAILABLE)?;
    let mut id = String::with_capacity(CONFIRMATION_ID_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(id, "{byte:02x}");
    }
    Ok(id)
}

fn is_confirmation_id(value: &str) -> bool {
    value.len() == CONFIRMATION_ID_BYTES * 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    #[test]
    fn confirmation_is_one_use_and_needs_the_typed_text() {
        let mut state = StoreRecoveryState::default();
        assert_eq!(state.prepare_confirmation(), Err(NOT_PENDING));
        state.arm(identity());
        let id = state.prepare_confirmation().unwrap();
        assert!(is_confirmation_id(&id));
        assert_eq!(
            state.take_confirmed(&id, "archive"),
            Err(STORE_RECOVERY_CONFIRMATION_REQUIRED)
        );
        assert_eq!(state.take_confirmed(&id, "ARCHIVE").unwrap(), identity());
        assert_eq!(
            state.take_confirmed(&id, "ARCHIVE"),
            Err(STORE_RECOVERY_CONFIRMATION_REQUIRED)
        );
    }

    #[test]
    fn clear_revokes_pending_and_awaiting_capabilities() {
        let mut state = StoreRecoveryState::default();
        state.arm(identity());
        state.clear();
        assert_eq!(state.prepare_confirmation(), Err(NOT_PENDING));
        state.arm(identity());
        let id = state.prepare_confirmation().unwrap();
        state.clear();
        assert_eq!(
            state.take_confirmed(&id, STORE_RECOVERY_TYPED_CONFIRMATION_TEXT),
            Err(STORE_RECOVERY_CONFIRMATION_REQUIRED)
        );
    }

    #[test]
    fn only_reset_and_migration_diagnostics_arm_recovery() {
        assert!(is_recoverable_store_login_diagnostic(
            "p3.2-login-store-reset-required"
        ));
        assert!(is_recoverable_store_login_diagnostic(
            "p3.2-login-store-migration-required"
        ));
        assert!(!is_recoverable_store_login_diagnostic(
            "p3.2-login-store-locked"
        ));
    }
}
