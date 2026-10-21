//! The typed half of a SharedCore FFI error: what the caller can do next.
//!
//! Every domain error used to be a single `Failed { code, description }`, so
//! Swift parsed codes to decide whether to wait, retry or give up. Converted
//! errors carry a [`CoreErrorKind`] first; the code stays as a stable detail
//! for diagnostics and for the rare caller that needs one exact condition.
//!
//! Converted: session restore, sign-in, owner attach, sync start/stop,
//! session status, leftover (logout, wipe, forget) and send-text errors.
//! To convert another domain error:
//! 1. add `kind: CoreErrorKind` as the first field of its `Failed` variant;
//! 2. build it through one constructor that takes the kind from
//!    [`CoreErrorKind::for_code`] for this crate's own codes, or from
//!    [`CoreErrorKind::from_category`] when it wraps a Core command failure;
//! 3. implement [`TypedFfiError`] for it, and in Swift add it to the
//!    `SharedCoreTypedError` conformances and switch on `coreErrorKind`.

use super::*;

/// What a caller can do about a failed SharedCore call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CoreErrorKind {
    /// Retry the same call later, unchanged: the stores are closed for
    /// background, the Keychain is locked, or the network or homeserver
    /// failed.
    Retryable,
    /// No signed-in session, retained client or attached owners yet. Retry
    /// once restore or sign-in has finished.
    NotReady,
    /// The user has to act first: sign in again, check the password, or
    /// reset this device's local data.
    ActionRequired,
    /// Retrying unchanged fails the same way: invalid input, an unsupported
    /// server, a mismatched account, or an internal invariant.
    Terminal,
}

impl CoreErrorKind {
    /// The kind of a Core command failure, from its closed category.
    pub(crate) fn from_category(category: MatrixIpcErrorCategory) -> Self {
        use MatrixIpcErrorCategory as Category;
        match category {
            Category::RateLimited
            | Category::Connectivity
            | Category::HomeserverUnavailable
            | Category::StoreLocked
            | Category::Cancellation
            | Category::StaleSessionGeneration => Self::Retryable,
            Category::AuthenticationRejected
            | Category::UserDeactivated
            | Category::InteractiveAuthRequired
            | Category::StoreCorrupt
            | Category::RecoveryFailure
            | Category::VerificationFailure => Self::ActionRequired,
            _ => Self::Terminal,
        }
    }

    /// The kind of one of this crate's lifecycle and send codes. A code
    /// without an entry is terminal: nothing says retrying would help.
    pub(crate) fn for_code(code: &str) -> Self {
        match code {
            SESSION_SUSPENDED_CODE
            | VAULT_UNAVAILABLE_CODE
            | LOGIN_VAULT_UNAVAILABLE_CODE
            | RESTORE_FAILED_CODE
            | ATTACH_FAILED_CODE
            | SYNC_START_FAILED_CODE
            | CLIENT_RESUME_FAILED_CODE
            | SYNC_STOP_FAILED_CODE
            | CLIENT_PAUSE_FAILED_CODE
            | SESSION_STATUS_FAILED_CODE
            | LEFTOVER_FAILED_CODE => Self::Retryable,
            ATTACH_SESSION_MISSING_CODE
            | SYNC_NOT_ATTACHED_CODE
            | LEFTOVER_NO_SESSION_CODE
            | SEND_TEXT_NO_SESSION_CODE
            | SEND_ROOM_NOT_FOUND_CODE => Self::NotReady,
            MATERIAL_MISSING_CODE
            | STORE_RESET_REQUIRED_CODE
            | LOGIN_STORE_RESET_REQUIRED_CODE
            | LOGIN_FAILED_CODE => Self::ActionRequired,
            _ => Self::Terminal,
        }
    }
}

/// The room is not known yet, typically while a restored session's first
/// sync is still loading rooms.
pub(super) const SEND_ROOM_NOT_FOUND_CODE: &str = "d0.4-send-room-not-found";

/// A converted FFI error: its kind first, its stable code second.
pub trait TypedFfiError {
    fn kind(&self) -> CoreErrorKind;
    fn code(&self) -> &str;
}

