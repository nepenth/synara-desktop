//! Desktop local-index query path (matrix-sdk `experimental-search`).
//!
//! Never issues Client-Server `search_events`. Hits stay ids + a capped
//! snippet. Global results keep SDK relevance interleave (consecutive groups
//! only when adjacent hits share a room).

use futures_util::StreamExt;
use matrix_sdk::deserialized_responses::TimelineEvent;
use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId, OwnedUserId, RoomId};
use matrix_sdk::Client;

use super::{
    encode_next_offset, group_items, highlights_from_term, map_hit_event, HitEvent,
    MatrixMessageSearchItem, MatrixMessageSearchResult, MESSAGE_SEARCH_LIMIT,
};

pub(super) async fn search_index(
    client: &Client,
    term: &str,
    offset: usize,
    rooms: Option<&[OwnedRoomId]>,
    senders: Option<&[OwnedUserId]>,
) -> Result<MatrixMessageSearchResult, &'static str> {
    let needed = usize::from(MESSAGE_SEARCH_LIMIT).saturating_add(1);
    let scored = match rooms {
        None => global_hits(client, term, offset, needed).await?,
        Some([room_id]) => room_hits(client, room_id, term, offset, needed).await?,
        Some(room_ids) => merge_room_hits(client, room_ids, term, offset, needed).await?,
    };

    let has_more = scored.len() > usize::from(MESSAGE_SEARCH_LIMIT);
    let page = if has_more {
        &scored[..usize::from(MESSAGE_SEARCH_LIMIT)]
    } else {
        scored.as_slice()
    };

    let mut items = Vec::new();
    for (room_id, score, event_id) in page {
        let Some(item) = load_hit(client, room_id, *score, event_id).await else {
            continue;
        };
        if let Some(allowed) = senders {
            if !allowed.iter().any(|sender| sender.as_str() == item.sender) {
                continue;
            }
        }
        items.push(item);
        if items.len() >= usize::from(MESSAGE_SEARCH_LIMIT) {
            break;
        }
    }

    Ok(MatrixMessageSearchResult {
        next_token: encode_next_offset(
            offset,
            page.len().min(usize::from(MESSAGE_SEARCH_LIMIT)),
            has_more,
        ),
        highlights: highlights_from_term(term),
        groups: group_items(items),
    })
}

async fn global_hits(
    client: &Client,
    term: &str,
    offset: usize,
    needed: usize,
) -> Result<Vec<(OwnedRoomId, f32, OwnedEventId)>, &'static str> {
    let mut stream = Box::pin(client.search_messages(term.to_owned()).build());
    let mut skipped = 0usize;
    let mut hits = Vec::new();
    while let Some(page) = stream.next().await {
        let page = page.map_err(|_| "v-search.sdk-failed")?;
        for (room_id, score, event_id) in page {
            if skipped < offset {
                skipped = skipped.saturating_add(1);
                continue;
            }
            hits.push((room_id, score, event_id));
            if hits.len() >= needed {
                return Ok(hits);
            }
        }
    }
    Ok(hits)
}

async fn room_hits(
    client: &Client,
    room_id: &RoomId,
    term: &str,
    offset: usize,
    needed: usize,
) -> Result<Vec<(OwnedRoomId, f32, OwnedEventId)>, &'static str> {
    let Some(room) = client.get_room(room_id) else {
        return Ok(Vec::new());
    };
    let page = room
        .search(term, needed, Some(offset))
        .await
        .map_err(|_| "v-search.sdk-failed")?;
    Ok(page
        .into_iter()
        .map(|(score, event_id)| (room_id.to_owned(), score, event_id))
        .collect())
}

async fn merge_room_hits(
    client: &Client,
    rooms: &[OwnedRoomId],
    term: &str,
    offset: usize,
    needed: usize,
) -> Result<Vec<(OwnedRoomId, f32, OwnedEventId)>, &'static str> {
    let fetch = offset.saturating_add(needed);
    let mut merged = Vec::new();
    for room_id in rooms {
        let Some(room) = client.get_room(room_id) else {
            continue;
        };
        let page = room
            .search(term, fetch, Some(0))
            .await
            .map_err(|_| "v-search.sdk-failed")?;
        for (score, event_id) in page {
            merged.push((room_id.clone(), score, event_id));
        }
    }
    merged.sort_by(|a, b| b.1.total_cmp(&a.1));
    if offset >= merged.len() {
        return Ok(Vec::new());
    }
    let end = offset.saturating_add(needed).min(merged.len());
    Ok(merged[offset..end].to_vec())
}

async fn load_hit(
    client: &Client,
    room_id: &RoomId,
    score: f32,
    event_id: &matrix_sdk::ruma::EventId,
) -> Option<MatrixMessageSearchItem> {
    let room = client.get_room(room_id)?;
    let event = room.load_or_fetch_event(event_id, None).await.ok()?;
    map_timeline_hit(score, room_id, &event)
}

fn map_timeline_hit(
    score: f32,
    room_id: &RoomId,
    event: &TimelineEvent,
) -> Option<MatrixMessageSearchItem> {
    let parsed: HitEvent = serde_json::from_str(event.raw().json().get()).ok()?;
    let mut item = map_hit_event(f64::from(score), Some(room_id.as_str()), parsed)?;
    if let Some(event_id) = event.event_id() {
        if item.event_id.is_empty() {
            item.event_id = event_id.to_string();
        }
    }
    Some(item)
}
