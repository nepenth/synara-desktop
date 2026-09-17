//! Live message search.
//!
//! Desktop (`search-index`) queries the 0.19 local index and never sends
//! Client-Server `search_events`. iOS SharedCore keeps typed `/search`.
//! Failed diagnostics never echo term, room id, event id, or tokens.

use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};
use matrix_sdk::Client;

use super::{MatrixMessageSearchGroup, MatrixMessageSearchItem, MatrixMessageSearchResult};

#[cfg(not(feature = "search-index"))]
#[path = "cs.rs"]
mod cs;
#[cfg(feature = "search-index")]
#[path = "index.rs"]
mod index;

pub const MAX_MESSAGE_SEARCH_TERM_CHARS: usize = 256;
pub const MAX_MESSAGE_SEARCH_BODY_CHARS: usize = 512;
pub const MAX_MESSAGE_SEARCH_HIGHLIGHTS: usize = 32;
pub const MAX_MESSAGE_SEARCH_HIGHLIGHT_CHARS: usize = 64;
pub const MAX_MESSAGE_SEARCH_GROUPS: usize = 20;
pub const MAX_MESSAGE_SEARCH_ITEMS: usize = 20;
pub const MAX_MESSAGE_SEARCH_ROOMS: usize = 64;
pub const MAX_MESSAGE_SEARCH_SENDERS: usize = 64;
pub const MAX_MESSAGE_SEARCH_NEXT_TOKEN_CHARS: usize = 1024;
pub const MESSAGE_SEARCH_LIMIT: u16 = 20;

#[derive(serde::Deserialize)]
struct HitEvent {
    event_id: Option<String>,
    sender: Option<String>,
    origin_server_ts: Option<u64>,
    room_id: Option<String>,
    content: Option<serde_json::Value>,
}

pub fn parse_message_search_term(term: &str) -> Result<Option<String>, &'static str> {
    let trimmed = term.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_MESSAGE_SEARCH_TERM_CHARS {
        return Err("v-search.term-too-long");
    }
    if contains_secret_marker(trimmed) {
        return Err("v-search.invalid-term");
    }
    Ok(Some(trimmed.to_owned()))
}

pub fn parse_message_search_next_token(
    next_token: Option<&str>,
) -> Result<Option<String>, &'static str> {
    let Some(token) = next_token.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if token.chars().count() > MAX_MESSAGE_SEARCH_NEXT_TOKEN_CHARS {
        return Err("v-search.invalid-token");
    }
    if contains_secret_marker(token) {
        return Err("v-search.invalid-token");
    }
    #[cfg(feature = "search-index")]
    {
        if !token.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("v-search.invalid-token");
        }
        let _offset: usize = token.parse().map_err(|_| "v-search.invalid-token")?;
    }
    Ok(Some(token.to_owned()))
}

#[cfg(feature = "search-index")]
pub fn parse_message_search_offset(next_token: Option<&str>) -> Result<usize, &'static str> {
    match parse_message_search_next_token(next_token)? {
        None => Ok(0),
        Some(token) => token.parse().map_err(|_| "v-search.invalid-token"),
    }
}

/// Local index is relevance-only. Omitted, `rank`, and `recent` are accepted as
/// rank so leftover UI that still sends Recent does not fail closed.
pub fn parse_message_search_order(order: Option<&str>) -> Result<(), &'static str> {
    match order.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("rank") | Some("recent") => Ok(()),
        _ => Err("v-search.invalid-order"),
    }
}

pub fn parse_message_search_rooms(
    rooms: Option<&[String]>,
) -> Result<Option<Vec<OwnedRoomId>>, &'static str> {
    let Some(rooms) = rooms.filter(|values| !values.is_empty()) else {
        return Ok(None);
    };
    if rooms.len() > MAX_MESSAGE_SEARCH_ROOMS {
        return Err("v-search.invalid-room");
    }
    let mut parsed = Vec::with_capacity(rooms.len());
    for room in rooms {
        let trimmed = room.trim();
        if trimmed.is_empty() || !trimmed.starts_with('!') {
            return Err("v-search.invalid-room");
        }
        parsed.push(OwnedRoomId::try_from(trimmed).map_err(|_| "v-search.invalid-room")?);
    }
    Ok(Some(parsed))
}

pub fn parse_message_search_senders(
    senders: Option<&[String]>,
) -> Result<Option<Vec<OwnedUserId>>, &'static str> {
    let Some(senders) = senders.filter(|values| !values.is_empty()) else {
        return Ok(None);
    };
    if senders.len() > MAX_MESSAGE_SEARCH_SENDERS {
        return Err("v-search.invalid-sender");
    }
    let mut parsed = Vec::with_capacity(senders.len());
    for sender in senders {
        let trimmed = sender.trim();
        if trimmed.is_empty() || !trimmed.starts_with('@') {
            return Err("v-search.invalid-sender");
        }
        parsed.push(OwnedUserId::try_from(trimmed).map_err(|_| "v-search.invalid-sender")?);
    }
    Ok(Some(parsed))
}

