//! Unit tests for P9.3 focus suppression and badge semantics.

use super::*;

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_notification_policy_markers(),
        MATRIX_NOTIFICATION_POLICY_MARKER
    );
}

#[test]
fn focused_same_room_is_suppressed() {
    let policy =
        SuppressionPolicy::new(FocusState::Focused, Some("!focused:example.org".to_owned()));

    assert!(policy.should_suppress("!focused:example.org"));
    assert!(!policy.should_suppress("!other:example.org"));
}

#[test]
fn background_allows_selected_room() {
    let policy = SuppressionPolicy::new(
        FocusState::Background,
        Some("!selected:example.org".to_owned()),
    );

    assert!(!policy.should_suppress("!selected:example.org"));
}

#[test]
fn focused_without_room_allows_candidate() {
    let mut policy = SuppressionPolicy::default();
    policy.update(FocusState::Focused, None);

    assert_eq!(policy.focus_state(), FocusState::Focused);
    assert!(policy.focused_room().is_none());
    assert!(!policy.should_suppress("!room:example.org"));
}

#[test]
fn badge_increment_decrement_and_clear_room() {
    let mut badges = BadgeCounter::new(7);

    assert_eq!(badges.increment_by("!noop:example.org", 0), 0);
    assert!(badges.is_empty());
    assert_eq!(badges.increment("!a:example.org"), 1);
    assert_eq!(badges.increment_by("!a:example.org", 2), 3);
    assert_eq!(badges.increment("!b:example.org"), 1);
    assert_eq!(badges.total(), 4);
    assert_eq!(badges.room_count("!a:example.org"), 3);
    assert_eq!(badges.tracked_rooms(), 2);

    assert_eq!(badges.decrement("!a:example.org"), 2);
    assert_eq!(badges.decrement_by("!a:example.org", 8), 0);
    assert_eq!(badges.room_count("!a:example.org"), 0);
    assert_eq!(badges.clear_room("!b:example.org"), 1);
    assert_eq!(badges.total(), 0);
    assert!(badges.is_empty());
}

#[test]
fn badge_counts_saturate_at_u32_max() {
    let mut badges = BadgeCounter::new(1);

    assert_eq!(badges.increment_by("!a:example.org", u32::MAX), u32::MAX);
    assert_eq!(badges.increment("!a:example.org"), u32::MAX);
    assert_eq!(badges.increment("!b:example.org"), 1);
    assert_eq!(badges.total(), u32::MAX);

    assert_eq!(badges.decrement("!a:example.org"), u32::MAX - 1);
    assert_eq!(badges.total(), u32::MAX);
    assert_eq!(badges.clear_room("!b:example.org"), 1);
    assert_eq!(badges.total(), u32::MAX - 1);
}

#[test]
fn clear_and_retire_generation_reset_counts() {
    let mut badges = BadgeCounter::new(3);
    badges.increment_by("!room:example.org", 4);
    badges.clear();
    assert_eq!(badges.session_generation(), 3);
    assert_eq!(badges.total(), 0);

    badges.increment("!room:example.org");
    badges.retire_generation(4);
    assert_eq!(badges.session_generation(), 4);
    assert_eq!(badges.total(), 0);
    assert!(badges.is_empty());
}
