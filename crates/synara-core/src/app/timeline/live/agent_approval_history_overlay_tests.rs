use super::*;
use crate::app::account_data::SynaraAgentApprovalHistoryDecision;

fn history_item(event_id: &str, decided_at: f64) -> SynaraAgentApprovalHistoryItem {
    SynaraAgentApprovalHistoryItem {
        room_id: "!room:example.org".to_owned(),
        event_id: event_id.to_owned(),
        sender: "@hermes:example.org".to_owned(),
        decision: SynaraAgentApprovalHistoryDecision::ApproveOnce,
        decided_at,
        origin_server_ts: decided_at - 1_000.0,
        expires_at: decided_at + 300_000.0,
        summary: "rm file".to_owned(),
    }
}

#[test]
fn overlay_unions_remote_sync_items_instead_of_masking_them() {
    let now = 1_700_000_000_000.0;
    let local = history_item("$local", now);
    let remote = history_item("$remote", now - 1.0);
    let merged = merge_approval_history_overlay(
        Some(std::slice::from_ref(&local)),
        vec![remote.clone()],
        now,
    );
    assert_eq!(merged.len(), 2);
    assert_eq!(merged[0].event_id, "$local");
    assert_eq!(merged[1].event_id, "$remote");
}

#[test]
fn overlay_keeps_the_newer_decision_when_sync_echoes_the_same_event() {
    let now = 1_700_000_000_000.0;
    let local = SynaraAgentApprovalHistoryItem {
        decision: SynaraAgentApprovalHistoryDecision::Deny,
        summary: "later".to_owned(),
        ..history_item("$same", now)
    };
    let stale = history_item("$same", now - 5_000.0);
    let merged = merge_approval_history_overlay(Some(&[local]), vec![stale], now);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].decision, SynaraAgentApprovalHistoryDecision::Deny);
    assert_eq!(merged[0].summary, "later");
}

#[test]
fn expired_overlay_does_not_hide_the_cached_snapshot() {
    let now = Instant::now();
    let pending = Some((now, vec![history_item("$local", 1_700_000_000_000.0)]));
    assert!(fresh_overlay_items(&pending, now).is_some());
    assert!(fresh_overlay_items(&pending, now + AGENT_APPROVAL_HISTORY_PENDING_TTL).is_none());
}

#[test]
fn unconfirmed_write_survives_overlay_ttl_in_the_snapshot_merge() {
    let now = 1_700_000_000_000.0;
    let unconfirmed = history_item("$local", now);
    let cached = history_item("$remote", now - 1.0);
    let local = merge_approval_history_overlay(None, vec![unconfirmed.clone()], now);
    let visible = merge_approval_history_overlay(Some(&local), vec![cached], now);
    assert!(visible.iter().any(|item| item.event_id == "$local"));
    assert!(visible.iter().any(|item| item.event_id == "$remote"));
}

#[test]
fn in_flight_overlay_is_visible_before_the_homeserver_write_returns() {
    let now = Instant::now();
    let item = history_item("$local", 1_700_000_000_000.0);
    let (_, items) = push_overlay_history_item(None, item.clone(), now);
    assert_eq!(items, vec![item]);
}

#[test]
fn second_local_decision_is_merged_into_the_existing_overlay() {
    let now = Instant::now();
    let first = history_item("$first", 1_700_000_000_000.0);
    let second = history_item("$second", 1_700_000_000_001.0);
    let pending = Some(push_overlay_history_item(None, first.clone(), now));
    let (_, items) = push_overlay_history_item(pending, second.clone(), now);
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| item.event_id == "$first"));
    assert!(items.iter().any(|item| item.event_id == "$second"));
}

#[test]
fn overlay_survives_an_empty_or_pre_write_cache() {
    let now = 1_700_000_000_000.0;
    let local = history_item("$local", now);
    let merged =
        merge_approval_history_overlay(Some(std::slice::from_ref(&local)), Vec::new(), now);
    assert_eq!(merged, vec![local]);
}

#[test]
fn unconfirmed_history_is_keyed_by_room_and_event() {
    let mut pending = HashMap::new();
    let item = history_item("$ev", 1_700_000_000_000.0);
    pending.insert(approval_history_item_key(&item), item.clone());
    assert_eq!(
        pending.remove(&("!room:example.org".to_owned(), "$ev".to_owned())),
        Some(item)
    );
    assert!(pending
        .remove(&("!room:example.org".to_owned(), "$other".to_owned()))
        .is_none());
}

