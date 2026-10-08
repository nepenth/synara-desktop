//! Session generation numbering.

use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic session generations for one process: 1, 2, 3, ...
///
/// Every install (login, register, restore) allocates a new generation, and
/// every generation-fenced operation (rejected-session retirement, deferred
/// owner work) compares against it. Desktop `MatrixAuthState` and iOS
/// `SharedCore` both use this counter.
#[derive(Debug, Default)]
pub struct SessionGenerations(AtomicU64);

impl SessionGenerations {
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Allocate the next generation. The first call returns 1.
    pub fn allocate(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed).saturating_add(1)
    }

    /// The most recently allocated generation, or 0 before the first install.
    pub fn current(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::SessionGenerations;

    #[test]
    fn generations_start_at_one_and_never_repeat() {
        let generations = SessionGenerations::new();
        assert_eq!(generations.current(), 0);
        assert_eq!(generations.allocate(), 1);
        assert_eq!(generations.allocate(), 2);
        assert_eq!(generations.current(), 2);
    }
}
