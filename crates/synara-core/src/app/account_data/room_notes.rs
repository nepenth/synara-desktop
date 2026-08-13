//! Credential-free `in.synara.room_notes` account-data codec.
//!
//! Live Client RMW is in `room_notes_live`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const ROOM_NOTES_EVENT_TYPE: &str = "in.synara.room_notes";
pub const ROOM_NOTES_ACCOUNT_DATA_VERSION: u32 = 1;
pub const MAX_NOTE_BODY_LENGTH: usize = 4000;
pub const MAX_MESSAGE_BODY_LENGTH: usize = 1000;
const MAX_ITEMS_PER_ROOM: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynaraRoomNotesContent {
    pub version: u32,
    pub rooms: BTreeMap<String, SynaraRoomNotesRoom>,
}

impl Default for SynaraRoomNotesContent {
    fn default() -> Self {
        Self {
            version: ROOM_NOTES_ACCOUNT_DATA_VERSION,
            rooms: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SynaraRoomNotesRoom {
    pub items: BTreeMap<String, SynaraRoomNoteItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynaraRoomNoteItem {
    pub id: String,
    pub kind: SynaraRoomNoteItemKind,
    pub room_id: String,
    pub created_at: f64,
    pub updated_at: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_ts: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SynaraRoomNoteItemKind {
    Note,
    Todo,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoomNoteMoveDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomNotesSnapshot {
    pub session_generation: u64,
    pub content: SynaraRoomNotesContent,
}

pub fn limit_text(value: &str, max_length: usize) -> String {
    value.trim().chars().take(max_length).collect()
}

fn finite_ts(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite())
}

pub fn normalize_room_note_item(item: &serde_json::Value) -> Option<SynaraRoomNoteItem> {
    let id = item.get("id")?.as_str()?.to_owned();
    let kind = match item.get("kind")?.as_str()? {
        "note" => SynaraRoomNoteItemKind::Note,
        "todo" => SynaraRoomNoteItemKind::Todo,
        "message" => SynaraRoomNoteItemKind::Message,
        _ => return None,
    };
    let room_id = item.get("roomId")?.as_str()?.to_owned();
    let created_at = item.get("createdAt")?.as_f64()?;
    let updated_at = item.get("updatedAt")?.as_f64()?;
    if id.is_empty() || room_id.is_empty() || !created_at.is_finite() || !updated_at.is_finite() {
        return None;
    }

    let body = item.get("body").and_then(|v| v.as_str()).map(|body| {
        limit_text(
            body,
            if kind == SynaraRoomNoteItemKind::Message {
                MAX_MESSAGE_BODY_LENGTH
            } else {
                MAX_NOTE_BODY_LENGTH
            },
        )
    });
    let body = body.filter(|b| !b.is_empty());

    let next = SynaraRoomNoteItem {
        id,
        kind,
        room_id,
        created_at,
        updated_at,
        body,
        completed_at: finite_ts(item.get("completedAt").and_then(|v| v.as_f64())),
        order: finite_ts(item.get("order").and_then(|v| v.as_f64())),
        event_id: item
            .get("eventId")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .filter(|v| !v.is_empty()),
        event_ts: finite_ts(item.get("eventTs").and_then(|v| v.as_f64())),
        sender: item
            .get("sender")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .filter(|v| !v.is_empty()),
    };

    match next.kind {
        SynaraRoomNoteItemKind::Note | SynaraRoomNoteItemKind::Todo => {
            next.body.as_ref()?;
        }
        SynaraRoomNoteItemKind::Message => {
            next.event_id.as_ref()?;
        }
    }
    Some(next)
}

pub fn normalize_room_notes_content(value: Option<&serde_json::Value>) -> SynaraRoomNotesContent {
    let mut rooms = BTreeMap::new();
    let Some(raw_rooms) = value
        .and_then(|v| v.get("rooms"))
        .and_then(|v| v.as_object())
    else {
        return SynaraRoomNotesContent::default();
    };

    for (room_id, room_notes) in raw_rooms {
        let Some(raw_items) = room_notes.get("items").and_then(|v| v.as_object()) else {
            continue;
        };
        let mut items: Vec<SynaraRoomNoteItem> = raw_items
            .values()
            .filter_map(normalize_room_note_item)
            .filter(|item| item.room_id == *room_id)
            .collect();
        items.sort_by(|a, b| {
            b.updated_at
                .partial_cmp(&a.updated_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(MAX_ITEMS_PER_ROOM);
        if items.is_empty() {
            continue;
        }
        let map = items
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect();
        rooms.insert(room_id.clone(), SynaraRoomNotesRoom { items: map });
    }

    SynaraRoomNotesContent {
        version: ROOM_NOTES_ACCOUNT_DATA_VERSION,
        rooms,
    }
}

pub fn put_room_note_item(
    content: SynaraRoomNotesContent,
    item: SynaraRoomNoteItem,
) -> SynaraRoomNotesContent {
    let mut next = content;
    next.version = ROOM_NOTES_ACCOUNT_DATA_VERSION;
    let room = next.rooms.entry(item.room_id.clone()).or_default();
    room.items.insert(item.id.clone(), item);
    // Cap after upsert.
    if room.items.len() > MAX_ITEMS_PER_ROOM {
        let mut ordered: Vec<_> = room.items.values().cloned().collect();
        ordered.sort_by(|a, b| {
            b.updated_at
                .partial_cmp(&a.updated_at)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ordered.truncate(MAX_ITEMS_PER_ROOM);
        room.items = ordered
            .into_iter()
            .map(|item| (item.id.clone(), item))
            .collect();
    }
    next
}

pub fn remove_room_note_item(
    content: SynaraRoomNotesContent,
    room_id: &str,
    item_id: &str,
) -> SynaraRoomNotesContent {
    let mut next = content;
    if let Some(room) = next.rooms.get_mut(room_id) {
        room.items.remove(item_id);
        if room.items.is_empty() {
            next.rooms.remove(room_id);
        }
    }
    next
}

pub fn complete_room_todo_item(
    content: SynaraRoomNotesContent,
    room_id: &str,
    item_id: &str,
    completed: bool,
    now: f64,
) -> SynaraRoomNotesContent {
    let Some(item) = content
        .rooms
        .get(room_id)
        .and_then(|room| room.items.get(item_id))
        .cloned()
    else {
        return content;
    };
    if item.kind != SynaraRoomNoteItemKind::Todo {
        return content;
    }
    put_room_note_item(
        content,
        SynaraRoomNoteItem {
            updated_at: now,
            completed_at: if completed { Some(now) } else { None },
            ..item
        },
    )
}

pub fn move_room_todo_item(
    content: SynaraRoomNotesContent,
    room_id: &str,
    item_id: &str,
    direction: RoomNoteMoveDirection,
    now: f64,
) -> SynaraRoomNotesContent {
    let Some(room) = content.rooms.get(room_id) else {
        return content;
    };
    let mut todo_items: Vec<_> = room
        .items
        .values()
        .filter(|item| item.kind == SynaraRoomNoteItemKind::Todo)
        .cloned()
        .collect();
    todo_items.sort_by(
        |a, b| match (a.completed_at.is_some(), b.completed_at.is_some()) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => {
                let a_order = a.order.unwrap_or(a.updated_at);
                let b_order = b.order.unwrap_or(b.updated_at);
                b_order
                    .partial_cmp(&a_order)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        },
    );
    let current_index = match todo_items.iter().position(|item| item.id == item_id) {
        Some(index) => index,
        None => return content,
    };
    let target_index = match direction {
        RoomNoteMoveDirection::Up if current_index > 0 => current_index - 1,
        RoomNoteMoveDirection::Down => current_index + 1,
        _ => return content,
    };
    let Some(current_item) = todo_items.get(current_index).cloned() else {
        return content;
    };
    let Some(target_item) = todo_items.get(target_index).cloned() else {
        return content;
    };
    if current_item.completed_at.is_some() != target_item.completed_at.is_some() {
        return content;
    }
    let current_order = current_item.order.unwrap_or(current_item.updated_at);
    let target_order = target_item.order.unwrap_or(target_item.updated_at);
    let swapped = put_room_note_item(
        content,
        SynaraRoomNoteItem {
            order: Some(target_order),
            updated_at: now,
            ..current_item
        },
    );
    put_room_note_item(
        swapped,
        SynaraRoomNoteItem {
            order: Some(current_order),
            updated_at: now,
            ..target_item
        },
    )
}
