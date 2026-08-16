//! Credential-free `m.direct` snapshot DTO and string-map mutate helpers.
//!
//! Live Client load/store is in `mdirect_live` and is owned by the image-pack
//! session owner.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// user_id → room_ids, matching the `m.direct` account-data map.
pub type MDirectRooms = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMDirectSnapshot {
    pub session_generation: u64,
    pub room_ids: Vec<String>,
    /// User keys in `m.direct` that still have at least one joined DM room.
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMDirectMutationResult {
    pub room_id: String,
    pub status: &'static str,
}

pub fn snapshot_from_mdirect_rooms(
    content: &MDirectRooms,
    joined_rooms: &BTreeSet<String>,
    session_generation: u64,
) -> NativeMDirectSnapshot {
    let mut room_ids = BTreeSet::new();
    let mut user_ids = BTreeSet::new();
    for (user, rooms) in content {
        let mut has_joined = false;
        for room_id in rooms {
            room_ids.insert(room_id.clone());
            if joined_rooms.contains(room_id) {
                has_joined = true;
            }
        }
        if has_joined {
            user_ids.insert(user.clone());
        }
    }
    NativeMDirectSnapshot {
        session_generation,
        room_ids: room_ids.into_iter().collect(),
        user_ids: user_ids.into_iter().collect(),
    }
}

/// Product parity with legacy JS: a room is a DM for at most one user key.
pub fn apply_add_mdirect_room(content: &mut MDirectRooms, room_id: &str, user_id: &str) {
    for (key, rooms) in content.iter_mut() {
        if key == user_id {
            continue;
        }
        rooms.retain(|id| id != room_id);
    }
    let rooms = content.entry(user_id.to_owned()).or_default();
    if !rooms.iter().any(|id| id == room_id) {
        rooms.push(room_id.to_owned());
    }
}

pub fn apply_remove_mdirect_room(content: &mut MDirectRooms, room_id: &str) {
    for rooms in content.values_mut() {
        rooms.retain(|id| id != room_id);
    }
    content.retain(|_, rooms| !rooms.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = NativeMDirectSnapshot {
            session_generation: 4,
            room_ids: vec!["!dm:example.org".into()],
            user_ids: vec!["@bob:example.org".into()],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 4);
        assert_eq!(value["roomIds"][0], "!dm:example.org");
        assert_eq!(value["userIds"][0], "@bob:example.org");
    }

    #[test]
    fn snapshot_user_ids_require_joined_room() {
        let mut content = MDirectRooms::new();
        content.insert(
            "@alice:example.org".into(),
            vec!["!joined:example.org".into()],
        );
        content.insert("@bob:example.org".into(), vec!["!left:example.org".into()]);
        let mut joined_rooms = BTreeSet::new();
        joined_rooms.insert("!joined:example.org".into());
        let snap = snapshot_from_mdirect_rooms(&content, &joined_rooms, 1);
        assert_eq!(snap.user_ids, vec!["@alice:example.org".to_string()]);
        assert_eq!(snap.room_ids.len(), 2);
    }

    #[test]
    fn add_moves_room_to_single_user() {
        let mut content = MDirectRooms::new();
        content.insert("@bob:example.org".into(), vec!["!dm:example.org".into()]);
        apply_add_mdirect_room(&mut content, "!dm:example.org", "@alice:example.org");
        assert_eq!(
            content.get("@alice:example.org"),
            Some(&vec!["!dm:example.org".to_string()])
        );
        assert!(content
            .get("@bob:example.org")
            .map(|r| r.is_empty())
            .unwrap_or(true));
    }

    #[test]
    fn remove_clears_room_from_all_users() {
        let mut content = MDirectRooms::new();
        content.insert(
            "@alice:example.org".into(),
            vec!["!dm:example.org".into(), "!other:example.org".into()],
        );
        apply_remove_mdirect_room(&mut content, "!dm:example.org");
        assert_eq!(
            content.get("@alice:example.org"),
            Some(&vec!["!other:example.org".to_string()])
        );
    }
}