#[test]
fn history_write_is_serialized_on_the_owner_mutex_not_the_timeline_registry() {
    let source = include_str!("../live.rs");
    let record = source
        .split("    async fn record_agent_approval_history(")
        .nth(1)
        .and_then(|rest| rest.split("    pub async fn lock(").next())
        .expect("record_agent_approval_history");
    assert!(record.contains("self.approval_history_mutation.lock().await"));
    assert!(record.contains("append_agent_approval_history_item_live"));
    assert!(
        !record.contains("self.registry.lock()"),
        "history RMW must not hold the timeline registry across the homeserver await"
    );
    assert!(
        !record.contains("room_notes_mutation"),
        "history RMW must not share the image-pack notes mutex"
    );
}

#[test]
fn reaction_success_does_not_fail_the_decision_when_history_write_fails() {
    let source = include_str!("../live.rs");
    let decide = source
        .split("    pub async fn decide_agent_approval(")
        .nth(1)
        .and_then(|rest| rest.split("    pub async fn redact_reaction(").next())
        .expect("decide_agent_approval");
    assert!(decide.contains("if let Err(diagnostic) = self.record_agent_approval_history"));
    assert!(decide.contains("eprintln!(\"agent-approval-history-write-failed: {diagnostic}\")"));
    assert!(
        !decide.contains("eprintln!(\"agent-approval-history-write-failed: {diagnostic},"),
        "history-write failure logs must not interpolate room or event identifiers"
    );
    let after_write = decide
        .split("agent-approval-history-write-failed: {diagnostic}")
        .nth(1)
        .expect("decision continues after the history-write log");
    assert!(after_write.contains("Ok(NativeAgentApprovalDecisionResult"));
    assert!(!after_write.contains("return Err("));
}

#[test]
fn already_decided_retries_unconfirmed_history_after_dropping_decision_locks() {
    let source = include_str!("../live.rs");
    let decide = source
        .split("    pub async fn decide_agent_approval(")
        .nth(1)
        .and_then(|rest| rest.split("    pub async fn redact_reaction(").next())
        .expect("decide_agent_approval");
    assert!(decide.contains("retry_unconfirmed_approval_history"));
    let completed = decide
        .split("if decisions.is_completed(&decision_key)")
        .nth(1)
        .and_then(|rest| rest.split("let Some(decision_lock) = decision_lock").next())
        .expect("completed-memory gate");
    assert!(
        completed.contains("None"),
        "completed memory must leave the std mutex before the history retry await"
    );
    assert!(
        !completed.contains("return Ok("),
        "returning AlreadyDecided inside the approval_decisions lock would hold a std mutex across the history RMW"
    );
    assert!(decide.contains("drop(decision_guard)"));
}

#[test]
fn failed_history_write_is_queued_for_already_decided_retry() {
    let source = include_str!("../live.rs");
    let record = source
        .split("    async fn record_agent_approval_history(")
        .nth(1)
        .and_then(|rest| rest.split("    pub async fn lock(").next())
        .expect("record_agent_approval_history");
    assert!(record.contains(".insert(approval_history_item_key(&item), item)"));
    assert!(record.contains(".remove(&approval_history_item_key(&item))"));
    let retry = source
        .split("    async fn retry_unconfirmed_approval_history(")
        .nth(1)
        .and_then(|rest| rest.split("    pub async fn lock(").next())
        .expect("retry_unconfirmed_approval_history");
    assert!(
        !retry.contains("self.approval_decisions"),
        "history backfill must not take the decision registry"
    );
    assert!(
        !retry.contains("self.registry.lock()"),
        "history backfill must not take the timeline registry"
    );
}

#[test]
fn history_snapshot_merges_overlay_with_the_sdk_cache() {
    let source = include_str!("../live.rs");
    let snapshot = source
        .split("    pub async fn agent_approval_history_snapshot(")
        .nth(1)
        .and_then(|rest| {
            rest.split("    async fn record_agent_approval_history(")
                .next()
        })
        .expect("agent_approval_history_snapshot");
    assert!(snapshot.contains("merge_approval_history_overlay"));
    assert!(snapshot.contains("snapshot_agent_approval_history"));
    assert!(
        snapshot.contains("lock_approval_history_unconfirmed"),
        "failed writes must remain visible after the 30s overlay TTL"
    );
    assert!(snapshot.contains("if local.is_empty()"));
}
