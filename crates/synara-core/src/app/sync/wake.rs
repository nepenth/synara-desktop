//! Detect OS suspend/resume from wall-clock vs monotonic skew.
//!
//! After desktop sleep the SDK can keep reporting `Running` on a dead
//! long-poll. Webview `visibilitychange` often never fires on Linux because the
//! window stayed "visible". Comparing `SystemTime` (includes suspend) with
//! `Instant` (CLOCK_MONOTONIC, typically does not) is the portable signal.

use std::time::{Duration, Instant, SystemTime};

/// Wall-clock advance beyond monotonic time that counts as a suspend.
pub const SUSPEND_WALL_SKEW: Duration = Duration::from_secs(15);

/// Debounce duplicate recover IPC on the same wake. Wall time is required:
/// `Instant` (CLOCK_MONOTONIC) typically does not advance during OS sleep, so
/// an 8s monotonic cooldown can still look "hot" after hours of suspend and
/// skip the watchdog's one-shot resume.
pub const RECOVER_COOLDOWN: Duration = Duration::from_secs(8);

/// True when a previous successful recover should suppress another restart.
/// A backward NTP step returns false so recovery is not stuck.
pub fn recover_cooldown_active(
    last_success_wall: Option<SystemTime>,
    now_wall: SystemTime,
    cooldown: Duration,
) -> bool {
    let Some(last) = last_success_wall else {
        return false;
    };
    match now_wall.duration_since(last) {
        Ok(elapsed) => elapsed < cooldown,
        Err(_) => false,
    }
}

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

    #[test]
    fn hour_sleep_is_outside_recover_cooldown() {
        let last = UNIX_EPOCH + Duration::from_secs(1_000);
        let now = last + Duration::from_secs(3_600);
        assert!(!recover_cooldown_active(Some(last), now, RECOVER_COOLDOWN));
    }

    #[test]
    fn same_wake_is_inside_recover_cooldown() {
        let last = UNIX_EPOCH + Duration::from_secs(1_000);
        let now = last + Duration::from_secs(2);
        assert!(recover_cooldown_active(Some(last), now, RECOVER_COOLDOWN));
    }

    #[test]
    fn recover_cooldown_inactive_before_first_success() {
        assert!(!recover_cooldown_active(
            None,
            UNIX_EPOCH + Duration::from_secs(1_000),
            RECOVER_COOLDOWN
        ));
    }

    #[test]
    fn recover_cooldown_inactive_when_clock_steps_backward() {
        let last = UNIX_EPOCH + Duration::from_secs(1_000);
        let now = UNIX_EPOCH + Duration::from_secs(900);
        assert!(!recover_cooldown_active(Some(last), now, RECOVER_COOLDOWN));
    }
}
