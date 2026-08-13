//! Credential-free V-ROOMS.2 space presentation DTOs and cycle guard.
//!
//! Live Client hierarchy/child I/O lives in `live.rs`.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceParentEntry {
    pub room_id: String,
    pub parent_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceParentsSnapshot {
    pub session_generation: u64,
    pub entries: Vec<NativeSpaceParentEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceHierarchyRoom {
    pub room_id: String,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub room_type: Option<String>,
    pub num_joined_members: u64,
    pub join_rule: String,
    pub world_readable: bool,
    pub guest_can_join: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceHierarchySnapshot {
    pub session_generation: u64,
    pub rooms: Vec<NativeSpaceHierarchyRoom>,
}

/// One valid local `m.space.child` edge from a joined space room state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceChildEdge {
    pub parent_id: String,
    pub child_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    pub suggested: bool,
    pub via: Vec<String>,
    pub origin_server_ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceChildrenSnapshot {
    pub session_generation: u64,
    pub edges: Vec<NativeSpaceChildEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceChildMutationResult {
    pub parent_id: String,
    pub child_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRestrictedJoinReparentResult {
    pub room_id: String,
    pub status: &'static str,
}

/// Reject edges that would make `child` an ancestor of `parent` (JS cycle guard).
pub fn would_introduce_cycle(
    map: &BTreeMap<String, BTreeSet<String>>,
    parent: &str,
    child: &str,
) -> bool {
    let mut stack = vec![parent.to_owned()];
    let mut seen = HashSet::new();
    while let Some(id) = stack.pop() {
        if id == child {
            return true;
        }
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(parents) = map.get(&id) {
            stack.extend(parents.iter().cloned());
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_guard_rejects_child_that_is_already_an_ancestor() {
        let mut map = BTreeMap::new();
        map.insert(
            "!b:example.org".into(),
            BTreeSet::from(["!a:example.org".into()]),
        );
        assert!(would_introduce_cycle(
            &map,
            "!b:example.org",
            "!a:example.org"
        ));
        assert!(!would_introduce_cycle(
            &map,
            "!c:example.org",
            "!b:example.org"
        ));
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = NativeSpaceParentsSnapshot {
            session_generation: 2,
            entries: vec![NativeSpaceParentEntry {
                room_id: "!room:example.org".into(),
                parent_ids: vec!["!space:example.org".into()],
            }],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 2);
        assert_eq!(value["entries"][0]["roomId"], "!room:example.org");
        assert_eq!(value["entries"][0]["parentIds"][0], "!space:example.org");
    }

    #[test]
    fn hierarchy_snapshot_serializes_only_product_fields() {
        let snap = NativeSpaceHierarchySnapshot {
            session_generation: 3,
            rooms: vec![NativeSpaceHierarchyRoom {
                room_id: "!room:example.org".into(),
                name: Some("Room".into()),
                canonical_alias: None,
                topic: Some("Topic".into()),
                avatar_url: None,
                room_type: Some("m.space".into()),
                num_joined_members: 4,
                join_rule: "public".into(),
                world_readable: true,
                guest_can_join: false,
            }],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 3);
        assert_eq!(value["rooms"][0]["roomId"], "!room:example.org");
        assert_eq!(value["rooms"][0]["numJoinedMembers"], 4);
        assert!(value["rooms"][0].get("children_state").is_none());
    }

    #[test]
    fn children_snapshot_serializes_edge_fields() {
        let snap = NativeSpaceChildrenSnapshot {
            session_generation: 5,
            edges: vec![NativeSpaceChildEdge {
                parent_id: "!space:example.org".into(),
                child_id: "!room:example.org".into(),
                order: Some("a".into()),
                suggested: true,
                via: vec!["example.org".into()],
                origin_server_ts: 42,
            }],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 5);
        assert_eq!(value["edges"][0]["parentId"], "!space:example.org");
        assert_eq!(value["edges"][0]["childId"], "!room:example.org");
        assert_eq!(value["edges"][0]["order"], "a");
        assert_eq!(value["edges"][0]["suggested"], true);
        assert_eq!(value["edges"][0]["via"][0], "example.org");
        assert_eq!(value["edges"][0]["originServerTs"], 42);
    }

    #[test]
    fn mutation_result_serializes_status() {
        let result = NativeSpaceChildMutationResult {
            parent_id: "!space:example.org".into(),
            child_id: "!room:example.org".into(),
            status: "updated",
        };
        let value = serde_json::to_value(&result).expect("serialize");
        assert_eq!(value["status"], "updated");
        assert_eq!(value["parentId"], "!space:example.org");
    }
}
