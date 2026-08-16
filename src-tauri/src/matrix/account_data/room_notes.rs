//! Live `in.synara.room_notes` Client RMW. Codec types live in synara-core.
//!
//! Implementation lives in synara-core. This module keeps the desktop
//! `crate::matrix::account_data::room_notes::*` path resolving.

pub use synara_core::app::account_data::{
    complete_room_todo_item, complete_room_todo_item_live, delete_room_note_item_live,
    move_room_todo_item, move_room_todo_item_live, normalize_room_note_item,
    normalize_room_notes_content, put_room_note_item, remove_room_note_item, snapshot_room_notes,
    upsert_room_note_item, NativeRoomNotesSnapshot, RoomNoteMoveDirection, SynaraRoomNoteItem,
    SynaraRoomNoteItemKind, SynaraRoomNotesContent, SynaraRoomNotesRoom, MAX_MESSAGE_BODY_LENGTH,
    MAX_NOTE_BODY_LENGTH, ROOM_NOTES_ACCOUNT_DATA_VERSION, ROOM_NOTES_EVENT_TYPE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_enforces_body_caps_and_kind_rules() {
        let long = "x".repeat(5000);
        let value = json!({
            "version": 1,
            "rooms": {
                "!r:s": {
                    "items": {
                        "note:1": {
                            "id": "note:1",
                            "kind": "note",
                            "roomId": "!r:s",
                            "createdAt": 1.0,
                            "updatedAt": 2.0,
                            "body": long
                        },
                        "message:1": {
                            "id": "message:1",
                            "kind": "message",
                            "roomId": "!r:s",
                            "createdAt": 1.0,
                            "updatedAt": 2.0
                        }
                    }
                }
            }
        });
        let content = normalize_room_notes_content(Some(&value));
        assert_eq!(content.rooms["!r:s"].items.len(), 1);
        let body = content.rooms["!r:s"].items["note:1"].body.as_ref().unwrap();
        assert_eq!(body.chars().count(), MAX_NOTE_BODY_LENGTH);
    }

    #[test]
    fn complete_and_move_todo_items() {
        let mut content = SynaraRoomNotesContent::default();
        content = put_room_note_item(
            content,
            SynaraRoomNoteItem {
                id: "todo:a".into(),
                kind: SynaraRoomNoteItemKind::Todo,
                room_id: "!r:s".into(),
                created_at: 1.0,
                updated_at: 1.0,
                body: Some("a".into()),
                completed_at: None,
                order: Some(2.0),
                event_id: None,
                event_ts: None,
                sender: None,
            },
        );
        content = put_room_note_item(
            content,
            SynaraRoomNoteItem {
                id: "todo:b".into(),
                kind: SynaraRoomNoteItemKind::Todo,
                room_id: "!r:s".into(),
                created_at: 1.0,
                updated_at: 1.0,
                body: Some("b".into()),
                completed_at: None,
                order: Some(1.0),
                event_id: None,
                event_ts: None,
                sender: None,
            },
        );
        let moved = move_room_todo_item(
            content.clone(),
            "!r:s",
            "todo:a",
            RoomNoteMoveDirection::Down,
            10.0,
        );
        assert_eq!(moved.rooms["!r:s"].items["todo:a"].order, Some(1.0));
        assert_eq!(moved.rooms["!r:s"].items["todo:b"].order, Some(2.0));
        let completed = complete_room_todo_item(moved, "!r:s", "todo:a", true, 11.0);
        assert_eq!(
            completed.rooms["!r:s"].items["todo:a"].completed_at,
            Some(11.0)
        );
    }
}
