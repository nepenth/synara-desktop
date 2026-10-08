//! Retry backoff for watchdog work that touches platform secret storage.

use std::time::{Duration, Instant};

/// Exponential backoff: 5s after the first failure, doubling to at most 60s,
/// reset on success. The caller passes the monotonic time so tests and both
/// shells can drive it without a clock dependency.
#[derive(Debug)]
pub struct RetryBackoff {
    delay: Duration,
    next_attempt: Option<Instant>,
}

impl Default for RetryBackoff {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryBackoff {
    const INITIAL: Duration = Duration::from_secs(5);
    const MAX: Duration = Duration::from_secs(60);

    pub fn new() -> Self {
        Self {
            delay: Self::INITIAL,
            next_attempt: None,
        }
    }

    pub fn ready(&self, now: Instant) -> bool {
        self.next_attempt.is_none_or(|at| now >= at)
    }

    pub fn record(&mut self, now: Instant, succeeded: bool) {
        if succeeded {
            *self = Self::new();
        } else {
            self.next_attempt = Some(now + self.delay);
            self.delay = (self.delay * 2).min(Self::MAX);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RetryBackoff;
    use std::time::{Duration, Instant};

    #[test]
    fn keyring_retries_back_off_to_a_minute_and_reset_on_success() {
        let start = Instant::now();
        let mut backoff = RetryBackoff::new();
        assert!(backoff.ready(start), "the first attempt runs immediately");

        let mut now = start;
        for expected in [5, 10, 20, 40, 60, 60] {
            backoff.record(now, false);
            assert!(!backoff.ready(now + Duration::from_secs(expected) - Duration::from_millis(1)));
            now += Duration::from_secs(expected);
            assert!(backoff.ready(now), "retry after {expected}s");
        }

        backoff.record(now, true);
        assert!(backoff.ready(now), "success clears the wait");
        backoff.record(now, false);
        assert!(!backoff.ready(now + Duration::from_secs(4)));
        assert!(
            backoff.ready(now + Duration::from_secs(5)),
            "success resets to 5s"
        );
    }
}