macro_rules! typed_ffi_error {
    ($($error:ident),+ $(,)?) => {$(
        impl $error {
            /// The same error with the kind of the Core failure it wraps.
            /// Errors that never wrap a Core command failure do not use it.
            #[allow(dead_code)]
            pub(crate) fn with_kind(self, kind: CoreErrorKind) -> Self {
                match self {
                    Self::Failed {
                        code, description, ..
                    } => Self::Failed {
                        kind,
                        code,
                        description,
                    },
                }
            }
        }

        impl TypedFfiError for $error {
            fn kind(&self) -> CoreErrorKind {
                match self {
                    Self::Failed { kind, .. } => *kind,
                }
            }

            fn code(&self) -> &str {
                match self {
                    Self::Failed { code, .. } => code,
                }
            }
        }

        impl std::fmt::Display for $error {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Failed { description, .. } => formatter.write_str(description),
                }
            }
        }

        impl std::error::Error for $error {}
    )+};
}

typed_ffi_error!(
    SessionRestoreError,
    SessionLoginError,
    SessionAttachError,
    SyncStartError,
    SyncStopError,
    SessionStatusError,
    LeftoverCommandError,
    SendTextError,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_and_send_codes_have_the_kind_swift_acts_on() {
        for (code, kind) in [
            (SESSION_SUSPENDED_CODE, CoreErrorKind::Retryable),
            (VAULT_UNAVAILABLE_CODE, CoreErrorKind::Retryable),
            (SYNC_NOT_ATTACHED_CODE, CoreErrorKind::NotReady),
            (SEND_TEXT_NO_SESSION_CODE, CoreErrorKind::NotReady),
            (SEND_ROOM_NOT_FOUND_CODE, CoreErrorKind::NotReady),
            (STORE_RESET_REQUIRED_CODE, CoreErrorKind::ActionRequired),
            (
                LOGIN_STORE_RESET_REQUIRED_CODE,
                CoreErrorKind::ActionRequired,
            ),
            (MATERIAL_MISSING_CODE, CoreErrorKind::ActionRequired),
            (LOGIN_SLIDING_SYNC_UNSUPPORTED_CODE, CoreErrorKind::Terminal),
            (RESTORED_IDENTITY_MISMATCH_CODE, CoreErrorKind::Terminal),
            (ALREADY_RESTORED_CODE, CoreErrorKind::Terminal),
            // An oversized message fails the same way every time.
            (SEND_TEXT_FAILED_CODE, CoreErrorKind::Terminal),
            ("not-a-known-code", CoreErrorKind::Terminal),
        ] {
            assert_eq!(CoreErrorKind::for_code(code), kind, "{code}");
        }
    }

    #[test]
    fn constructors_type_the_error_and_keep_the_code_as_detail() {
        let suspended = send_text_failed(SESSION_SUSPENDED_CODE, SESSION_SUSPENDED_DESCRIPTION);
        assert_eq!(suspended.kind(), CoreErrorKind::Retryable);
        assert_eq!(suspended.code(), SESSION_SUSPENDED_CODE);
        assert_eq!(suspended.to_string(), SESSION_SUSPENDED_DESCRIPTION);

        let reset = login_store_key_error(store_reset_required());
        assert_eq!(reset.kind(), CoreErrorKind::ActionRequired);
        assert_eq!(reset.code(), LOGIN_STORE_RESET_REQUIRED_CODE);

        let not_attached = sync_stop_failed(SYNC_NOT_ATTACHED_CODE, SYNC_NOT_ATTACHED_DESCRIPTION);
        assert_eq!(not_attached.kind(), CoreErrorKind::NotReady);
    }

    #[test]
    fn wrapped_core_failures_take_the_kind_of_their_category() {
        let offline = MatrixIpcError::new(MatrixIpcErrorCategory::Connectivity)
            .with_diagnostic("p2-send-text-network");
        let error = map_send_text_core_error(SEND_TEXT_NO_SESSION_CODE, offline);
        assert_eq!(error.kind(), CoreErrorKind::Retryable);
        assert_eq!(error.code(), "p2-send-text-network");

        let no_session = MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
            .with_diagnostic(SEND_TEXT_NO_SESSION_CODE);
        assert_eq!(
            map_send_text_core_error(SEND_TEXT_NO_SESSION_CODE, no_session).kind(),
            CoreErrorKind::NotReady
        );

        let room = MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
            .with_diagnostic(SEND_ROOM_NOT_FOUND_CODE);
        assert_eq!(
            map_send_text_core_error(SEND_TEXT_NO_SESSION_CODE, room).kind(),
            CoreErrorKind::NotReady
        );

        let rejected = MatrixIpcError::new(MatrixIpcErrorCategory::AuthenticationRejected)
            .with_diagnostic("p2-session-snapshot-rejected");
        let status = map_session_status_core_error(rejected);
        assert_eq!(status.kind(), CoreErrorKind::ActionRequired);
    }
}
