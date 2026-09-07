//! Account-level replacement suppression, established before APNs registration.
//!
//! The Matrix default suppress-edits rule follows mention overrides. Synara's
//! product policy is stricter: edits must not alert even when they mention the
//! user. Own one custom override and preserve every unrelated rule.

use matrix_sdk::ruma::{
    api::client::push::{delete_pushrule, get_pushrules_all, set_pushrule, set_pushrule_enabled},
    push::{
        EventPropertyIsConditionData, NewConditionalPushRule, NewPushRule, PushCondition, RuleKind,
        Ruleset,
    },
};
use matrix_sdk::Client;

const RULE_ID: &str = "com.whylandcreative.synara.suppress_edits";
const POLICY_FAILED: &str = "v-pusher.edit-policy-failed";
const POLICY_UNCONFIRMED: &str = "v-pusher.edit-policy-unconfirmed";

fn conditions() -> Vec<PushCondition> {
    // Match the exact relation type, using the same condition as the Matrix
    // default suppress-edits rule. Escaped dots address the literal JSON key.
    vec![PushCondition::EventPropertyIs(
        EventPropertyIsConditionData::new(
            r"content.m\.relates_to.rel_type".to_owned(),
            "m.replace".into(),
        ),
    )]
}

fn precedes_notifying_overrides(rules: &Ruleset) -> bool {
    for rule in &rules.override_ {
        if rule.rule_id == RULE_ID {
            return true;
        }
        if rule.enabled && rule.actions.iter().any(|action| action.should_notify()) {
            return false;
        }
    }
    false
}

fn policy_is_confirmed(rules: &Ruleset) -> bool {
    rules.override_.get(RULE_ID).is_some_and(|rule| {
        !rule.default
            && rule.enabled
            && rule.actions.is_empty()
            && matches!(rule.conditions.as_slice(),
                [PushCondition::EventPropertyIs(condition)]
                    if condition.key == r"content.m\.relates_to.rel_type"
                        && condition.value.as_str() == Some("m.replace")
            )
    }) && precedes_notifying_overrides(rules)
}

/// Authenticated, authoritative readback: NotificationSettings/account cache can
/// be stale during startup and must not authorize pusher registration here.
async fn read_rules(client: &Client) -> Result<Ruleset, &'static str> {
    client
        .send(get_pushrules_all::v3::Request::new())
        .await
        .map(|response| response.global)
        .map_err(|_| POLICY_FAILED)
}

