//! Room folders in the room list: the section rule, `folder_id` on rows,
//! and the per-folder presentation (order and unread rollup).
//!
//! Section rule, for every client: a joined favorite shows in Favorites; a
//! joined room that is neither a DM nor a space and is filed shows in its
//! folder; everything else goes where the client puts it today (Rooms, a
//! space group, low priority, DMs). A favorite keeps its folder membership,
//! so unfavoriting returns it to the folder. A filed low-priority room stays
//! in its folder and sorts last there. See `docs/design/room-folders.md`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::app::account_data::{NativeRoomFolder, RoomFoldersDocument};
use crate::dto::{Membership, RoomSummary};

use super::presentation::{order_room_ids, room_attention, RoomListOrder, RoomOrderInput};

/// The fields the section rule reads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Record))]
pub struct RoomSectionInput {
    pub is_joined: bool,
    pub is_direct: bool,
    pub is_space: bool,
    pub is_favorite: bool,
    /// The folder the room is filed in, if any (the row's `folder_id`).
    pub folder_id: Option<String>,
}

/// Where one room shows in the room list.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "full-uniffi", derive(uniffi::Enum))]
pub enum RoomSectionPlacement {
    /// The built-in Favorites section.
    Favorites,
    /// A user folder.
    Folder { folder_id: String },
    /// Wherever the client puts the room without folders (Rooms, a space
    /// group, low priority, direct messages).
    Default,
}

/// Rooms only: joined, not a DM, not a space.
pub fn is_room_folder_eligible(is_joined: bool, is_direct: bool, is_space: bool) -> bool {
    is_joined && !is_direct && !is_space
}

/// The section rule. Favorites win over folders.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn room_section_placement(input: RoomSectionInput) -> RoomSectionPlacement {
    if input.is_joined && input.is_favorite {
        return RoomSectionPlacement::Favorites;
    }
    match input.folder_id {
        Some(folder_id)
            if is_room_folder_eligible(input.is_joined, input.is_direct, input.is_space) =>
        {
            RoomSectionPlacement::Folder { folder_id }
        }
        _ => RoomSectionPlacement::Default,
    }
}

fn section_input(room: &RoomSummary, folder_id: Option<&str>) -> RoomSectionInput {
    RoomSectionInput {
        is_joined: room.membership == Membership::Join,
        is_direct: room.is_direct,
        is_space: room.is_space,
        is_favorite: room.is_favorite,
        folder_id: folder_id.map(str::to_owned),
    }
}

/// [`room_section_placement`] for a row whose `folder_id` is already set.
pub fn room_section_placement_for(room: &RoomSummary) -> RoomSectionPlacement {
    room_section_placement(section_input(room, room.folder_id.as_deref()))
}

/// Set `folder_id` on every row from `folders`. Only rooms that can be
/// filed get one, so a DM, a space or an invite never carries a folder.
/// Favorites keep theirs (membership survives; the section rule hides it).
pub fn apply_room_folders(rooms: &mut [RoomSummary], folders: &RoomFoldersDocument) {
    let assignments = folders.assignments();
    for room in rooms {
        room.folder_id = is_room_folder_eligible(
            room.membership == Membership::Join,
            room.is_direct,
            room.is_space,
        )
        .then(|| assignments.get(room.room_id.as_str()))
        .flatten()
        .map(|folder_id| (*folder_id).to_owned());
    }
}

/// One folder as the room list renders it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct RoomFolderPresentation {
    pub id: String,
    pub name: String,
    /// Rooms that show in this folder (listed, joined, not DMs or spaces,
    /// favorites excluded), in filing order.
    pub room_ids: Vec<String>,
    /// `room_ids` newest activity first, low priority last.
    pub recent_order: Vec<String>,
    /// `room_ids` by name, low priority last.
    pub name_order: Vec<String>,
    /// Mentions and unread total over `room_ids`, with the per-room
    /// attention rules (muted rooms count nothing, marked-unread counts one).
    pub unread_highlight: u32,
    pub unread_total: u32,
    /// The rooms in `room_ids` that need attention, sorted by room id.
    pub unread_room_ids: Vec<String>,
}