pub fn empty_message_search_result() -> MatrixMessageSearchResult {
    MatrixMessageSearchResult {
        next_token: None,
        highlights: Vec::new(),
        groups: Vec::new(),
    }
}

pub async fn search_messages(
    client: &Client,
    term: &str,
    next_token: Option<&str>,
    rooms: Option<&[String]>,
    senders: Option<&[String]>,
    order: Option<&str>,
    indexed_message_search: bool,
) -> Result<MatrixMessageSearchResult, &'static str> {
    let _ = client.user_id().ok_or("v-search.no-session")?;
    let Some(term) = parse_message_search_term(term)? else {
        return Ok(empty_message_search_result());
    };
    parse_message_search_order(order)?;
    let rooms = parse_message_search_rooms(rooms)?;
    let senders = parse_message_search_senders(senders)?;

    #[cfg(feature = "search-index")]
    {
        if !indexed_message_search {
            return Err("v-search.index-disabled");
        }
        let offset = parse_message_search_offset(next_token)?;
        return index::search_index(client, &term, offset, rooms.as_deref(), senders.as_deref())
            .await;
    }

    #[cfg(not(feature = "search-index"))]
    {
        let _ = indexed_message_search;
        cs::search_homeserver(client, &term, next_token, rooms, senders, order).await
    }
}

fn map_hit_event(
    rank: f64,
    fallback_room_id: Option<&str>,
    parsed: HitEvent,
) -> Option<MatrixMessageSearchItem> {
    let event_id = parsed.event_id.filter(|value| value.starts_with('$'))?;
    let room_id = parsed
        .room_id
        .filter(|value| value.starts_with('!'))
        .or_else(|| {
            fallback_room_id
                .map(str::to_owned)
                .filter(|value| value.starts_with('!'))
        })?;
    let sender = parsed.sender.filter(|value| value.starts_with('@'))?;
    let body = cap_body(snippet_from_content(parsed.content.as_ref()));
    Some(MatrixMessageSearchItem {
        rank,
        event_id,
        sender,
        origin_server_ts: parsed.origin_server_ts.unwrap_or(0),
        body,
        room_id,
    })
}

fn snippet_from_content(content: Option<&serde_json::Value>) -> String {
    let Some(content) = content else {
        return String::new();
    };
    if let Some(body) = content.get("body").and_then(|value| value.as_str()) {
        return body.to_owned();
    }
    if let Some(filename) = content.get("filename").and_then(|value| value.as_str()) {
        return filename.to_owned();
    }
    if let Some(body) = content
        .get("new_content")
        .and_then(|value| value.get("body"))
        .and_then(|value| value.as_str())
    {
        return body.to_owned();
    }
    for key in ["m.poll.start", "org.matrix.msc3381.poll.start"] {
        if let Some(text) = content
            .get(key)
            .and_then(|value| value.get("question"))
            .and_then(|value| value.get("text").or_else(|| value.get("body")))
            .and_then(|value| value.as_str())
        {
            return text.to_owned();
        }
    }
    String::new()
}

fn group_items(items: Vec<MatrixMessageSearchItem>) -> Vec<MatrixMessageSearchGroup> {
    let mut groups: Vec<MatrixMessageSearchGroup> = Vec::new();
    for item in items {
        if let Some(last) = groups.last_mut() {
            if last.room_id == item.room_id {
                last.items.push(item);
                continue;
            }
        }
        if groups.len() >= MAX_MESSAGE_SEARCH_GROUPS {
            break;
        }
        groups.push(MatrixMessageSearchGroup {
            room_id: item.room_id.clone(),
            items: vec![item],
        });
    }
    groups
}

fn cap_body(body: String) -> String {
    let trimmed = body.trim();
    if trimmed.chars().count() <= MAX_MESSAGE_SEARCH_BODY_CHARS {
        return trimmed.to_owned();
    }
    trimmed
        .chars()
        .take(MAX_MESSAGE_SEARCH_BODY_CHARS)
        .collect()
}

fn cap_highlight(highlight: &str) -> Option<String> {
    let trimmed = highlight.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() <= MAX_MESSAGE_SEARCH_HIGHLIGHT_CHARS {
        return Some(trimmed.to_owned());
    }
    Some(
        trimmed
            .chars()
            .take(MAX_MESSAGE_SEARCH_HIGHLIGHT_CHARS)
            .collect(),
    )
}

#[cfg(feature = "search-index")]
fn highlights_from_term(term: &str) -> Vec<String> {
    term.split_whitespace()
        .filter_map(cap_highlight)
        .take(MAX_MESSAGE_SEARCH_HIGHLIGHTS)
        .collect()
}

fn contains_secret_marker(value: &str) -> bool {
    value.contains("access_token")
        || value.contains("refresh_token")
        || value.contains("syt_")
        || value.contains("syr_")
}

#[cfg(feature = "search-index")]
fn encode_next_offset(offset: usize, page_len: usize, has_more: bool) -> Option<String> {
    if has_more {
        Some(offset.saturating_add(page_len).to_string())
    } else {
        None
    }
}
