//! Per-room budget for urgent (Time-Sensitive/Critical, actionable) approval
//! alerts. A prompt over budget is still delivered, as an ordinary message
//! under push rules, so a flood cannot keep raising Focus-bypassing alerts.

use std::collections::{HashMap, VecDeque};

/// Rolling window and the urgent alerts each room may raise inside it.
pub const APPROVAL_ALERT_WINDOW_MS: u64 = 60_000;
pub const APPROVAL_ALERTS_PER_ROOM_PER_WINDOW: usize = 3;
/// Rooms tracked at once; the least recently alerted room is forgotten first.
const MAX_TRACKED_ROOMS: usize = 256;

#[derive(Debug, Default)]
pub struct ApprovalAlertLimiter {
    rooms: HashMap<String, VecDeque<u64>>,
}

impl ApprovalAlertLimiter {
    /// Reserve one urgent alert for `room_id` at `now_ms`. Returns false when
    /// the room already used its budget; the caller then downgrades the alert.
    pub fn try_reserve(&mut self, room_id: &str, now_ms: u64) -> bool {
        let cutoff = now_ms.saturating_sub(APPROVAL_ALERT_WINDOW_MS);
        self.rooms.retain(|_, sent| {
            while sent.front().is_some_and(|at| *at <= cutoff) {
                sent.pop_front();
            }
            !sent.is_empty()
        });
        if let Some(sent) = self.rooms.get_mut(room_id) {
            if sent.len() >= APPROVAL_ALERTS_PER_ROOM_PER_WINDOW {
                return false;
            }
            sent.push_back(now_ms);
            return true;
        }
        if self.rooms.len() >= MAX_TRACKED_ROOMS {
            if let Some(oldest) = self
                .rooms
                .iter()
                .min_by_key(|(_, sent)| sent.back().copied().unwrap_or_default())
                .map(|(id, _)| id.clone())
            {
                self.rooms.remove(&oldest);
            }
        }
        self.rooms
            .insert(room_id.to_owned(), VecDeque::from([now_ms]));
        true
    }

    /// Return a reservation whose alert was not shown (duplicate, focus, or
    /// an error), so retries of one prompt do not spend the room's budget.
    pub fn release(&mut self, room_id: &str, reserved_at_ms: u64) {
        if let Some(sent) = self.rooms.get_mut(room_id) {
            if let Some(index) = sent.iter().rposition(|at| *at == reserved_at_ms) {
                sent.remove(index);
            }
            if sent.is_empty() {
                self.rooms.remove(room_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_700_000_000_000;

    #[test]
    fn each_room_gets_a_rolling_budget() {
        let mut limiter = ApprovalAlertLimiter::default();
        for slot in 0..APPROVAL_ALERTS_PER_ROOM_PER_WINDOW as u64 {
            assert!(limiter.try_reserve("!a:example.org", NOW + slot * 1_000));
        }
        assert!(!limiter.try_reserve("!a:example.org", NOW + 5_000));
        // Another room is unaffected.
        assert!(limiter.try_reserve("!b:example.org", NOW + 5_000));
        // The oldest alert leaves the window and frees exactly one slot.
        assert!(limiter.try_reserve("!a:example.org", NOW + APPROVAL_ALERT_WINDOW_MS));
        assert!(!limiter.try_reserve("!a:example.org", NOW + APPROVAL_ALERT_WINDOW_MS));
    }

    #[test]
    fn released_reservations_do_not_spend_the_budget() {
        let mut limiter = ApprovalAlertLimiter::default();
        for _ in 0..10 {
            assert!(limiter.try_reserve("!a:example.org", NOW));
            limiter.release("!a:example.org", NOW);
        }
        assert!(limiter.rooms.is_empty());
    }

    #[test]
    fn tracked_rooms_stay_bounded() {
        let mut limiter = ApprovalAlertLimiter::default();
        for room in 0..(MAX_TRACKED_ROOMS as u64 + 10) {
            assert!(limiter.try_reserve(&format!("!{room}:example.org"), NOW + room));
        }
        assert!(limiter.rooms.len() <= MAX_TRACKED_ROOMS);
    }
}
