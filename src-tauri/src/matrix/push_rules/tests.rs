//! Unit tests for P9.2 push-rules index.

use super::*;

fn rule(id: &str, kind: PushRuleKind, actions: Vec<PushAction>) -> PushRule {
    PushRule {
        rule_id: id.into(),
        kind,
        enabled: true,
        default: false,
        pattern: None,
        actions,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_push_rules_markers(), MATRIX_PUSH_RULES_MARKER);
}

#[test]
fn upsert_list_enable() {
    let mut idx = PushRulesIndex::new(1);
    idx.upsert(rule(
        ".m.rule.message",
        PushRuleKind::Underride,
        vec![PushAction::Notify],
    ))
    .unwrap();
    idx.upsert(PushRule {
        rule_id: ".m.rule.contains_user_name".into(),
        kind: PushRuleKind::Content,
        enabled: true,
        default: true,
        pattern: Some("alice".into()),
        actions: vec![PushAction::Notify, PushAction::SetTweakHighlight],
    })
    .unwrap();
    assert_eq!(idx.len(), 2);
    assert_eq!(idx.list_kind(PushRuleKind::Content).len(), 1);
    idx.set_enabled(".m.rule.message", false).unwrap();
    assert!(!idx.get(".m.rule.message").unwrap().enabled);
    assert!(idx.any_notify_enabled());
}

#[test]
fn global_disable_blocks_notify() {
    let mut idx = PushRulesIndex::new(1);
    idx.upsert(rule(
        ".m.rule.message",
        PushRuleKind::Underride,
        vec![PushAction::Notify],
    ))
    .unwrap();
    idx.set_global_enabled(false);
    assert!(!idx.any_notify_enabled());
}

#[test]
fn forbids_tokens_and_retire() {
    let mut idx = PushRulesIndex::new(1);
    let err = idx
        .upsert(rule(
            "access_token=evil",
            PushRuleKind::Override,
            vec![PushAction::DontNotify],
        ))
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p9.2-forbidden-rule-id");
    idx.upsert(rule(
        ".m.rule.room_one_to_one",
        PushRuleKind::Underride,
        vec![PushAction::Notify],
    ))
    .unwrap();
    idx.retire_generation(9);
    assert!(idx.is_empty());
    assert_eq!(idx.session_generation(), 9);
}

#[test]
fn remove_not_found() {
    let mut idx = PushRulesIndex::new(1);
    assert!(!idx.remove("missing"));
    let err = idx.set_enabled("missing", true).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p9.2-rule-not-found");
}
