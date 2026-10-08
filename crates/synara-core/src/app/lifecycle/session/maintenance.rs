//! Periodic session maintenance shared by the desktop watchdog and the iOS
//! sync-status poll: failed-save retry, pending credential cleanup and
//! rejected-session retirement, each with its own backoff or dedup state.

use std::time::Instant;

use super::{AuthenticationRejectionWatch, RetryBackoff};

/// Backoff and dedup state for [`super::SessionLifecycleOwner::maintenance_tick`].
#[derive(Debug, Default)]
pub struct SessionMaintenance {
    pub(super) rejection: AuthenticationRejectionWatch,
    save: RetryBackoff,
    cleanup: RetryBackoff,
}

impl SessionMaintenance {
    pub fn save_retry_due(&self, now: Instant) -> bool {
        self.save.ready(now)
    }

    pub fn record_save_retry(&mut self, now: Instant, saved: bool) {
        self.save.record(now, saved);
    }

    pub fn cleanup_retry_due(&self, now: Instant) -> bool {
        self.cleanup.ready(now)
    }

    pub fn record_cleanup_retry(&mut self, now: Instant, cleaned: bool) {
        self.cleanup.record(now, cleaned);
    }
}

/// What one maintenance tick did. `None` means the step was not due or had
/// nothing to do.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MaintenanceReport {
    pub saved: Option<bool>,
    pub cleaned: Option<bool>,
    pub retired: Option<bool>,
}

/// A sync snapshot that reported a rejected refresh for `generation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RejectedGeneration {
    pub generation: u64,
    /// Whether the shell's Core is present to retire the generation.
    pub core_present: bool,
}
