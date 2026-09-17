//! Empty-term in-room Media/Files listing.
//!
//! Walks one room's event cache and `/messages` (not Client-Server `/search`).
//! Independent of NativeTimelinePresenter pagination. Fail closed on bad
//! input; a `/messages` failure keeps cache hits already collected.

use std::collections::HashSet;

use matrix_sdk::deserialized_responses::TimelineEvent;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::uint;
use matrix_sdk::Client;

use super::live::{group_items, map_hit_event, HitEvent};
use super::{
    parse_message_search_rooms, MatrixMessageSearchItem, MatrixMessageSearchResult,
    MAX_MESSAGE_SEARCH_ITEMS,
};

pub const MAX_ATTACHMENT_LISTING_PAGES: usize = 8;
pub const MAX_ATTACHMENT_LISTING_EVENTS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentListingKind {
    Media,
    Files,
}

pub fn parse_attachment_listing_kind(value: &str) -> Result<AttachmentListingKind, &'static str> {
    match value.trim() {
        "media" => Ok(AttachmentListingKind::Media),
        "files" => Ok(AttachmentListingKind::Files),
        _ => Err("v-search.invalid-listing"),
    }
}

pub fn msg_type_matches_listing(msg_type: Option<&str>, kind: AttachmentListingKind) -> bool {
    match kind {
        AttachmentListingKind::Media => matches!(msg_type, Some("m.image") | Some("m.video")),
        AttachmentListingKind::Files => matches!(msg_type, Some("m.file")),
    }
}

pub async fn list_room_attachments(
    client: &Client,
    room_id: &str,
    kind: &str,
    from_ts: u64,
    to_ts: u64,
) -> Result<MatrixMessageSearchResult, &'static str> {
    let _ = client.user_id().ok_or("v-search.no-session")?;
    if from_ts > to_ts {
        return Err("v-search.invalid-range");
    }
    let kind = parse_attachment_listing_kind(kind)?;
    let rooms = parse_message_search_rooms(Some(&[room_id.to_owned()]))?;
    let room_id = rooms
        .as_ref()
        .and_then(|rooms| rooms.first())
        .ok_or("v-search.invalid-room")?;
    let room = client
        .get_room(room_id.as_ref())
        .ok_or("v-search.invalid-room")?;
    let _ = client.event_cache().subscribe();

    let mut seen = HashSet::new();
    let mut matches = Vec::new();
    let mut events_seen = 0usize;
    let mut reached_from = false;
    let filter = ListingFilter {
        kind,
        from_ts,
        to_ts,
    };

    for event in collect_cached_events(&room, MAX_ATTACHMENT_LISTING_EVENTS).await {
        events_seen = events_seen.saturating_add(1);
        consider_event(
            event,
            room_id.as_str(),
            filter,
            &mut seen,
            &mut matches,
            None,
        );
        if events_seen >= MAX_ATTACHMENT_LISTING_EVENTS || matches.len() >= MAX_MESSAGE_SEARCH_ITEMS
        {
            break;
        }
    }

    let mut from_token: Option<String> = None;
    for _ in 0..MAX_ATTACHMENT_LISTING_PAGES {
        if reached_from
            || events_seen >= MAX_ATTACHMENT_LISTING_EVENTS
            || matches.len() >= MAX_MESSAGE_SEARCH_ITEMS
        {
            break;
        }
        let mut options = MessagesOptions::backward();
        options.limit = uint!(25);
        if let Some(token) = from_token.as_deref() {
            options = options.from(Some(token));
        }
        let page = match room.messages(options).await {
            Ok(page) => page,
            Err(_) => break,
        };
        if page.chunk.is_empty() {
            break;
        }
        for event in page.chunk {
            events_seen = events_seen.saturating_add(1);
            if consider_event(
                event,
                room_id.as_str(),
                filter,
                &mut seen,
                &mut matches,
                Some(&mut reached_from),
            ) {
                break;
            }
            if events_seen >= MAX_ATTACHMENT_LISTING_EVENTS
                || matches.len() >= MAX_MESSAGE_SEARCH_ITEMS
            {
                break;
            }
        }
        match page.end {
            Some(token) if !token.is_empty() => from_token = Some(token),
            _ => break,
        }
    }

    matches.sort_by_key(|item| std::cmp::Reverse(item.origin_server_ts));
    matches.truncate(MAX_MESSAGE_SEARCH_ITEMS);

    Ok(MatrixMessageSearchResult {
        next_token: None,
        highlights: Vec::new(),
        groups: group_items(matches),
    })
}

async fn collect_cached_events(room: &matrix_sdk::Room, cap: usize) -> Vec<TimelineEvent> {
    let Ok((cache, _handles)) = room.event_cache().await else {
        return Vec::new();
    };
    let mut events = Vec::new();
    let _ = cache
        .rfind_map_event_in_memory_by(|event| {
            events.push(event.clone());
            (events.len() >= cap).then_some(())
        })
        .await;
    events
}

#[derive(Clone, Copy)]
struct ListingFilter {
    kind: AttachmentListingKind,
    from_ts: u64,
    to_ts: u64,
}

fn consider_event(
    event: TimelineEvent,
    room_id: &str,
    filter: ListingFilter,
    seen: &mut HashSet<String>,
    matches: &mut Vec<MatrixMessageSearchItem>,
    reached_from: Option<&mut bool>,
) -> bool {
    let Some(item) = map_timeline_event(room_id, &event) else {
        return false;
    };
    if !seen.insert(item.event_id.clone()) {
        return false;
    }
    if item.origin_server_ts > 0 && item.origin_server_ts < filter.from_ts {
        if let Some(reached_from) = reached_from {
            *reached_from = true;
            return true;
        }
        return false;
    }
    if item.origin_server_ts > filter.to_ts {
        return false;
    }
    if msg_type_matches_listing(item.msg_type.as_deref(), filter.kind) {
        matches.push(item);
    }
    false
}

fn map_timeline_event(room_id: &str, event: &TimelineEvent) -> Option<MatrixMessageSearchItem> {
    let parsed: HitEvent = serde_json::from_str(event.raw().json().get()).ok()?;
    map_hit_event(1.0, Some(room_id), parsed)
}
