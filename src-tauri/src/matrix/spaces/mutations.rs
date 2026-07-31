//! Live V-ROOMS.2c space child mutation ownership.
//!
//! Owns `m.space.child` set/remove and restricted join-rule allow-list updates
//! used by lobby reorder, suggested toggle, add-existing, and create-room parent link.

use matrix_sdk::{
    ruma::{
        events::{room::join_rules::RoomJoinRulesEventContent, space::child::SpaceChildEventContent},
        room::{AllowRule, JoinRule},
        OwnedRoomId, OwnedServerName, OwnedSpaceChildOrder, SpaceChildOrder,
    },
    Client, RoomState,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceChildMutationResult {
    pub parent_id: String,
    pub child_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeJoinRulesMutationResult {
    pub room_id: String,
    pub status: &'static str,
}

/// Product input for setting/updating an `m.space.child` edge.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceChildSetArgs {
    pub parent_id: String,
    pub child_id: String,
    pub via: Vec<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub suggested: bool,
}

/// Product input for restricted/knock_restricted join-rule allow lists.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRulesSetArgs {
    pub room_id: String,
    pub join_rule: String,
    #[serde(default)]
    pub allow: Vec<JoinRuleAllowArg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRuleAllowArg {
    /// Matrix allow type; product uses `m.room_membership`.
    #[serde(default = "default_room_membership_type")]
    pub r#type: String,
    pub room_id: String,
}

fn default_room_membership_type() -> String {
    "m.room_membership".into()
}

pub async fn set_space_child(
    client: &Client,
    args: &SpaceChildSetArgs,
) -> Result<NativeSpaceChildMutationResult, &'static str> {
    let parent_id = parse_room_id(&args.parent_id, "v-rooms.2c-space-child-invalid-parent")?;
    let child_id = parse_room_id(&args.child_id, "v-rooms.2c-space-child-invalid-child")?;
    if parent_id == child_id {
        return Err("v-rooms.2c-space-child-self-parent");
    }
    let via = parse_via_servers(&args.via)?;
    let order = parse_order(args.order.as_deref())?;
    let mut content = SpaceChildEventContent::new(via);
    content.order = order;
    content.suggested = args.suggested;

    let room = require_joined_room(client, &parent_id)?;
    room.send_state_event_for_key(&child_id, content)
        .await
        .map_err(|_| "v-rooms.2c-space-child-set-failed")?;

    Ok(NativeSpaceChildMutationResult {
        parent_id: parent_id.to_string(),
        child_id: child_id.to_string(),
        status: "updated",
    })
}

pub async fn remove_space_child(
    client: &Client,
    parent_id: &str,
    child_id: &str,
) -> Result<NativeSpaceChildMutationResult, &'static str> {
    let parent_id = parse_room_id(parent_id, "v-rooms.2c-space-child-invalid-parent")?;
    let child_id = parse_room_id(child_id, "v-rooms.2c-space-child-invalid-child")?;
    if parent_id == child_id {
        return Err("v-rooms.2c-space-child-self-parent");
    }

    let room = require_joined_room(client, &parent_id)?;
    // Match product JS: empty content clears the child edge (invalid without `via`).
    room.send_state_event_raw("m.space.child", child_id.as_str(), serde_json::json!({}))
        .await
        .map_err(|_| "v-rooms.2c-space-child-remove-failed")?;

    Ok(NativeSpaceChildMutationResult {
        parent_id: parent_id.to_string(),
        child_id: child_id.to_string(),
        status: "updated",
    })
}

pub async fn set_room_join_rules(
    client: &Client,
    args: &JoinRulesSetArgs,
) -> Result<NativeJoinRulesMutationResult, &'static str> {
    let room_id = parse_room_id(&args.room_id, "v-rooms.2c-join-rules-invalid-room")?;
    let content = build_join_rules_content(&args.join_rule, &args.allow)?;
    let room = require_joined_room(client, &room_id)?;
    room.send_state_event(content)
        .await
        .map_err(|_| "v-rooms.2c-join-rules-set-failed")?;
    Ok(NativeJoinRulesMutationResult {
        room_id: room_id.to_string(),
        status: "updated",
    })
}

fn build_join_rules_content(
    join_rule: &str,
    allow: &[JoinRuleAllowArg],
) -> Result<RoomJoinRulesEventContent, &'static str> {
    let allow_rules = parse_allow_rules(allow)?;
    match join_rule {
        "public" => Ok(RoomJoinRulesEventContent::new(JoinRule::Public)),
        "invite" => Ok(RoomJoinRulesEventContent::new(JoinRule::Invite)),
        "knock" => Ok(RoomJoinRulesEventContent::new(JoinRule::Knock)),
        "private" => Ok(RoomJoinRulesEventContent::new(JoinRule::Private)),
        "restricted" => Ok(RoomJoinRulesEventContent::restricted(allow_rules)),
        "knock_restricted" => Ok(RoomJoinRulesEventContent::knock_restricted(allow_rules)),
        _ => Err("v-rooms.2c-join-rules-invalid-rule"),
    }
}

fn parse_allow_rules(allow: &[JoinRuleAllowArg]) -> Result<Vec<AllowRule>, &'static str> {
    let mut rules = Vec::with_capacity(allow.len());
    for entry in allow {
        if entry.r#type != "m.room_membership" {
            return Err("v-rooms.2c-join-rules-invalid-allow-type");
        }
        let room_id = parse_room_id(&entry.room_id, "v-rooms.2c-join-rules-invalid-allow-room")?;
        rules.push(AllowRule::room_membership(room_id));
    }
    Ok(rules)
}

