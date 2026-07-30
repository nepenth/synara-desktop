//! Live V-ROOMS.2a space parent-map projection.
//!
//! Builds the child→parents map used by nav/unread rollup from joined spaces'
//! `m.space.child` state. Lobby hierarchy mutations remain a later residual.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use matrix_sdk::{
    deserialized_responses::RawSyncOrStrippedState,
    ruma::events::space::child::SpaceChildEventContent, Client, Room, RoomState,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceParentEntry {
    pub room_id: String,
    pub parent_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSpaceParentsSnapshot {
    pub session_generation: u64,
    pub entries: Vec<NativeSpaceParentEntry>,
}

pub async fn snapshot_space_parents(
    client: &Client,
    session_generation: u64,
) -> Result<NativeSpaceParentsSnapshot, &'static str> {
    let mut child_to_parents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for room in client.joined_rooms() {
        if room.state() != RoomState::Joined || !room.is_space() {
            continue;
        }
        let parent_id = room.room_id().to_string();
        let children = space_child_ids(&room).await?;
        for child_id in children {
            if child_id == parent_id {
                continue;
            }
            if would_introduce_cycle(&child_to_parents, &parent_id, &child_id) {
                continue;
            }
            child_to_parents
                .entry(child_id)
                .or_default()
                .insert(parent_id.clone());
        }
    }

    let mut entries: Vec<NativeSpaceParentEntry> = child_to_parents
        .into_iter()
        .map(|(room_id, parents)| NativeSpaceParentEntry {
            room_id,
            parent_ids: parents.into_iter().collect(),
        })
        .collect();
    entries.sort_by(|a, b| a.room_id.cmp(&b.room_id));

    Ok(NativeSpaceParentsSnapshot {
        session_generation,
        entries,
    })
}

async fn space_child_ids(room: &Room) -> Result<Vec<String>, &'static str> {
    let events = room
        .get_state_events_static::<SpaceChildEventContent>()
        .await
        .map_err(|_| "v-rooms.2a-space-child-state-failed")?;
    let mut children = Vec::new();
    for raw in events {
        let Some(child_id) = valid_child_room_id(&raw) else {
            continue;
        };
        children.push(child_id);
    }
    children.sort();
    children.dedup();
    Ok(children)
}

fn valid_child_room_id(raw: &RawSyncOrStrippedState<SpaceChildEventContent>) -> Option<String> {
    match raw {
        RawSyncOrStrippedState::Sync(raw) => {
            let event = raw.deserialize().ok()?;
            let original = event.as_original()?;
            // Match Synara `isValidChild`: content must expose a `via` array.
            let _via = &original.content.via;
            Some(original.state_key.to_string())
        }
        RawSyncOrStrippedState::Stripped(raw) => {
            let event = raw.deserialize().ok()?;
            if !event.content.is_valid() {
                return None;
            }
            Some(event.state_key.to_string())
        }
    }
}

/// Reject edges that would make `child` an ancestor of `parent` (JS cycle guard).
fn would_introduce_cycle(
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
        // Existing edge: !a is a parent of !b.
        map.insert(
            "!b:example.org".into(),
            BTreeSet::from(["!a:example.org".into()]),
        );
        // Adding !b as a parent of !a would cycle.
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
}