fn order_input(room: &RoomSummary) -> RoomOrderInput {
    RoomOrderInput {
        room_id: room.room_id.clone(),
        name: room.name.clone(),
        last_activity_ms: room.last_activity_ts,
        is_low_priority: room.is_low_priority,
    }
}

/// Every folder in display order, empty folders included. Rows not in
/// `rooms` (left, not yet synced) are skipped.
pub fn room_folders_presentation(
    rooms: &[RoomSummary],
    folders: &[NativeRoomFolder],
) -> Vec<RoomFolderPresentation> {
    let mut members: HashMap<&str, &str> = HashMap::new();
    for folder in folders {
        for room_id in &folder.room_ids {
            members
                .entry(room_id.as_str())
                .or_insert(folder.id.as_str());
        }
    }
    // Snapshot order (SDK recency) per folder, so the Recent order breaks
    // ties the way the Rooms section does.
    let mut shown: HashMap<&str, Vec<&RoomSummary>> = HashMap::new();
    for room in rooms {
        let Some(folder_id) = members.get(room.room_id.as_str()) else {
            continue;
        };
        if room_section_placement(section_input(room, Some(folder_id)))
            == (RoomSectionPlacement::Folder {
                folder_id: (*folder_id).to_owned(),
            })
        {
            shown.entry(folder_id).or_default().push(room);
        }
    }
    folders
        .iter()
        .map(|folder| {
            let rows = shown.remove(folder.id.as_str()).unwrap_or_default();
            let by_id: HashMap<&str, &RoomSummary> = rows
                .iter()
                .map(|room| (room.room_id.as_str(), *room))
                .collect();
            let room_ids: Vec<String> = folder
                .room_ids
                .iter()
                .filter(|room_id| by_id.contains_key(room_id.as_str()))
                .cloned()
                .collect();
            let inputs: Vec<RoomOrderInput> = rows.iter().map(|room| order_input(room)).collect();
            let mut unread_highlight = 0u32;
            let mut unread_total = 0u32;
            let mut unread_room_ids = Vec::new();
            for attention in rows.iter().filter_map(|room| room_attention(room)) {
                unread_highlight = unread_highlight.saturating_add(attention.highlight);
                unread_total = unread_total.saturating_add(attention.total);
                unread_room_ids.push(attention.room_id);
            }
            unread_room_ids.sort();
            RoomFolderPresentation {
                id: folder.id.clone(),
                name: folder.name.clone(),
                room_ids,
                recent_order: order_room_ids(inputs.clone(), RoomListOrder::Recent),
                name_order: order_room_ids(inputs, RoomListOrder::Name),
                unread_highlight,
                unread_total,
                unread_room_ids,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::app::account_data::normalize_room_folders_content;
    use crate::app::room_list::summary::RoomSummaryBuilder;
    use crate::dto::NotificationMode;

    fn input(
        joined: bool,
        direct: bool,
        space: bool,
        favorite: bool,
        folder: Option<&str>,
    ) -> RoomSectionInput {
        RoomSectionInput {
            is_joined: joined,
            is_direct: direct,
            is_space: space,
            is_favorite: favorite,
            folder_id: folder.map(str::to_owned),
        }
    }

    fn folder(id: &str) -> RoomSectionPlacement {
        RoomSectionPlacement::Folder {
            folder_id: id.to_owned(),
        }
    }

    #[test]
    fn section_rule_puts_favorites_first_then_folders() {
        use RoomSectionPlacement::{Default, Favorites};
        let cases = [
            (input(true, false, false, true, Some("f")), Favorites),
            (input(true, false, false, false, Some("f")), folder("f")),
            (input(true, false, false, false, None), Default),
            // DMs and spaces cannot be filed; a favorite DM is a favorite.
            (input(true, true, false, false, Some("f")), Default),
            (input(true, true, false, true, Some("f")), Favorites),
            (input(true, false, true, false, Some("f")), Default),
            // Invites never show in a folder or in Favorites.
            (input(false, false, false, true, Some("f")), Default),
            (input(false, false, false, false, Some("f")), Default),
        ];
        for (input, expected) in cases {
            assert_eq!(room_section_placement(input.clone()), expected, "{input:?}");
        }
    }

    fn folders() -> RoomFoldersDocument {
        normalize_room_folders_content(Some(&json!({
            "version": 1,
            "folders": [
                { "id": "work", "name": "Work", "rooms": ["!c:x", "!a:x", "!fav:x", "!dm:x", "!gone:x", "!quiet:x", "!muted:x"] },
                { "id": "empty", "name": "Empty", "rooms": [] },
                { "id": "space", "name": "Space", "rooms": ["!space:x", "!invite:x"] }
            ]
        })))
    }

    fn rows() -> Vec<RoomSummary> {
        let mut space = RoomSummaryBuilder::new("!space:x").build().unwrap();
        space.is_space = true;
        let mut muted = RoomSummaryBuilder::new("!muted:x")
            .name("Muted")
            .unread(9, 9)
            .build()
            .unwrap();
        muted.notification_mode = Some(NotificationMode::Mute);
        vec![
            RoomSummaryBuilder::new("!a:x")
                .name("Alpha")
                .last_activity_ts(10)
                .unread(2, 1)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!quiet:x")
                .name("Aardvark")
                .last_activity_ts(99)
                .low_priority(true)
                .marked_unread(true)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!c:x")
                .name("Charlie")
                .last_activity_ts(50)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!fav:x")
                .name("Fav")
                .favorite(true)
                .unread(5, 5)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!dm:x")
                .direct(true)
                .unread(1, 1)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!invite:x")
                .membership(Membership::Invite)
                .build()
                .unwrap(),
            RoomSummaryBuilder::new("!plain:x").build().unwrap(),
            space,
            muted,
        ]
    }

    #[test]
    fn apply_sets_folder_ids_only_on_rooms_that_can_be_filed() {
        let mut rooms = rows();
        apply_room_folders(&mut rooms, &folders());
        let by_id: HashMap<_, _> = rooms
            .iter()
            .map(|room| (room.room_id.as_str(), room.folder_id.as_deref()))
            .collect();
        assert_eq!(by_id["!a:x"], Some("work"));
        // A favorite keeps its membership.
        assert_eq!(by_id["!fav:x"], Some("work"));
        assert_eq!(by_id["!dm:x"], None);
        assert_eq!(by_id["!space:x"], None);
        assert_eq!(by_id["!invite:x"], None);
        assert_eq!(by_id["!plain:x"], None);
        let fav = rooms.iter().find(|room| room.room_id == "!fav:x").unwrap();
        assert_eq!(
            room_section_placement_for(fav),
            RoomSectionPlacement::Favorites
        );
        let a = rooms.iter().find(|room| room.room_id == "!a:x").unwrap();
        assert_eq!(room_section_placement_for(a), folder("work"));
        // Unfavoriting returns the room to its folder.
        let mut unfavorited = fav.clone();
        unfavorited.is_favorite = false;
        assert_eq!(room_section_placement_for(&unfavorited), folder("work"));
    }

    #[test]
    fn folder_presentation_orders_rooms_and_rolls_up_unread_without_favorites() {
        let document = folders();
        let presentation = room_folders_presentation(&rows(), &document.snapshot(1).folders);
        let ids: Vec<_> = presentation.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["work", "empty", "space"]);
        let work = &presentation[0];
        assert_eq!(work.name, "Work");
        // Filing order; favorite, DM and unlisted rooms are not shown.
        assert_eq!(work.room_ids, ["!c:x", "!a:x", "!quiet:x", "!muted:x"]);
        // Low priority last in both orders.
        assert_eq!(work.recent_order, ["!c:x", "!a:x", "!muted:x", "!quiet:x"]);
        assert_eq!(work.name_order, ["!a:x", "!c:x", "!muted:x", "!quiet:x"]);
        // Favorite (5/5) and muted (9/9) rooms do not count; marked unread counts one.
        assert_eq!((work.unread_highlight, work.unread_total), (1, 3));
        assert_eq!(work.unread_room_ids, ["!a:x", "!quiet:x"]);
        assert!(presentation[1].room_ids.is_empty());
        assert!(presentation[2].room_ids.is_empty());
        assert_eq!(presentation[2].unread_total, 0);
    }
}