fn require_joined_room<'a>(
    client: &'a Client,
    room_id: &OwnedRoomId,
) -> Result<matrix_sdk::Room, &'static str> {
    let room = client
        .get_room(room_id)
        .ok_or("v-rooms.2c-space-child-room-missing")?;
    if room.state() != RoomState::Joined {
        return Err("v-rooms.2c-space-child-room-not-joined");
    }
    Ok(room)
}

fn parse_room_id(value: &str, err: &'static str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(value.trim()).map_err(|_| err)
}

fn parse_via_servers(via: &[String]) -> Result<Vec<OwnedServerName>, &'static str> {
    let mut servers = Vec::with_capacity(via.len());
    for server in via {
        let trimmed = server.trim();
        if trimmed.is_empty() {
            return Err("v-rooms.2c-space-child-invalid-via");
        }
        servers.push(
            OwnedServerName::try_from(trimmed).map_err(|_| "v-rooms.2c-space-child-invalid-via")?,
        );
    }
    Ok(servers)
}

fn parse_order(order: Option<&str>) -> Result<Option<OwnedSpaceChildOrder>, &'static str> {
    match order {
        None => Ok(None),
        Some(value) => SpaceChildOrder::parse(value.trim())
            .map(Some)
            .map_err(|_| "v-rooms.2c-space-child-invalid-order"),
    }
}

/// Pure builder for unit tests without a live client.
pub fn build_space_child_content_for_test(
    via: &[&str],
    order: Option<&str>,
    suggested: bool,
) -> Result<SpaceChildEventContent, &'static str> {
    let via = parse_via_servers(
        &via.iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>(),
    )?;
    let order = parse_order(order)?;
    let mut content = SpaceChildEventContent::new(via);
    content.order = order;
    content.suggested = suggested;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_result_serializes_camel_case() {
        let result = NativeSpaceChildMutationResult {
            parent_id: "!space:example.org".into(),
            child_id: "!room:example.org".into(),
            status: "updated",
        };
        let value = serde_json::to_value(&result).expect("serialize");
        assert_eq!(value["parentId"], "!space:example.org");
        assert_eq!(value["childId"], "!room:example.org");
        assert_eq!(value["status"], "updated");
    }

    #[test]
    fn set_args_deserialize_camel_case() {
        let json = serde_json::json!({
            "parentId": "!space:example.org",
            "childId": "!room:example.org",
            "via": ["example.org"],
            "order": "a0",
            "suggested": true
        });
        let args: SpaceChildSetArgs = serde_json::from_value(json).expect("deserialize");
        assert_eq!(args.parent_id, "!space:example.org");
        assert_eq!(args.child_id, "!room:example.org");
        assert_eq!(args.via, vec!["example.org".to_string()]);
        assert_eq!(args.order.as_deref(), Some("a0"));
        assert!(args.suggested);
    }

    #[test]
    fn space_child_content_includes_order_and_suggested() {
        let content = build_space_child_content_for_test(&["example.org"], Some("a0"), true)
            .expect("content");
        let value = serde_json::to_value(&content).expect("serialize");
        assert_eq!(value["via"][0], "example.org");
        assert_eq!(value["order"], "a0");
        assert_eq!(value["suggested"], true);
    }

    #[test]
    fn space_child_content_omits_default_suggested() {
        let content =
            build_space_child_content_for_test(&["example.org"], None, false).expect("content");
        let value = serde_json::to_value(&content).expect("serialize");
        assert!(value.get("order").is_none());
        assert!(value.get("suggested").is_none() || value["suggested"] == false);
    }

    #[test]
    fn invalid_order_is_rejected() {
        assert_eq!(
            build_space_child_content_for_test(&["example.org"], Some("🔝"), false)
                .expect_err("invalid"),
            "v-rooms.2c-space-child-invalid-order"
        );
    }

    #[test]
    fn join_rules_restricted_builds_allow_list() {
        let content = build_join_rules_content(
            "restricted",
            &[JoinRuleAllowArg {
                r#type: "m.room_membership".into(),
                room_id: "!space:example.org".into(),
            }],
        )
        .expect("content");
        match content.join_rule {
            JoinRule::Restricted(restricted) => {
                assert_eq!(restricted.allow.len(), 1);
            }
            other => panic!("expected restricted, got {other:?}"),
        }
    }

    #[test]
    fn join_rules_unknown_rule_rejected() {
        assert_eq!(
            build_join_rules_content("custom", &[]).expect_err("invalid"),
            "v-rooms.2c-join-rules-invalid-rule"
        );
    }

    #[test]
    fn join_rules_args_deserialize_camel_case() {
        let json = serde_json::json!({
            "roomId": "!room:example.org",
            "joinRule": "restricted",
            "allow": [{ "type": "m.room_membership", "roomId": "!space:example.org" }]
        });
        let args: JoinRulesSetArgs = serde_json::from_value(json).expect("deserialize");
        assert_eq!(args.room_id, "!room:example.org");
        assert_eq!(args.join_rule, "restricted");
        assert_eq!(args.allow[0].room_id, "!space:example.org");
    }
}
