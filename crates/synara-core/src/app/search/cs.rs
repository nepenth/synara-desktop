//! iOS / NSE Client-Server `/search` via typed ruma `search_events`.
//!
//! Desktop with `search-index` does not compile this module.

use matrix_sdk::ruma::{
    api::client::{
        filter::RoomEventFilter,
        search::search_events::v3::{Categories, Criteria, EventContext, OrderBy, Request},
    },
    uint, OwnedRoomId, OwnedUserId, UInt,
};
use matrix_sdk::Client;

use super::{
    cap_highlight, group_items, map_hit_event, parse_message_search_next_token, HitEvent,
    MatrixMessageSearchResult, MAX_MESSAGE_SEARCH_HIGHLIGHTS, MAX_MESSAGE_SEARCH_ITEMS,
    MESSAGE_SEARCH_LIMIT,
};

pub(super) async fn search_homeserver(
    client: &Client,
    term: &str,
    next_token: Option<&str>,
    rooms: Option<Vec<OwnedRoomId>>,
    senders: Option<Vec<OwnedUserId>>,
    order: Option<&str>,
) -> Result<MatrixMessageSearchResult, &'static str> {
    let next_batch = parse_message_search_next_token(next_token)?;
    let order_by = match order.map(str::trim).filter(|value| !value.is_empty()) {
        Some("rank") => OrderBy::Rank,
        _ => OrderBy::Recent,
    };

    let mut filter = RoomEventFilter::default();
    filter.limit = Some(UInt::from(MESSAGE_SEARCH_LIMIT));
    filter.rooms = rooms;
    filter.senders = senders;

    let mut event_context = EventContext::new();
    event_context.before_limit = uint!(0);
    event_context.after_limit = uint!(0);
    event_context.include_profile = false;

    let mut criteria = Criteria::new(term.to_owned());
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
) -> Option<super::MatrixMessageSearchItem> {
    let raw = raw?;
    let parsed: HitEvent = serde_json::from_str(raw.json().get()).ok()?;
    map_hit_event(rank, None, parsed)
}
