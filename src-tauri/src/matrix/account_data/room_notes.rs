//! Live `in.synara.room_notes` Client RMW. Codec types live in synara-core.

use matrix_sdk::{
    ruma::{
        events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType},
        serde::Raw,
    },
    Client,
};
use serde_json::value::to_raw_value;

pub use synara_core::app::account_data::{
    complete_room_todo_item, move_room_todo_item, normalize_room_note_item,
    normalize_room_notes_content, put_room_note_item, remove_room_note_item,
    NativeRoomNotesSnapshot, RoomNoteMoveDirection, SynaraRoomNoteItem, SynaraRoomNoteItemKind,
    SynaraRoomNotesContent, SynaraRoomNotesRoom, ROOM_NOTES_ACCOUNT_DATA_VERSION,
    ROOM_NOTES_EVENT_TYPE,
};

fn room_notes_event_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(ROOM_NOTES_EVENT_TYPE)
}
async fn load_room_notes_content(client: &Client) -> Result<SynaraRoomNotesContent, &'static str> {
    let raw = client
        .account()
        .account_data_raw(room_notes_event_type())
        .await
        .map_err(|_| "v-timeline-room-notes-fetch-failed")?;
    let value = match raw {
        Some(raw) => raw
            .deserialize_as_unchecked::<serde_json::Value>()
            .map_err(|_| "v-timeline-room-notes-deserialize-failed")?,
        None => return Ok(SynaraRoomNotesContent::default()),
    };
    Ok(normalize_room_notes_content(Some(&value)))
}

async fn store_room_notes_content(
    client: &Client,
    content: &SynaraRoomNotesContent,
) -> Result<(), &'static str> {
    let raw_value = to_raw_value(content).map_err(|_| "v-timeline-room-notes-serialize-failed")?;
    let raw = Raw::<AnyGlobalAccountDataEventContent>::from_json(raw_value);
    client
        .account()
        .set_account_data_raw(room_notes_event_type(), raw)
        .await
        .map_err(|_| "v-timeline-room-notes-set-failed")?;
    Ok(())
}

pub async fn snapshot_room_notes(
    client: &Client,
    session_generation: u64,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    Ok(NativeRoomNotesSnapshot {
        session_generation,
        content: load_room_notes_content(client).await?,
    })
}

async fn mutate_room_notes<F>(
    client: &Client,
    session_generation: u64,
    mutate: F,
) -> Result<NativeRoomNotesSnapshot, &'static str>
where
    F: FnOnce(SynaraRoomNotesContent) -> SynaraRoomNotesContent,
{
    let next = mutate(load_room_notes_content(client).await?);
    store_room_notes_content(client, &next).await?;
    Ok(NativeRoomNotesSnapshot {
        session_generation,
        content: next,
    })
}

fn validate_note_item(item: &SynaraRoomNoteItem) -> Result<(), &'static str> {
    if item.id.is_empty() || item.room_id.is_empty() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    if !item.created_at.is_finite() || !item.updated_at.is_finite() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    match item.kind {
        SynaraRoomNoteItemKind::Note | SynaraRoomNoteItemKind::Todo => {
            if item.body.as_ref().is_none_or(|b| b.is_empty()) {
                return Err("v-timeline-room-notes-invalid-item");
            }
        }
        SynaraRoomNoteItemKind::Message => {
            if item.event_id.as_ref().is_none_or(|e| e.is_empty()) {
                return Err("v-timeline-room-notes-invalid-item");
            }
        }
    }
    Ok(())
}

pub async fn upsert_room_note_item(
    client: &Client,
    session_generation: u64,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    validate_note_item(&item)?;
    let mut item = item;
    if let Some(body) = item.body.take() {
        let capped = limit_text(
            &body,
            if item.kind == SynaraRoomNoteItemKind::Message {
                MAX_MESSAGE_BODY_LENGTH
            } else {
                MAX_NOTE_BODY_LENGTH
            },
        );
        item.body = if capped.is_empty() {
            None
        } else {
            Some(capped)
        };
    }
    validate_note_item(&item)?;
    mutate_room_notes(client, session_generation, |content| {
        put_room_note_item(content, item)
    })
    .await
}

pub async fn delete_room_note_item_live(
    client: &Client,
    session_generation: u64,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    if room_id.is_empty() || item_id.is_empty() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    mutate_room_notes(client, session_generation, |content| {
        remove_room_note_item(content, &room_id, &item_id)
    })
    .await
}

pub async fn complete_room_todo_item_live(
    client: &Client,
    session_generation: u64,
    room_id: String,
    item_id: String,
    completed: bool,
    now: f64,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    if room_id.is_empty() || item_id.is_empty() || !now.is_finite() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    mutate_room_notes(client, session_generation, |content| {
        complete_room_todo_item(content, &room_id, &item_id, completed, now)
    })
    .await
}

pub async fn move_room_todo_item_live(
    client: &Client,
    session_generation: u64,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
    now: f64,
) -> Result<NativeRoomNotesSnapshot, &'static str> {
    if room_id.is_empty() || item_id.is_empty() || !now.is_finite() {
        return Err("v-timeline-room-notes-invalid-item");
    }
    mutate_room_notes(client, session_generation, |content| {
        move_room_todo_item(content, &room_id, &item_id, direction, now)
    })
    .await
}

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
