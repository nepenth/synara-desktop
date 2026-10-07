//! Closed, static session lifecycle failures.
//!
//! Every field is a fixed string so a fault can cross into a shell error, a
//! log line or an IPC payload without carrying tokens, ids or SDK text.

/// Broad failure class. Shells map it onto their own error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionFaultKind {
    /// The request or its stored state is malformed.
    InvalidRequest,
    /// The session or its credentials are not usable.
    Forbidden,
    /// Storage or the platform could not complete the operation.
    Unavailable,
}

/// A static lifecycle failure: kind, fixed message and closed diagnostic id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionFault {
    pub kind: SessionFaultKind,
    pub message: &'static str,
    pub diagnostic_id: &'static str,
}

impl SessionFault {
    pub const fn new(
        kind: SessionFaultKind,
        message: &'static str,
        diagnostic_id: &'static str,
    ) -> Self {
        Self {
            kind,
            message,
            diagnostic_id,
        }
    }

    /// Session storage failed. Message matches the desktop's existing copy.
    pub const fn unavailable(diagnostic_id: &'static str) -> Self {
        Self::new(
            SessionFaultKind::Unavailable,
            "Native Matrix session storage is unavailable.",
            diagnostic_id,
        )
    }
}

/// Errors that expose a closed diagnostic id. Policy helpers use it to decide
/// whether an id is safe to log without knowing the shell's error type.
pub trait HasDiagnosticId {
    fn diagnostic_id(&self) -> &str;
}

impl HasDiagnosticId for SessionFault {
    fn diagnostic_id(&self) -> &str {
        self.diagnostic_id
    }
}
