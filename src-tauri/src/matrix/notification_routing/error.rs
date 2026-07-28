//! Privacy-safe errors for notification route resolution (P9.4).

use crate::matrix::ipc::MatrixIpcErrorCategory;

/// Notification routing failure.
///
/// The error deliberately stores only a static diagnostic id. Invalid input
/// values are never retained or rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationRoutingError {
    diagnostic_id: &'static str,
}

impl NotificationRoutingError {
    pub(super) const fn invalid(diagnostic_id: &'static str) -> Self {
        Self { diagnostic_id }
    }

    pub fn diagnostic_id(&self) -> &'static str {
        self.diagnostic_id
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        MatrixIpcErrorCategory::SdkInvariant
    }
}

impl std::fmt::Display for NotificationRoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid notification routing input ({})",
            self.diagnostic_id
        )
    }
}

impl std::error::Error for NotificationRoutingError {}
