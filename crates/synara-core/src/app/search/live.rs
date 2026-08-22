//! Live Client-Server `/search` via typed ruma `search_events`.
//!
//! Does not enable matrix-sdk `search-index`. Failed diagnostics never echo
//! term, room id, event id, or tokens.

use matrix_sdk::ruma::{
    api::client::{
        filter::RoomEventFilter,
        search::search_events::v3::{Categories, Criteria, EventContext, OrderBy, Request},
    },
    uint, OwnedRoomId, OwnedUserId, UInt,
};
use matrix_sdk::Client;
use serde::Deserialize;

use super::{MatrixMessageSearchGroup, MatrixMessageSearchItem, MatrixMessageSearchResult};

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

#[derive(Deserialize)]
struct HitEvent {
    event_id: Option<String>,
    sender: Option<String>,
    origin_server_ts: Option<u64>,
    room_id: Option<String>,
    content: Option<HitContent>,
}

#[derive(Deserialize)]
struct HitContent {
    #[serde(default)]
    body: Option<String>,
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
    Ok(Some(token.to_owned()))
}

pub fn parse_message_search_order(order: Option<&str>) -> Result<OrderBy, &'static str> {
    match order.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("recent") => Ok(OrderBy::Recent),
        Some("rank") => Ok(OrderBy::Rank),
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
) -> Result<MatrixMessageSearchResult, &'static str> {
    let _ = client.user_id().ok_or("v-search.no-session")?;
    let Some(term) = parse_message_search_term(term)? else {
        return Ok(empty_message_search_result());
    };
    let next_batch = parse_message_search_next_token(next_token)?;
    let order_by = parse_message_search_order(order)?;
    let rooms = parse_message_search_rooms(rooms)?;
    let senders = parse_message_search_senders(senders)?;

    let mut filter = RoomEventFilter::default();
    filter.limit = Some(UInt::from(MESSAGE_SEARCH_LIMIT));
    filter.rooms = rooms;
    filter.senders = senders;

    let mut event_context = EventContext::new();
    event_context.before_limit = uint!(0);
    event_context.after_limit = uint!(0);
    event_context.include_profile = false;

    let mut criteria = Criteria::new(term);
    criteria.filter = filter;
    criteria.order_by = Some(order_by);
    criteria.event_context = event_context;
    criteria.include_state = Some(false);

    let mut categories = Categories::new();
    categories.room_events = Some(criteria);
    let mut request = Request::new(categories);
    request.next_batch = next_batch;

    let response = client
        .send(request)
        .await
        .map_err(|_| "v-search.sdk-failed")?;
    Ok(map_search_response(response.search_categories.room_events))
}

fn map_search_response(
    room_events: matrix_sdk::ruma::api::client::search::search_events::v3::ResultRoomEvents,
) -> MatrixMessageSearchResult {
    let highlights = room_events
        .highlights
        .into_iter()
        .filter_map(|highlight| cap_highlight(&highlight))
        .take(MAX_MESSAGE_SEARCH_HIGHLIGHTS)
        .collect();
    let next_token = room_events
        .next_batch
        .and_then(|token| parse_message_search_next_token(Some(&token)).ok().flatten());

    let mut items = Vec::new();
    for hit in room_events.results {
        if items.len() >= MAX_MESSAGE_SEARCH_ITEMS {
            break;
        }
        let Some(item) = map_search_hit(hit.rank.unwrap_or(0.0), hit.result.as_ref()) else {
            continue;
        };
        items.push(item);
    }

    MatrixMessageSearchResult {
        next_token,
        highlights,
        groups: group_items(items),
    }
}

fn map_search_hit(
    rank: f64,
    raw: Option<&matrix_sdk::ruma::serde::Raw<matrix_sdk::ruma::events::AnyTimelineEvent>>,
) -> Option<MatrixMessageSearchItem> {
    let raw = raw?;
    let parsed: HitEvent = serde_json::from_str(raw.json().get()).ok()?;
    let event_id = parsed.event_id.filter(|value| value.starts_with('$'))?;
    let room_id = parsed.room_id.filter(|value| value.starts_with('!'))?;
    let sender = parsed.sender.filter(|value| value.starts_with('@'))?;
    let body = cap_body(
        parsed
            .content
            .and_then(|content| content.body)
            .unwrap_or_default(),
    );
    Some(MatrixMessageSearchItem {
        rank,
        event_id,
        sender,
        origin_server_ts: parsed.origin_server_ts.unwrap_or(0),
        body,
        room_id,
    })
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

fn contains_secret_marker(value: &str) -> bool {
    value.contains("access_token")
        || value.contains("refresh_token")
        || value.contains("syt_")
        || value.contains("syr_")
}
