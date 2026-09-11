//! Detect OS suspend/resume from wall-clock vs monotonic skew.
//!
//! After desktop sleep the SDK can keep reporting `Running` on a dead
//! long-poll. Webview `visibilitychange` often never fires on Linux because the
//! window stayed "visible". Comparing `SystemTime` (includes suspend) with
//! `Instant` (CLOCK_MONOTONIC, typically does not) is the portable signal.

use std::time::{Duration, Instant, SystemTime};

/// Wall-clock advance beyond monotonic time that counts as a suspend.
pub const SUSPEND_WALL_SKEW: Duration = Duration::from_secs(15);

/// True when wall time jumped forward relative to monotonic time by at least
/// `min_skew`. A backward NTP step returns false.
pub fn suspend_detected(
    previous_wall: SystemTime,
    previous_mono: Instant,
    now_wall: SystemTime,
    now_mono: Instant,
    min_skew: Duration,
) -> bool {
    let Ok(wall) = now_wall.duration_since(previous_wall) else {
        return false;
    };
    let mono = now_mono.saturating_duration_since(previous_mono);
    wall.saturating_sub(mono) >= min_skew
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn hour_sleep_is_detected() {
        let wall0 = UNIX_EPOCH + Duration::from_secs(1_000);
        let mono0 = Instant::now();
        let wall1 = wall0 + Duration::from_secs(3_600);
        let mono1 = mono0 + Duration::from_secs(5);
        assert!(suspend_detected(
            wall0,
            mono0,
            wall1,
            mono1,
            SUSPEND_WALL_SKEW
        ));
    }

    #[test]
    fn ordinary_five_second_poll_is_not_a_suspend() {
        let wall0 = UNIX_EPOCH + Duration::from_secs(1_000);
        let mono0 = Instant::now();
        let wall1 = wall0 + Duration::from_secs(5);
        let mono1 = mono0 + Duration::from_secs(5);
        assert!(!suspend_detected(
            wall0,
            mono0,
            wall1,
            mono1,
            SUSPEND_WALL_SKEW
        ));
    }

    #[test]
    fn backward_clock_step_is_not_a_suspend() {
        let wall0 = UNIX_EPOCH + Duration::from_secs(1_000);
        let mono0 = Instant::now();
        let wall1 = UNIX_EPOCH + Duration::from_secs(900);
        let mono1 = mono0 + Duration::from_secs(5);
        assert!(!suspend_detected(
            wall0,
            mono0,
            wall1,
            mono1,
            SUSPEND_WALL_SKEW
        ));
    }
}
