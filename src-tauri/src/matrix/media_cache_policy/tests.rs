//! Unit tests for P7.6 media-cache retention and privacy policy.

use super::*;

fn entry(handle_id: &str, last_access_secs: u64, encrypted_only: bool) -> EntryMeta {
    EntryMeta {
        handle_id: handle_id.to_owned(),
        last_access_secs,
        encrypted_only,
    }
}

fn policy(max_entries: usize, max_age_secs: Option<u64>, purge_on_logout: bool) -> RetentionPolicy {
    RetentionPolicy {
        max_entries,
        max_age_secs,
        purge_on_logout,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_media_cache_policy_markers(),
        MATRIX_MEDIA_CACHE_POLICY_MARKER
    );
}

#[test]
fn age_limit_expires_at_ttl_and_handles_future_timestamps() {
    let entries = [
        entry("expired", 40, false),
        entry("fresh", 41, false),
        entry("future", 101, false),
    ];

    let plan = plan_purge(&entries, &policy(10, Some(60), false), 100);

    assert_eq!(plan.handle_ids, vec!["expired"]);
}

#[test]
fn entry_cap_purges_lru_with_stable_tie_breaking() {
    let entries = [
        entry("new", 30, false),
        entry("old-b", 10, false),
        entry("old-a", 10, false),
        entry("middle", 20, false),
    ];

    let plan = plan_purge(&entries, &policy(2, None, false), 100);

    assert_eq!(plan.handle_ids, vec!["old-a", "old-b"]);
}

#[test]
fn logout_rule_purges_only_encrypted_media() {
    let entries = [
        entry("public", 10, false),
        entry("private", 20, true),
        entry("private-new", 30, true),
    ];

    let plan = plan_purge(&entries, &policy(10, None, true), 100);

    assert_eq!(plan.handle_ids, vec!["private", "private-new"]);
}

#[test]
fn rules_form_a_unique_union_before_applying_entry_cap() {
    let entries = [
        entry("expired-private", 1, true),
        entry("expired", 2, false),
        entry("old-retained", 80, false),
        entry("new-retained", 90, false),
    ];

    let plan = plan_purge(&entries, &policy(1, Some(50), true), 100);

    assert_eq!(
        plan.handle_ids,
        vec!["expired-private", "expired", "old-retained"]
    );
}

#[test]
fn zero_limits_purge_everything() {
    let entries = [entry("b", 2, false), entry("a", 1, false)];

    let plan = plan_purge(&entries, &policy(0, Some(0), false), 2);

    assert_eq!(plan.handle_ids, vec!["a", "b"]);
}

#[test]
fn duplicate_handles_are_normalized_and_debug_is_opaque() {
    let entries = [
        entry("sensitive-handle", 1, false),
        entry("sensitive-handle", 100, true),
    ];

    let plan = plan_purge(&entries, &policy(10, None, true), 100);

    assert_eq!(plan.handle_ids, vec!["sensitive-handle"]);
    assert!(!format!("{:?}", entries[0]).contains("sensitive-handle"));
    assert!(!format!("{plan:?}").contains("sensitive-handle"));
}