pub(super) async fn ensure_edit_notification_policy(client: &Client) -> Result<(), &'static str> {
    let _ = client.user_id().ok_or("v-pusher.no-session")?;
    let rules = read_rules(client).await?;
    if policy_is_confirmed(&rules) {
        return Ok(());
    }

    // PUT preserves an existing rule's order. Recreate only our own rule when
    // its order is wrong, so insertion places it above default mention rules.
    // Never delete/recreate unrelated rules or PATCH a whole ruleset.
    let existing = rules.override_.get(RULE_ID);
    let must_reposition = existing.is_some() && !precedes_notifying_overrides(&rules);
    if must_reposition {
        client
            .send(delete_pushrule::v3::Request::new(
                RuleKind::Override,
                RULE_ID.to_owned(),
            ))
            .await
            .map_err(|_| POLICY_FAILED)?;
    }
    let rule = NewPushRule::Override(NewConditionalPushRule::new(
        RULE_ID.to_owned(),
        conditions(),
        vec![],
    ));
    client
        .send(set_pushrule::v3::Request::new(rule))
        .await
        .map_err(|_| POLICY_FAILED)?;
    // Existing rules retain their enabled flag on PUT; new rules default on.
    if !must_reposition && existing.is_some_and(|rule| !rule.enabled) {
        client
            .send(set_pushrule_enabled::v3::Request::new(
                RuleKind::Override,
                RULE_ID.to_owned(),
                true,
            ))
            .await
            .map_err(|_| POLICY_FAILED)?;
    }
    if !policy_is_confirmed(&read_rules(client).await?) {
        return Err(POLICY_UNCONFIRMED);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::user_id;

    #[test]
    fn default_edit_rule_does_not_precede_mentions() {
        let rules = Ruleset::server_default(user_id!("@reader:example.org"));
        let edits = rules
            .override_
            .get_index_of(".m.rule.suppress_edits")
            .unwrap();
        let mentions = rules
            .override_
            .get_index_of(".m.rule.is_user_mention")
            .unwrap();
        assert!(mentions < edits);
        assert!(!policy_is_confirmed(&rules));
    }

    #[test]
    fn custom_override_preserves_other_rules_and_has_only_replacement_condition() {
        let mut rules = Ruleset::server_default(user_id!("@reader:example.org"));
        let original = serde_json::to_value(&rules).unwrap();
        rules
            .insert(
                NewPushRule::Override(NewConditionalPushRule::new(
                    RULE_ID.to_owned(),
                    conditions(),
                    vec![],
                )),
                None,
                None,
            )
            .unwrap();
        assert!(policy_is_confirmed(&rules));
        let serialized = serde_json::to_value(rules.override_.get(RULE_ID).unwrap()).unwrap();
        assert_eq!(
            serialized["conditions"],
            serde_json::json!([{
                "kind":"event_property_is", "key":r"content.m\.relates_to.rel_type", "value":"m.replace"
            }])
        );
        assert_eq!(serialized["actions"], serde_json::json!([]));
        rules.remove(RuleKind::Override, RULE_ID).unwrap();
        assert_eq!(serde_json::to_value(rules).unwrap(), original);
    }

    #[tokio::test]
    async fn edits_with_mentions_are_suppressed_and_new_message_actions_are_preserved() {
        use matrix_sdk::ruma::{push::PushConditionRoomCtx, room_id, serde::Raw};
        let user = user_id!("@reader:example.org");
        let context = PushConditionRoomCtx::new(
            room_id!("!room:example.org").to_owned(),
            2u32.into(),
            user.to_owned(),
            "Reader".to_owned(),
        );
        let baseline = Ruleset::server_default(user);
        let mut repaired = baseline.clone();
        repaired
            .insert(
                NewPushRule::Override(NewConditionalPushRule::new(
                    RULE_ID.to_owned(),
                    conditions(),
                    vec![],
                )),
                None,
                None,
            )
            .unwrap();
        for encrypted in [false, true] {
            for relation in [
                None,
                Some("m.thread"),
                Some("m.replace"),
                Some("m.replacement"),
            ] {
                let mut event = serde_json::json!({
                    "sender":"@sender:example.org",
                    "type": if encrypted { "m.room.encrypted" } else { "m.room.message" },
                    "content": {"msgtype":"m.text", "body":"Reader, update", "m.mentions":{"user_ids":[user]}}
                });
                if let Some(relation) = relation {
                    event["content"]["m.relates_to"] =
                        serde_json::json!({"rel_type":relation, "event_id":"$original"});
                }
                let event: Raw<serde_json::Value> =
                    Raw::from_json_string(event.to_string()).unwrap();
                let before = baseline.get_actions(&event, &context).await;
                assert!(before.iter().any(|action| action.should_notify()));
                let after = repaired.get_actions(&event, &context).await;
                if relation == Some("m.replace") {
                    assert!(after.is_empty());
                } else {
                    assert_eq!(after, before);
                }
            }
        }
    }

    #[test]
    fn disabled_or_shadowed_policy_needs_repair() {
        let mut rules = Ruleset::server_default(user_id!("@reader:example.org"));
        rules
            .insert(
                NewPushRule::Override(NewConditionalPushRule::new(
                    RULE_ID.to_owned(),
                    conditions(),
                    vec![],
                )),
                None,
                None,
            )
            .unwrap();
        rules
            .set_enabled(RuleKind::Override, RULE_ID, false)
            .unwrap();
        assert!(!policy_is_confirmed(&rules));
        rules
            .set_enabled(RuleKind::Override, RULE_ID, true)
            .unwrap();
        rules
            .insert(
                NewPushRule::Override(NewConditionalPushRule::new(
                    "example.other-notify".to_owned(),
                    vec![],
                    vec![matrix_sdk::ruma::push::Action::Notify],
                )),
                None,
                None,
            )
            .unwrap();
        assert!(!policy_is_confirmed(&rules));
    }
}

#[cfg(test)]
#[path = "edit_policy_route_tests.rs"]
mod route_tests;
